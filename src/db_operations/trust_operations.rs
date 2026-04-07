use crate::access::types::DOMAIN_PERSONAL;
use crate::access::{AuditEvent, AuditLog, TrustGraph};
use crate::schema::types::SchemaError;
use crate::storage::traits::TypedStore;

use super::DbOperations;

const TRUST_GRAPH_PREFIX: &str = "trust_graph:";
const AUDIT_LOG_KEY: &str = "audit_log";

/// Legacy key — migrated to `trust_graph:personal` on first access.
const LEGACY_TRUST_GRAPH_KEY: &str = "trust_graph";

impl DbOperations {
    /// Storage key for a domain's trust graph.
    fn trust_graph_key(domain: &str) -> String {
        format!("{}{}", TRUST_GRAPH_PREFIX, domain)
    }

    /// Load the trust graph for a specific domain. Returns empty graph if none stored.
    /// On first call, migrates the legacy `trust_graph` key to `trust_graph:personal`.
    pub async fn load_trust_graph_for_domain(
        &self,
        domain: &str,
    ) -> Result<TrustGraph, SchemaError> {
        let key = Self::trust_graph_key(domain);
        match self.permissions_store().get_item::<TrustGraph>(&key).await {
            Ok(Some(graph)) => Ok(graph),
            Ok(None) => {
                // Migration: if requesting "personal" and legacy key exists, migrate it
                if domain == DOMAIN_PERSONAL {
                    if let Ok(Some(legacy)) = self
                        .permissions_store()
                        .get_item::<TrustGraph>(LEGACY_TRUST_GRAPH_KEY)
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
                Ok(TrustGraph::new())
            }
            Err(e) => Err(SchemaError::InvalidData(format!(
                "Failed to load trust graph for domain '{}': {}",
                domain, e
            ))),
        }
    }

    /// Load the trust graph (default "personal" domain). Backwards compatible.
    pub async fn load_trust_graph(&self) -> Result<TrustGraph, SchemaError> {
        self.load_trust_graph_for_domain(DOMAIN_PERSONAL).await
    }

    /// Persist the trust graph for a specific domain.
    pub async fn store_trust_graph_for_domain(
        &self,
        domain: &str,
        graph: &TrustGraph,
    ) -> Result<(), SchemaError> {
        let key = Self::trust_graph_key(domain);
        self.permissions_store()
            .put_item(&key, graph)
            .await
            .map_err(|e| {
                SchemaError::InvalidData(format!(
                    "Failed to store trust graph for domain '{}': {}",
                    domain, e
                ))
            })
    }

    /// Persist the trust graph (default "personal" domain). Backwards compatible.
    pub async fn store_trust_graph(&self, graph: &TrustGraph) -> Result<(), SchemaError> {
        self.store_trust_graph_for_domain(DOMAIN_PERSONAL, graph)
            .await
    }

    /// Delete a trust domain's graph entirely.
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

    /// List all trust domain names that have stored graphs.
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
}
