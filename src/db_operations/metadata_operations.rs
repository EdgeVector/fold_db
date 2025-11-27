// Legacy v1 metadata operations - DEPRECATED
// Use metadata_operations_v2.rs for new code
#[allow(dead_code)]
use super::core_refactored::DbOperationsV2 as DbOperations;
use crate::schema::SchemaError;
use uuid::Uuid;

impl DbOperations {
    /// Retrieves or generates and persists the node identifier
    pub fn get_node_id(&self) -> Result<String, SchemaError> {
        if let Some(bytes) = self
            .metadata_tree
            .get("node_id")
            .map_err(|e| SchemaError::InvalidData(format!("Failed to get node_id: {}", e)))?
        {
            let id = String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| String::new());
            if !id.is_empty() {
                return Ok(id);
            }
        }
        let new_id = Uuid::new_v4().to_string();
        self.set_node_id(&new_id)?;
        Ok(new_id)
    }

    /// Sets the node identifier
    pub fn set_node_id(&self, node_id: &str) -> Result<(), SchemaError> {
        self.metadata_tree
            .insert("node_id", node_id.as_bytes())
            .map_err(|e| SchemaError::InvalidData(format!("Failed to set node_id: {}", e)))?;
        self.metadata_tree
            .flush()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to flush metadata: {}", e)))?;
        Ok(())
    }

    /// Retrieves the list of permitted schemas for the given node using generic operations
    pub fn get_schema_permissions(&self, node_id: &str) -> Result<Vec<String>, SchemaError> {
        self.get_from_tree(&self.permissions_tree, node_id)
            .map(|opt| opt.unwrap_or_default())
    }

    /// Sets the permitted schemas for the given node using generic operations
    pub fn set_schema_permissions(
        &self,
        node_id: &str,
        schemas: &[String],
    ) -> Result<(), SchemaError> {
        let schemas_vec: Vec<String> = schemas.to_vec();
        self.store_in_tree(&self.permissions_tree, node_id, &schemas_vec)
    }

    /// Lists all nodes with permissions
    pub fn list_nodes_with_permissions(&self) -> Result<Vec<String>, SchemaError> {
        self.list_keys_in_tree(&self.permissions_tree)
    }

    /// Deletes permissions for a node
    pub fn delete_schema_permissions(&self, node_id: &str) -> Result<bool, SchemaError> {
        match self.permissions_tree.remove(node_id.as_bytes()) {
            Ok(old_value) => Ok(old_value.is_some()),
            Err(e) => Err(SchemaError::InvalidData(format!(
                "Failed to delete permissions: {}",
                e
            ))),
        }
    }

    /// Checks if a node has permissions set
    pub fn node_has_permissions(&self, node_id: &str) -> Result<bool, SchemaError> {
        self.exists_in_tree(&self.permissions_tree, node_id)
    }
}
