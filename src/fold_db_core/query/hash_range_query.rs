//! HashRange Query Processor
//!
//! Handles query processing for HashRange schemas using field resolution.
//!
//! For personal-scope schemas (no `org_hash`), the processor also scans
//! `from:{sender_hash}:` namespaces for each active `ShareSubscription` so
//! that data received from other users (sync-replayed under the `from:`
//! prefix by `SyncEngine::rewrite_key_if_needed`) surfaces alongside the
//! caller's own data. Each received-from row has its `writer_pubkey`
//! stamped with the sender hash when no molecule-level writer is already
//! present, providing source attribution to downstream consumers.

use crate::db_operations::DbOperations;
use crate::schema::types::field::Field;
use crate::schema::types::field::FieldValue;
use crate::schema::types::field::FieldVariant;
use crate::schema::types::field::HashRangeFilter;
use crate::schema::types::key_value::KeyValue;
use crate::schema::{Schema, SchemaError};
use crate::storage::SledPool;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;

/// Processor for HashRange schema queries using field resolution
pub struct HashRangeQueryProcessor {
    db_ops: Arc<DbOperations>,
    /// Optional Sled pool used to enumerate active `ShareSubscription`s.
    /// When `None`, received-from namespaces are not scanned.
    sled_pool: Option<Arc<SledPool>>,
}

impl HashRangeQueryProcessor {
    /// Create a new HashRange query processor.
    pub fn new(db_ops: Arc<DbOperations>, sled_pool: Option<Arc<SledPool>>) -> Self {
        Self { db_ops, sled_pool }
    }

    pub async fn query_with_filter(
        &self,
        schema: &mut Schema,
        fields: &[String],
        filter: Option<HashRangeFilter>,
        as_of: Option<DateTime<Utc>>,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        let current_user = crate::logging::core::get_current_user_id();
        log::info!(
            "🔍 HashRangeQueryProcessor: schema={}, filter={:?}, user_context={:?}",
            schema.name,
            filter,
            current_user
        );

        // Gather the list of `from:{sender_hash}` namespaces to scan in
        // addition to the schema's own namespace. Only applies to personal
        // schemas; org-scoped schemas have a single defined namespace.
        let received_from_namespaces: Vec<String> = if schema.org_hash.is_none() {
            self.collect_received_from_namespaces()
        } else {
            Vec::new()
        };

        let mut result: HashMap<String, HashMap<KeyValue, FieldValue>> = HashMap::new();
        let return_all = fields.is_empty();
        for (field_name, field) in schema.runtime_fields.iter_mut() {
            if !return_all && !fields.contains(field_name) {
                continue;
            }
            log::debug!("🔍 Resolving field: {}", field_name);

            // Resolve against the schema's native namespace (personal or org).
            let mut field_value = field
                .resolve_value(&self.db_ops, filter.clone(), as_of)
                .await?;
            log::debug!(
                "✅ Field '{}' resolved {} values from own namespace",
                field_name,
                field_value.len()
            );

            // Resolve against each `from:{sender}` namespace, if any.
            for namespace in &received_from_namespaces {
                match self
                    .resolve_field_from_namespace(field, namespace, filter.clone(), as_of)
                    .await
                {
                    Ok(mut shared) => {
                        log::debug!(
                            "✅ Field '{}' resolved {} values from namespace '{}'",
                            field_name,
                            shared.len(),
                            namespace
                        );
                        // Stamp writer_pubkey with the sender hash so downstream
                        // consumers can distinguish received data. Respect any
                        // existing molecule-level writer_pubkey (which carries
                        // the original signer's key — source of truth per PR
                        // #544) and only fall back to the namespace sender hash
                        // when unset.
                        let sender_hash = Self::sender_hash_from_namespace(namespace);
                        for fv in shared.values_mut() {
                            if fv.writer_pubkey.is_none() {
                                fv.writer_pubkey = Some(sender_hash.clone());
                            }
                        }
                        // Merge into the result map. If a key collides across
                        // namespaces (e.g. both Bob and Alice have a note at
                        // the same (title, date)), the personal-namespace row
                        // wins since it was inserted first.
                        for (k, v) in shared {
                            field_value.entry(k).or_insert(v);
                        }
                    }
                    Err(e) => {
                        // Do not swallow: surface this as a hard error so
                        // data is never silently dropped. Callers can
                        // re-architect around it if needed.
                        return Err(SchemaError::InvalidData(format!(
                            "Failed to resolve field '{}' from namespace '{}': {}",
                            field_name, namespace, e
                        )));
                    }
                }
            }

            result.insert(field_name.clone(), field_value);
        }
        log::debug!(
            "✅ HashRangeQueryProcessor::query_with_filter: returning {} fields",
            result.len()
        );
        Ok(result)
    }

