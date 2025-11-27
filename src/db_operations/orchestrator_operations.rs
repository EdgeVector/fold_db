// Legacy v1 orchestrator operations - DEPRECATED
#[allow(dead_code)]
use super::core_refactored::DbOperationsV2 as DbOperations;
use crate::schema::SchemaError;
use serde::{de::DeserializeOwned, Serialize};

impl DbOperations {
    /// Stores orchestrator state using generic tree operations
    pub fn store_orchestrator_state<T: Serialize>(
        &self,
        key: &str,
        state: &T,
    ) -> Result<(), SchemaError> {
        self.store_in_tree(&self.orchestrator_tree, key, state)
    }

    /// Gets orchestrator state using generic tree operations
    pub fn get_orchestrator_state<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, SchemaError> {
        self.get_from_tree(&self.orchestrator_tree, key)
    }

    /// Lists all orchestrator state keys
    pub fn list_orchestrator_keys(&self) -> Result<Vec<String>, SchemaError> {
        self.list_keys_in_tree(&self.orchestrator_tree)
    }

    /// Deletes orchestrator state
    pub fn delete_orchestrator_state(&self, key: &str) -> Result<bool, SchemaError> {
        self.delete_from_tree(&self.orchestrator_tree, key)
    }
}
