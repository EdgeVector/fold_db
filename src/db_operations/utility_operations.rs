use super::core_refactored::DbOperationsV2 as DbOperations;
use crate::schema::SchemaError;
use serde::{de::DeserializeOwned, Serialize};

impl DbOperations {
    /// Batch store multiple items
    pub fn batch_store<T: Serialize>(&self, items: &[(String, T)]) -> Result<(), SchemaError> {
        for (key, item) in items {
            self.store_item(key, item)?;
        }
        Ok(())
    }

    /// Batch get multiple items
    pub fn batch_get<T: DeserializeOwned>(
        &self,
        keys: &[String],
    ) -> Result<Vec<Option<T>>, SchemaError> {
        let mut results = Vec::new();
        for key in keys {
            results.push(self.get_item(key)?);
        }
        Ok(results)
    }
}
