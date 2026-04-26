//! Permissions domain store.
//!
//! Owns the `node_id_schema_permissions` and `public_keys` namespaces.
//! External callers reach these via `DbOperations::permissions()`.
//!
//! Responsibilities:
//! - System-wide public key storage
//! - Trust maps (per domain) and audit log

use std::collections::HashMap;
use std::sync::Arc;

use crate::access::types::{AccessMap, DOMAIN_PERSONAL};
use crate::access::{AuditEvent, AuditLog};
use crate::constants::SINGLE_PUBLIC_KEY_ID;
use crate::schema::SchemaError;
use crate::security::PublicKeyInfo;
use crate::storage::traits::{KvStore, TypedStore};
use crate::storage::TypedKvStore;

const TRUST_GRAPH_PREFIX: &str = "trust_graph:";
const AUDIT_LOG_KEY: &str = "audit_log";

/// Legacy key — migrated to `trust_graph:personal` on first access.
const LEGACY_TRUST_GRAPH_KEY: &str = "trust_graph";

/// Domain store for permissions / trust / public-key persistence.
#[derive(Clone)]
pub struct PermissionsStore {
    permissions_store: Arc<TypedKvStore<dyn KvStore>>,
    public_keys_store: Arc<TypedKvStore<dyn KvStore>>,
}

impl PermissionsStore {
    pub(crate) fn new(
        permissions_store: Arc<TypedKvStore<dyn KvStore>>,
        public_keys_store: Arc<TypedKvStore<dyn KvStore>>,
    ) -> Self {
        Self {
            permissions_store,
            public_keys_store,
        }
    }

    /// Crate-internal access to the raw permissions namespace (used by org purge).
    pub(crate) fn raw_permissions(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.permissions_store
    }

    /// Crate-internal access to the raw public-keys namespace (used by org purge).
    pub(crate) fn raw_public_keys(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.public_keys_store
    }

    // ===== Public key operations =====

    /// Gets the system-wide public key
    pub async fn get_system_public_key(&self) -> Result<Option<PublicKeyInfo>, SchemaError> {
        Ok(self
            .public_keys_store
            .get_item(SINGLE_PUBLIC_KEY_ID)
            .await?)
    }

    /// Stores the system-wide public key
    pub async fn store_system_public_key(
        &self,
        key_info: &PublicKeyInfo,
    ) -> Result<(), SchemaError> {
        self.public_keys_store
            .put_item(SINGLE_PUBLIC_KEY_ID, key_info)
            .await?;
        self.public_keys_store.inner().flush().await?;
        Ok(())
    }

    /// Deletes the system-wide public key
    pub async fn delete_system_public_key(&self) -> Result<bool, SchemaError> {
        Ok(self
            .public_keys_store
            .delete_item(SINGLE_PUBLIC_KEY_ID)
            .await?)
    }

    // ===== Trust-map operations =====

    fn trust_graph_key(domain: &str) -> String {
        format!("{}{}", TRUST_GRAPH_PREFIX, domain)
    }

    /// Load the trust map for a specific domain. Returns empty map if none stored.
    /// On first call for `personal`, migrates the legacy `trust_graph` key to
    /// `trust_graph:personal`. If deserialization of old format fails, returns
    /// an empty map (graceful migration).
    pub async fn load_trust_map_for_domain(&self, domain: &str) -> Result<AccessMap, SchemaError> {
        let key = Self::trust_graph_key(domain);
        match self.permissions_store.get_item::<AccessMap>(&key).await {
            Ok(Some(map)) => Ok(map),
            Ok(None) => {
                if domain == DOMAIN_PERSONAL {
                    if let Ok(Some(legacy)) = self
                        .permissions_store
                        .get_item::<AccessMap>(LEGACY_TRUST_GRAPH_KEY)
                        .await
                    {
                        let _ = self.permissions_store.put_item(&key, &legacy).await;
                        let _ = self
                            .permissions_store
                            .delete_item(LEGACY_TRUST_GRAPH_KEY)
                            .await;
                        return Ok(legacy);
                    }
                }
                Ok(HashMap::new())
            }
            Err(_) => Ok(HashMap::new()),
        }
    }

    /// Load the trust map (default "personal" domain).
    pub async fn load_trust_map(&self) -> Result<AccessMap, SchemaError> {
        self.load_trust_map_for_domain(DOMAIN_PERSONAL).await
    }

    /// Persist the trust map for a specific domain.
    pub async fn store_trust_map_for_domain(
        &self,
        domain: &str,
        map: &AccessMap,
    ) -> Result<(), SchemaError> {
        let key = Self::trust_graph_key(domain);
        self.permissions_store
            .put_item(&key, map)
            .await
            .map_err(|e| {
                SchemaError::InvalidData(format!(
                    "Failed to store trust map for domain '{}': {}",
                    domain, e
                ))
            })
    }

    /// Persist the trust map (default "personal" domain).
    pub async fn store_trust_map(&self, map: &AccessMap) -> Result<(), SchemaError> {
        self.store_trust_map_for_domain(DOMAIN_PERSONAL, map).await
    }

    /// Delete a trust domain's map entirely.
    pub async fn delete_trust_domain(&self, domain: &str) -> Result<(), SchemaError> {
        let key = Self::trust_graph_key(domain);
        self.permissions_store
            .delete_item(&key)
            .await
            .map_err(|e| {
                SchemaError::InvalidData(format!(
                    "Failed to delete trust domain '{}': {}",
                    domain, e
                ))
            })?;
        Ok(())
    }

    /// Load the audit log from storage. Returns empty log if none stored.
    pub async fn load_audit_log(&self) -> Result<AuditLog, SchemaError> {
        match self
            .permissions_store
            .get_item::<AuditLog>(AUDIT_LOG_KEY)
            .await
        {
            Ok(Some(log)) => Ok(log),
            Ok(None) => Ok(AuditLog::new()),
            Err(e) => Err(SchemaError::InvalidData(format!(
                "Failed to load audit log: {}",
                e
            ))),
        }
    }

    /// Persist the audit log to storage.
    pub async fn store_audit_log(&self, log: &AuditLog) -> Result<(), SchemaError> {
        self.permissions_store
            .put_item(AUDIT_LOG_KEY, log)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to store audit log: {}", e)))
    }

    /// Append a single audit event and persist. Convenience method.
    pub async fn append_audit_event(&self, event: AuditEvent) -> Result<(), SchemaError> {
        let mut log = self.load_audit_log().await?;
        log.record(event);
        self.store_audit_log(&log).await
    }

    /// List all trust domain names that have stored maps.
    pub async fn list_trust_domains(&self) -> Result<Vec<String>, SchemaError> {
        let entries = self
            .permissions_store
            .inner()
            .scan_prefix(TRUST_GRAPH_PREFIX.as_bytes())
            .await
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to scan trust domains: {}", e))
            })?;

        let domains: Vec<String> = entries
            .iter()
            .filter_map(|(key_bytes, _)| {
                let key = String::from_utf8_lossy(key_bytes);
                key.strip_prefix(TRUST_GRAPH_PREFIX).map(|d| d.to_string())
            })
            .collect();

        Ok(domains)
    }
}
