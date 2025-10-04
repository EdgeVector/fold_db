use super::error_utils::ErrorUtils;
use crate::schema::SchemaError;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;

/// Enhanced database operations struct that provides unified access to all database operations.
/// This replaces the previous mixed approach of direct sled access and DbOperations.
#[derive(Clone)]
pub struct DbOperations {
    /// The underlying sled database instance
    db: sled::Db,
    /// Cached trees for performance
    pub(crate) metadata_tree: sled::Tree,
    pub(crate) permissions_tree: sled::Tree,
    pub(crate) transforms_tree: sled::Tree,
    pub(crate) orchestrator_tree: sled::Tree,
    pub(crate) schema_states_tree: sled::Tree,
    pub(crate) schemas_tree: sled::Tree,
    /// Tree for storing public keys
    pub(crate) public_keys_tree: sled::Tree,
    pub(crate) transform_queue_tree: sled::Tree,
}

impl DbOperations {
    /// Creates a new enhanced DbOperations instance with all required trees
    pub fn new(db: sled::Db) -> Result<Self, sled::Error> {
        let metadata_tree = db.open_tree("metadata")?;
        let permissions_tree = db.open_tree("node_id_schema_permissions")?;
        let transforms_tree = db.open_tree("transforms")?;
        let orchestrator_tree = db.open_tree("orchestrator_state")?;
        let schema_states_tree = db.open_tree("schema_states")?;
        let schemas_tree = db.open_tree("schemas")?;
        let public_keys_tree = db.open_tree("public_keys")?;
        let transform_queue_tree = db.open_tree("transform_queue_tree")?;

        Ok(Self {
            db,
            metadata_tree,
            permissions_tree,
            transforms_tree,
            orchestrator_tree,
            schema_states_tree,
            schemas_tree,
            public_keys_tree,
            transform_queue_tree,
        })
    }

    /// Gets a reference to the underlying database
    pub fn db(&self) -> &sled::Db {
        &self.db
    }

    /// Generic function to store a serializable item in the database
    pub fn store_item<T: Serialize>(&self, key: &str, item: &T) -> Result<(), SchemaError> {
        let bytes =
            serde_json::to_vec(item).map_err(ErrorUtils::from_serialization_error("item"))?;

        self.db
            .insert(key.as_bytes(), bytes)
            .map_err(ErrorUtils::from_sled_error("insert"))?;

        // Ensure the data is durably written to disk
        self.db
            .flush()
            .map_err(ErrorUtils::from_sled_error("flush"))?;

        Ok(())
    }

    /// Generic function to retrieve a deserializable item from the database
    pub fn get_item<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, SchemaError> {
        match self.db.get(key.as_bytes()) {
            Ok(Some(bytes)) => {
                let item = serde_json::from_slice(&bytes)
                    .map_err(ErrorUtils::from_deserialization_error("item"))?;
                Ok(Some(item))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ErrorUtils::database_error("retrieve", e)),
        }
    }

    /// Lists all items with a given prefix
    pub fn list_items_with_prefix(&self, prefix: &str) -> Result<Vec<String>, SchemaError> {
        let mut items = Vec::new();
        for result in self.db.scan_prefix(prefix.as_bytes()) {
            let (key, _) = result
                .map_err(|e| SchemaError::InvalidData(format!("Failed to scan prefix: {}", e)))?;
            let key_str = String::from_utf8_lossy(&key).to_string();
            items.push(key_str);
        }
        Ok(items)
    }

    /// Gets database statistics
    pub fn get_stats(&self) -> Result<HashMap<String, u64>, SchemaError> {
        let mut stats = HashMap::new();

        // Count items in each tree
        stats.insert("atoms".to_string(), self.count_items_with_prefix("atom:")?);
        stats.insert("refs".to_string(), self.count_items_with_prefix("ref:")?);
        stats.insert("metadata".to_string(), self.metadata_tree.len() as u64);
        stats.insert(
            "permissions".to_string(),
            self.permissions_tree.len() as u64,
        );
        stats.insert("transforms".to_string(), self.transforms_tree.len() as u64);
        stats.insert(
            "orchestrator".to_string(),
            self.orchestrator_tree.len() as u64,
        );
        stats.insert(
            "schema_states".to_string(),
            self.schema_states_tree.len() as u64,
        );
        stats.insert("schemas".to_string(), self.schemas_tree.len() as u64);
        stats.insert(
            "public_keys".to_string(),
            self.public_keys_tree.len() as u64,
        );
        stats.insert(
            "transform_queue".to_string(),
            self.transform_queue_tree.len() as u64,
        );

        Ok(stats)
    }

