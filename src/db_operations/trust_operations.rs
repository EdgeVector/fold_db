use std::collections::HashMap;

use crate::access::types::{TrustMap, DOMAIN_PERSONAL};
use crate::access::{AuditEvent, AuditLog};
use crate::schema::types::SchemaError;
use crate::storage::traits::TypedStore;

use super::DbOperations;

const TRUST_GRAPH_PREFIX: &str = "trust_graph:";
const AUDIT_LOG_KEY: &str = "audit_log";

/// Legacy key — migrated to `trust_graph:personal` on first access.
const LEGACY_TRUST_GRAPH_KEY: &str = "trust_graph";

impl DbOperations {
    /// Storage key for a domain's trust map.
    fn trust_graph_key(domain: &str) -> String {
        format!("{}{}", TRUST_GRAPH_PREFIX, domain)
    }

    /// Load the trust map for a specific domain. Returns empty map if none stored.
    /// On first call, migrates the legacy `trust_graph` key to `trust_graph:personal`.
    /// If deserialization of old format fails, returns empty map (graceful migration).
    pub async fn load_trust_map_for_domain(&self, domain: &str) -> Result<TrustMap, SchemaError> {
        let key = Self::trust_graph_key(domain);
        match self.permissions_store().get_item::<TrustMap>(&key).await {
            Ok(Some(map)) => Ok(map),
            Ok(None) => {
                // Migration: if requesting "personal" and legacy key exists, try to migrate
                if domain == DOMAIN_PERSONAL {
                    if let Ok(Some(legacy)) = self
                        .permissions_store()
                        .get_item::<TrustMap>(LEGACY_TRUST_GRAPH_KEY)
                        .await
                    {
                        // Migrate: store under new key, delete legacy
                        let _ = self.permissions_store().put_item(&key, &legacy).await;
                        let _ = self
                            .permissions_store()
                            .delete_item(LEGACY_TRUST_GRAPH_KEY)
                            .await;
                        return Ok(legacy);
                    }
                }
                Ok(HashMap::new())
            }
            Err(_) => {
                // Old TrustGraph format can't be deserialized — return empty map
                Ok(HashMap::new())
            }
        }
    }

    /// Load the trust map (default "personal" domain). Backwards compatible.
    pub async fn load_trust_map(&self) -> Result<TrustMap, SchemaError> {
        self.load_trust_map_for_domain(DOMAIN_PERSONAL).await
    }

    /// Persist the trust map for a specific domain.
    pub async fn store_trust_map_for_domain(
        &self,
        domain: &str,
        map: &TrustMap,
    ) -> Result<(), SchemaError> {
        let key = Self::trust_graph_key(domain);
        self.permissions_store()
            .put_item(&key, map)
            .await
            .map_err(|e| {
                SchemaError::InvalidData(format!(
                    "Failed to store trust map for domain '{}': {}",
                    domain, e
                ))
            })
    }

    /// Persist the trust map (default "personal" domain). Backwards compatible.
    pub async fn store_trust_map(&self, map: &TrustMap) -> Result<(), SchemaError> {
        self.store_trust_map_for_domain(DOMAIN_PERSONAL, map).await
    }

    /// Delete a trust domain's map entirely.
    pub async fn delete_trust_domain(&self, domain: &str) -> Result<(), SchemaError> {
        let key = Self::trust_graph_key(domain);
        self.permissions_store()
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
            .permissions_store()
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
        self.permissions_store()
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
            .permissions_store()
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
