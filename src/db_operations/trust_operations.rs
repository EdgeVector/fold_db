use crate::access::{AuditEvent, AuditLog, TrustGraph};
use crate::schema::types::SchemaError;
use crate::storage::traits::TypedStore;

use super::DbOperations;

const TRUST_GRAPH_KEY: &str = "trust_graph";
const AUDIT_LOG_KEY: &str = "audit_log";

impl DbOperations {
    /// Load the trust graph from storage. Returns empty graph if none stored.
    pub async fn load_trust_graph(&self) -> Result<TrustGraph, SchemaError> {
        match self
            .permissions_store()
            .get_item::<TrustGraph>(TRUST_GRAPH_KEY)
            .await
        {
            Ok(Some(graph)) => Ok(graph),
            Ok(None) => Ok(TrustGraph::new()),
            Err(e) => Err(SchemaError::InvalidData(format!(
                "Failed to load trust graph: {}",
                e
            ))),
        }
    }

    /// Persist the trust graph to storage.
    pub async fn store_trust_graph(&self, graph: &TrustGraph) -> Result<(), SchemaError> {
        self.permissions_store()
            .put_item(TRUST_GRAPH_KEY, graph)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to store trust graph: {}", e)))
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
