use crate::access::{CapabilityConstraint, CapabilityKind, PaymentGate};
use crate::schema::types::SchemaError;
use crate::storage::traits::TypedStore;

use super::DbOperations;

const CAPABILITIES_PREFIX: &str = "cap:";
const PAYMENT_GATE_PREFIX: &str = "payment_gate:";

impl DbOperations {
    /// Store a capability token.
    /// Key format: "cap:{schema}:{field}:{public_key}:{kind}"
    pub async fn store_capability(
        &self,
        schema_name: &str,
        field_name: &str,
        constraint: &CapabilityConstraint,
    ) -> Result<(), SchemaError> {
        let key = format!(
            "{}{}:{}:{}:{:?}",
            CAPABILITIES_PREFIX, schema_name, field_name, constraint.public_key, constraint.kind
        );
        self.permissions_store()
            .put_item(&key, constraint)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to store capability: {}", e)))
    }

    /// Load all capabilities for a schema field.
    pub async fn load_capabilities(
        &self,
        schema_name: &str,
        field_name: &str,
    ) -> Result<Vec<CapabilityConstraint>, SchemaError> {
        let prefix = format!("{}{}:{}", CAPABILITIES_PREFIX, schema_name, field_name);
        let items: Vec<(String, CapabilityConstraint)> = self
            .permissions_store()
            .scan_items_with_prefix(&prefix)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to load capabilities: {}", e)))?;
        Ok(items.into_iter().map(|(_, c)| c).collect())
    }

    /// Delete a capability token.
    pub async fn delete_capability(
        &self,
        schema_name: &str,
        field_name: &str,
        public_key: &str,
        kind: CapabilityKind,
    ) -> Result<bool, SchemaError> {
        let key = format!(
            "{}{}:{}:{}:{:?}",
            CAPABILITIES_PREFIX, schema_name, field_name, public_key, kind
        );
        self.permissions_store()
            .delete_item(&key)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to delete capability: {}", e)))
    }

    /// Decrement the quota of a capability token and persist.
    pub async fn decrement_capability(
        &self,
        schema_name: &str,
        field_name: &str,
        public_key: &str,
        kind: CapabilityKind,
    ) -> Result<bool, SchemaError> {
        let key = format!(
            "{}{}:{}:{}:{:?}",
            CAPABILITIES_PREFIX, schema_name, field_name, public_key, kind
        );
        let cap: Option<CapabilityConstraint> =
            self.permissions_store().get_item(&key).await.map_err(|e| {
                SchemaError::InvalidData(format!("Failed to load capability: {}", e))
            })?;

        match cap {
            Some(mut c) => {
                if c.decrement() {
                    self.permissions_store()
                        .put_item(&key, &c)
                        .await
                        .map_err(|e| {
                            SchemaError::InvalidData(format!("Failed to update capability: {}", e))
                        })?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            None => Ok(false),
        }
    }

    /// Store a payment gate for a schema.
    pub async fn store_payment_gate(
        &self,
        schema_name: &str,
        gate: &PaymentGate,
    ) -> Result<(), SchemaError> {
        let key = format!("{}{}", PAYMENT_GATE_PREFIX, schema_name);
        self.permissions_store()
            .put_item(&key, gate)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to store payment gate: {}", e)))
    }

    /// Load the payment gate for a schema.
    pub async fn load_payment_gate(
        &self,
        schema_name: &str,
    ) -> Result<Option<PaymentGate>, SchemaError> {
        let key = format!("{}{}", PAYMENT_GATE_PREFIX, schema_name);
        self.permissions_store()
            .get_item(&key)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to load payment gate: {}", e)))
    }

    /// Delete the payment gate for a schema.
    pub async fn delete_payment_gate(&self, schema_name: &str) -> Result<bool, SchemaError> {
        let key = format!("{}{}", PAYMENT_GATE_PREFIX, schema_name);
        self.permissions_store()
            .delete_item(&key)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to delete payment gate: {}", e)))
    }
}