    /// Look up active `ShareSubscription`s and return a deduped list of
    /// `from:{sender_hash}` namespace prefixes to scan. Returns an empty
    /// vector if `sled_pool` is absent or the store has no subscriptions.
    ///
    /// Subscriptions with an unparseable `share_prefix` are logged and
    /// skipped (never silently). Inactive subscriptions are excluded.
    fn collect_received_from_namespaces(&self) -> Vec<String> {
        let Some(pool) = self.sled_pool.as_ref() else {
            return Vec::new();
        };

        let subs = match crate::sharing::store::list_share_subscriptions(pool) {
            Ok(s) => s,
            Err(e) => {
                log::error!(
                    "HashRangeQueryProcessor: failed to list share subscriptions: {}",
                    e
                );
                return Vec::new();
            }
        };

        let mut namespaces: Vec<String> = Vec::new();
        for sub in subs {
            if !sub.active {
                continue;
            }
            match Self::parse_sender_hash(&sub.share_prefix) {
                Some(sender_hash) => {
                    let ns = format!("from:{}", sender_hash);
                    if !namespaces.contains(&ns) {
                        namespaces.push(ns);
                    }
                }
                None => {
                    log::error!(
                        "HashRangeQueryProcessor: subscription has unparseable \
                         share_prefix '{}' (expected 'share:{{sender}}:{{recipient}}'); \
                         skipping.",
                        sub.share_prefix
                    );
                }
            }
        }
        namespaces
    }

    /// Parse the sender hash from a share prefix of the form
    /// `share:{sender_hash}:{recipient_hash}`. Returns `None` if the prefix
    /// doesn't match this structure.
    fn parse_sender_hash(share_prefix: &str) -> Option<String> {
        let mut parts = share_prefix.split(':');
        let kind = parts.next()?;
        if kind != "share" {
            return None;
        }
        let sender = parts.next()?;
        if sender.is_empty() {
            return None;
        }
        Some(sender.to_string())
    }

    /// Extract the sender hash from a `from:{sender_hash}` namespace prefix.
    fn sender_hash_from_namespace(namespace: &str) -> String {
        namespace.strip_prefix("from:").unwrap_or(namespace).to_string()
    }

    /// Resolve a field's values from a specific storage namespace by
    /// temporarily rewriting the field's `org_hash`. The field is cloned so
    /// we don't mutate the caller's schema state.
    async fn resolve_field_from_namespace(
        &self,
        field: &FieldVariant,
        namespace: &str,
        filter: Option<HashRangeFilter>,
        as_of: Option<DateTime<Utc>>,
    ) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
        let mut cloned = field.clone();
        // Clear any in-memory molecule cached from a previous resolve so the
        // next refresh reads from the target namespace. Also drop per-key
        // metadata that belonged to the previous namespace's molecule.
        match &mut cloned {
            FieldVariant::Single(f) => {
                f.base.molecule = None;
            }
            FieldVariant::Hash(f) => {
                f.base.molecule = None;
            }
            FieldVariant::Range(f) => {
                f.base.molecule = None;
            }
            FieldVariant::HashRange(f) => {
                f.base.molecule = None;
            }
        }
        cloned
            .common_mut()
            .set_org_hash(Some(namespace.to_string()));
        cloned.resolve_value(&self.db_ops, filter, as_of).await
    }
}
