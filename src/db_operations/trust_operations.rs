//! Thin delegator methods on `DbOperations` for trust-map and audit-log
//! persistence. Real implementations live on
//! [`super::permissions_store::PermissionsStore`].

use crate::access::types::AccessMap;
use crate::access::{AuditEvent, AuditLog};
use crate::schema::SchemaError;

use super::DbOperations;

impl DbOperations {
    /// Load the trust map for a specific domain.
    pub async fn load_trust_map_for_domain(&self, domain: &str) -> Result<AccessMap, SchemaError> {
        self.permissions().load_trust_map_for_domain(domain).await
    }

    /// Load the trust map (default "personal" domain).
    pub async fn load_trust_map(&self) -> Result<AccessMap, SchemaError> {
        self.permissions().load_trust_map().await
    }

    /// Persist the trust map for a specific domain.
    pub async fn store_trust_map_for_domain(
        &self,
        domain: &str,
        map: &AccessMap,
    ) -> Result<(), SchemaError> {
        self.permissions()
            .store_trust_map_for_domain(domain, map)
            .await
    }

    /// Persist the trust map (default "personal" domain).
    pub async fn store_trust_map(&self, map: &AccessMap) -> Result<(), SchemaError> {
        self.permissions().store_trust_map(map).await
    }

    /// Delete a trust domain's map entirely.
    pub async fn delete_trust_domain(&self, domain: &str) -> Result<(), SchemaError> {
        self.permissions().delete_trust_domain(domain).await
    }

    /// Load the audit log from storage. Returns empty log if none stored.
    pub async fn load_audit_log(&self) -> Result<AuditLog, SchemaError> {
        self.permissions().load_audit_log().await
    }

    /// Persist the audit log to storage.
    pub async fn store_audit_log(&self, log: &AuditLog) -> Result<(), SchemaError> {
        self.permissions().store_audit_log(log).await
    }

    /// Append a single audit event and persist.
    pub async fn append_audit_event(&self, event: AuditEvent) -> Result<(), SchemaError> {
        self.permissions().append_audit_event(event).await
    }

    /// List all trust domain names that have stored maps.
    pub async fn list_trust_domains(&self) -> Result<Vec<String>, SchemaError> {
        self.permissions().list_trust_domains().await
    }
}
