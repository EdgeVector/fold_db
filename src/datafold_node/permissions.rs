use std::collections::HashMap;

use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;

use super::config::NodeInfo;
use super::DataFoldNode;

impl DataFoldNode {
    /// Adds a trusted node to the node's trusted nodes list.
    pub fn add_trusted_node(&mut self, node_id: &str) -> FoldDbResult<()> {
        self.trusted_nodes.insert(
            node_id.to_string(),
            NodeInfo {
                id: node_id.to_string(),
                trust_distance: self.config.default_trust_distance,
            },
        );
        Ok(())
    }

    /// Removes a trusted node from the node's trusted nodes list.
    pub fn remove_trusted_node(&mut self, node_id: &str) -> FoldDbResult<()> {
        self.trusted_nodes.remove(node_id);
        Ok(())
    }

    /// Gets the current list of trusted nodes and their trust distances.
    pub fn get_trusted_nodes(&self) -> &HashMap<String, NodeInfo> {
        &self.trusted_nodes
    }

    /// Allows operations on a schema and persists permission for this node.
    /// If the schema exists on disk but is not loaded, it will be loaded.
    pub fn allow_schema(&mut self, schema_name: &str) -> FoldDbResult<()> {
        // Check if schema is already loaded
        if !self.is_schema_loaded(schema_name)? {
            // Try to load the schema from disk
            let schema_path = {
                let db = self
                    .db
                    .lock()
                    .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
                db.schema_manager.get_schema_path(schema_name)
            };

            if schema_path.exists() {
                self.load_schema_from_file(schema_path.to_str().unwrap())
                    .map_err(|e| {
                        FoldDbError::Config(format!("Failed to load schema from disk: {}", e))
                    })?;
            } else {
                return Err(FoldDbError::Config(format!(
                    "Schema {} not found on disk",
                    schema_name
                )));
            }
        }

        // Grant permission for this schema to this node
        self.grant_schema_permission(schema_name)?;

        Ok(())
    }

    /// Grants schema permission for this node.
    pub fn grant_schema_permission(&mut self, schema_name: &str) -> FoldDbResult<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        let mut perms = db.get_schema_permissions(&self.node_id);

        if !perms.contains(&schema_name.to_string()) {
            perms.push(schema_name.to_string());
            db.set_schema_permissions(&self.node_id, &perms)
                .map_err(|e| FoldDbError::Config(format!("Failed to set permissions: {}", e)))?;
        }
        Ok(())
    }

    /// Revokes schema permission for this node.
    pub fn revoke_schema_permission(&mut self, schema_name: &str) -> FoldDbResult<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| FoldDbError::Config("Cannot lock database mutex".into()))?;
        let mut perms = db.get_schema_permissions(&self.node_id);
        perms.retain(|s| s != schema_name);
        db.set_schema_permissions(&self.node_id, &perms)
            .map_err(|e| FoldDbError::Config(format!("Failed to set permissions: {}", e)))?;
        Ok(())
    }

    /// Checks if this node has permission to access the given schema.
    /// Only approved schemas are accessible - available and blocked schemas are denied.
    pub fn check_schema_permission(&self, schema_name: &str) -> FoldDbResult<bool> {
        let db = self.db.lock().map_err(|e| {
            log_feature!(
                LogFeature::Permissions,
                error,
                "Failed to lock database mutex for permission check: {:?}",
                e
            );
            FoldDbError::Config("Cannot lock database mutex for permission check".into())
        })?;

        // Check schema state - only approved schemas are accessible
        match db.schema_manager.get_schema_state(schema_name) {
            Some(crate::schema::core::SchemaState::Approved) => Ok(true),
            Some(crate::schema::core::SchemaState::Available) => {
                log_feature!(
                    LogFeature::Permissions,
                    warn,
                    "Schema '{}' is available but not approved - access denied",
                    schema_name
                );
                Ok(false)
            }
            Some(crate::schema::core::SchemaState::Blocked) => {
                log_feature!(LogFeature::Permissions, warn, "Schema '{}' is blocked - access denied", schema_name);
                Ok(false)
            }
            None => {
                log_feature!(LogFeature::Permissions, warn, "Schema '{}' not found - access denied", schema_name);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datafold_node::config::NodeConfig;
    use tempfile::tempdir;

    #[test]
    fn trusted_node_management() {
        let dir = tempdir().unwrap();
        let config = NodeConfig {
            storage_path: dir.path().to_path_buf(),
            default_trust_distance: 1,
            network_listen_address: "/ip4/127.0.0.1/tcp/0".to_string(),
            security_config: crate::security::SecurityConfig::default(),
        };
        let mut node = DataFoldNode::new(config).unwrap();
        node.add_trusted_node("peer1").unwrap();
        assert!(node.get_trusted_nodes().contains_key("peer1"));
        node.remove_trusted_node("peer1").unwrap();
        assert!(!node.get_trusted_nodes().contains_key("peer1"));
    }
}