    /// Counts items with a given prefix
    fn count_items_with_prefix(&self, prefix: &str) -> Result<u64, SchemaError> {
        let mut count = 0;
        for result in self.db.scan_prefix(prefix.as_bytes()) {
            result
                .map_err(|e| SchemaError::InvalidData(format!("Failed to scan prefix: {}", e)))?;
            count += 1;
        }
        Ok(count)
    }

    // ========== GENERIC TREE OPERATIONS ==========

    /// Generic function to store any serializable item in a specific tree
    pub fn store_in_tree<T: Serialize>(
        &self,
        tree: &sled::Tree,
        key: &str,
        item: &T,
    ) -> Result<(), SchemaError> {
        let bytes = serde_json::to_vec(item)
            .map_err(|e| SchemaError::InvalidData(format!("Serialization failed: {}", e)))?;

        tree.insert(key.as_bytes(), bytes)
            .map_err(|e| SchemaError::InvalidData(format!("Store failed: {}", e)))?;

        tree.flush()
            .map_err(|e| SchemaError::InvalidData(format!("Flush failed: {}", e)))?;

        Ok(())
    }

    /// Generic function to retrieve any deserializable item from a specific tree
    pub fn get_from_tree<T: DeserializeOwned>(
        &self,
        tree: &sled::Tree,
        key: &str,
    ) -> Result<Option<T>, SchemaError> {
        match tree.get(key.as_bytes()) {
            Ok(Some(bytes)) => {
                let item = serde_json::from_slice(&bytes).map_err(|e| {
                    SchemaError::InvalidData(format!("Deserialization failed: {}", e))
                })?;
                Ok(Some(item))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(SchemaError::InvalidData(format!("Retrieval failed: {}", e))),
        }
    }

    /// List all keys in a tree
    pub fn list_keys_in_tree(&self, tree: &sled::Tree) -> Result<Vec<String>, SchemaError> {
        let mut keys = Vec::new();
        for result in tree.iter() {
            let (key, _) = result
                .map_err(|e| SchemaError::InvalidData(format!("Tree iteration failed: {}", e)))?;
            keys.push(String::from_utf8_lossy(&key).to_string());
        }
        Ok(keys)
    }

    /// List all key-value pairs in a tree
    pub fn list_items_in_tree<T: DeserializeOwned>(
        &self,
        tree: &sled::Tree,
    ) -> Result<Vec<(String, T)>, SchemaError> {
        let mut items = Vec::new();
        for result in tree.iter() {
            let (key, value) = result
                .map_err(|e| SchemaError::InvalidData(format!("Tree iteration failed: {}", e)))?;
            let key_str = String::from_utf8_lossy(&key).to_string();
            let item = serde_json::from_slice(&value).map_err(|e| {
                SchemaError::InvalidData(format!(
                    "Deserialization failed for key '{}': {}",
                    key_str, e
                ))
            })?;
            items.push((key_str, item));
        }
        Ok(items)
    }

    /// Delete an item from a specific tree
    pub fn delete_from_tree(&self, tree: &sled::Tree, key: &str) -> Result<bool, SchemaError> {
        let existed = tree
            .remove(key.as_bytes())
            .map_err(|e| SchemaError::InvalidData(format!("Delete failed: {}", e)))?
            .is_some();

        tree.flush()
            .map_err(|e| SchemaError::InvalidData(format!("Flush failed: {}", e)))?;

        Ok(existed)
    }

    /// Check if a key exists in a specific tree
    pub fn exists_in_tree(&self, tree: &sled::Tree, key: &str) -> Result<bool, SchemaError> {
        tree.contains_key(key.as_bytes())
            .map_err(|e| SchemaError::InvalidData(format!("Existence check failed: {}", e)))
    }

    /// Flush the database to ensure all data is persisted to disk
    pub fn flush(&self) -> Result<(), SchemaError> {
        log::info!("🔄 Flushing database to disk...");
        match self.db.flush() {
            Ok(_) => {
                log::info!("✅ Database flush completed successfully");
                Ok(())
            }
            Err(e) => {
                log::error!("❌ Database flush failed: {}", e);
                Err(SchemaError::InvalidData(format!("Database flush failed: {}", e)))
            }
        }
    }
}
