use crate::log_feature;
use crate::logging::features::LogFeature;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::{FoldDbError, FoldDbResult, NetworkErrorKind};
use crate::fold_db_core::FoldDB;
use crate::network::{NetworkConfig, NetworkCore, PeerId};
use crate::security::{EncryptionManager, SecurityManager};

use super::config::NodeInfo;
use super::node::NetworkStatus;
use super::DataFoldNode;
use std::future::Future;

impl DataFoldNode {
    /// Initialize the network layer
    pub async fn init_network(&mut self, network_config: NetworkConfig) -> FoldDbResult<()> {
        let network_core = NetworkCore::new(network_config)
            .await
            .map_err(|e| FoldDbError::Network(e.into()))?;

        let mut network_core = network_core;
        let db_clone = self.db.clone();

        network_core
            .schema_service_mut()
            .set_schema_check_callback(move |schema_names| {
                let db = match db_clone.lock() {
                    Ok(db) => db,
                    Err(_) => return Vec::new(),
                };

                schema_names
                    .iter()
                    .filter(|name| matches!(db.schema_manager.get_schema(name), Ok(Some(_))))
                    .cloned()
                    .collect()
            });

        let local_peer_id = network_core.local_peer_id();
        network_core.register_node_id(&self.node_id, local_peer_id);
        log_feature!(
            LogFeature::Network,
            info,
            "Registered node ID {} with peer ID {}",
            self.node_id,
            local_peer_id
        );

        self.network = Some(Arc::new(tokio::sync::Mutex::new(network_core)));

        Ok(())
    }

    /// Helper to run an operation with the network core
    async fn with_network<'a, F, Fut, T>(&'a self, f: F) -> FoldDbResult<T>
    where
        F: FnOnce(tokio::sync::MutexGuard<'a, NetworkCore>) -> Fut,
        Fut: Future<Output = FoldDbResult<T>> + 'a,
    {
        let network = self.network.as_ref().ok_or_else(|| {
            FoldDbError::Network(NetworkErrorKind::Protocol(
                "Network not initialized".to_string(),
            ))
        })?;

        let guard = network.lock().await;
        f(guard).await
    }

    /// Start the network service using the node configuration address
    pub async fn start_network(&self) -> FoldDbResult<()> {
        let address = self.config.network_listen_address.clone();
        self.with_network(|mut network| async move {
            network
                .run(&address)
                .await
                .map_err(|e| FoldDbError::Network(e.into()))?;
            Ok(())
        })
        .await
    }

    /// Start the network service with a specific listen address
    pub async fn start_network_with_address(&self, listen_address: &str) -> FoldDbResult<()> {
        let address = listen_address.to_string();
        self.with_network(|mut network| async move {
            network
                .run(&address)
                .await
                .map_err(|e| FoldDbError::Network(e.into()))?;
            Ok(())
        })
        .await
    }

    /// Stop the network service
    pub async fn stop_network(&self) -> FoldDbResult<()> {
        self.with_network(|mut network_guard| async move {
            log_feature!(LogFeature::Network, info, "Stopping network service");
            network_guard.stop();
            Ok(())
        })
        .await
    }

    /// Get a mutable reference to the network core
    pub async fn get_network_mut(&self) -> FoldDbResult<tokio::sync::MutexGuard<'_, NetworkCore>> {
        self.with_network(|guard| async move { Ok(guard) }).await
    }

    /// Discover nodes on the local network using mDNS
    pub async fn discover_nodes(&self) -> FoldDbResult<Vec<PeerId>> {
        self.with_network(|network_guard| async move {
            log_feature!(LogFeature::Network, info, "Triggering mDNS discovery...");
            let known_peers: Vec<PeerId> = network_guard.known_peers().iter().cloned().collect();
            Ok(known_peers)
        })
        .await
    }

    /// Get the list of known nodes
    pub async fn get_known_nodes(&self) -> FoldDbResult<HashMap<String, NodeInfo>> {
        self.with_network(|network_guard| async move {
            let mut result = HashMap::new();
            for peer_id in network_guard.known_peers() {
                let peer_id_str = peer_id.to_string();

                if let Some(info) = self.trusted_nodes.get(&peer_id_str) {
                    result.insert(peer_id_str, info.clone());
                } else {
                    result.insert(
                        peer_id_str.clone(),
                        NodeInfo {
                            id: peer_id_str,
                            trust_distance: self.config.default_trust_distance,
                        },
                    );
                }
            }
            Ok(result)
        })
        .await
    }

    /// Check which schemas are available on a remote peer
    pub async fn check_remote_schemas(
        &self,
        peer_id_str: &str,
        schema_names: Vec<String>,
    ) -> FoldDbResult<Vec<String>> {
        let peer_id = peer_id_str.parse::<PeerId>().map_err(|e| {
            FoldDbError::Network(NetworkErrorKind::Connection(format!(
                "Invalid peer ID: {}",
                e
            )))
        })?;

        self.with_network(|mut network| async move {
            let result = network
                .check_schemas(peer_id, schema_names)
                .await
                .map_err(|e| FoldDbError::Network(e.into()))?;
            Ok(result)
        })
        .await
    }

    /// Forward a request to another node
    pub async fn forward_request(&self, peer_id: PeerId, request: Value) -> FoldDbResult<Value> {
        self.with_network(|mut network| async move {
            let node_id = network
                .get_node_id_for_peer(&peer_id)
                .unwrap_or_else(|| peer_id.to_string());

            log_feature!(
                LogFeature::Network,
                info,
                "Forwarding request to node {} (peer {})",
                node_id,
                peer_id
            );

            let response = network
                .forward_request(peer_id, request)
                .await
                .map_err(|e| FoldDbError::Network(e.into()))?;

            log_feature!(
                LogFeature::Network,
                info,
                "Received response from node {} (peer {})",
                node_id,
                peer_id
            );

            Ok(response)
        })
        .await
    }

    /// Simple method to connect to another node
    pub async fn connect_to_node(&mut self, node_id: &str) -> FoldDbResult<()> {
        self.add_trusted_node(node_id)
    }

    /// Retrieve basic network status information
    pub async fn get_network_status(&self) -> FoldDbResult<NetworkStatus> {
        let initialized = self.network.is_some();
        let connected_nodes_count = if let Some(network) = &self.network {
            let guard = network.lock().await;
            guard.known_peers().len()
        } else {
            0
        };
        Ok(NetworkStatus {
            node_id: self.node_id.clone(),
            initialized,
            connected_nodes_count,
        })
    }

    /// Restart the node by reinitializing all components
    pub async fn restart(&mut self) -> FoldDbResult<()> {
        log_feature!(LogFeature::Network, info, "Restarting DataFoldNode...");

        if self.network.is_some() {
            log_feature!(
                LogFeature::Network,
                info,
                "Stopping network service for restart"
            );
            if let Err(e) = self.stop_network().await {
                log_feature!(
                    LogFeature::Network,
                    warn,
                    "Failed to stop network during restart: {}",
                    e
                );
            }
        }

        let storage_path = self
            .config
            .storage_path
            .to_str()
            .ok_or_else(|| FoldDbError::Config("Invalid storage path".to_string()))?
            .to_string();

        log_feature!(LogFeature::Network, info, "Closing existing database");

        // Properly close the database to release file locks
        if let Ok(db_guard) = self.db.lock() {
            if let Err(e) = db_guard.close() {
                log_feature!(
                    LogFeature::Network,
                    warn,
                    "Failed to close database properly: {}",
                    e
                );
            }
        }

        // Replace with a temporary database and drop the old one
        let old_db = std::mem::replace(
            &mut self.db,
            Arc::new(Mutex::new(FoldDB::new(&format!("{}_temp", storage_path))?)),
        );

        drop(old_db);

        // Wait longer for file system to release locks
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        // Force remove the database directory to ensure clean slate
        if let Err(e) = std::fs::remove_dir_all(&storage_path) {
            log_feature!(
                LogFeature::Network,
                warn,
                "Failed to remove database directory {}: {}",
                storage_path,
                e
            );
        }

        // Create directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(
            std::path::Path::new(&storage_path)
                .parent()
                .unwrap_or(std::path::Path::new(".")),
        ) {
            log_feature!(
                LogFeature::Network,
                warn,
                "Failed to create parent directory: {}",
                e
            );
        }

        log_feature!(LogFeature::Network, info, "Reinitializing database");
        let new_db = Arc::new(Mutex::new(FoldDB::new(&storage_path)?));
        self.db = new_db;

        self.network = None;
        self.trusted_nodes.clear();

        let mut security_config = self.config.security_config.clone();
        if security_config.encrypt_at_rest && security_config.master_key.is_none() {
            security_config.master_key = Some(EncryptionManager::generate_master_key());
        }
        self.security_manager = Arc::new(
            SecurityManager::new(security_config)
                .map_err(|e| FoldDbError::SecurityError(e.to_string()))?,
        );

        log_feature!(
            LogFeature::Network,
            info,
            "DataFoldNode restart completed successfully"
        );
        Ok(())
    }

    /// Perform a soft restart that preserves network connections
    pub async fn soft_restart(&mut self) -> FoldDbResult<()> {
        log_feature!(
            LogFeature::Network,
            info,
            "Performing soft restart of DataFoldNode..."
        );

        let storage_path = self
            .config
            .storage_path
            .to_str()
            .ok_or_else(|| FoldDbError::Config("Invalid storage path".to_string()))?
            .to_string();

        log_feature!(LogFeature::Network, info, "Closing existing database");

        // Properly close the database to release file locks
        if let Ok(db_guard) = self.db.lock() {
            if let Err(e) = db_guard.close() {
                log_feature!(
                    LogFeature::Network,
                    warn,
                    "Failed to close database properly: {}",
                    e
                );
            }
        }

        // Replace with a temporary database and drop the old one
        let old_db = std::mem::replace(
            &mut self.db,
            Arc::new(Mutex::new(FoldDB::new(&format!("{}_temp", storage_path))?)),
        );

        drop(old_db);

        // Wait longer for file system to release locks
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        // Force remove the database directory to ensure clean slate
        if let Err(e) = std::fs::remove_dir_all(&storage_path) {
            log_feature!(
                LogFeature::Network,
                warn,
                "Failed to remove database directory {}: {}",
                storage_path,
                e
            );
        }

        // Create directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(
            std::path::Path::new(&storage_path)
                .parent()
                .unwrap_or(std::path::Path::new(".")),
        ) {
            log_feature!(
                LogFeature::Network,
                warn,
                "Failed to create parent directory: {}",
                e
            );
        }

        log_feature!(LogFeature::Network, info, "Reinitializing database");
        let new_db = Arc::new(Mutex::new(FoldDB::new(&storage_path)?));
        self.db = new_db;

        log_feature!(
            LogFeature::Network,
            info,
            "DataFoldNode soft restart completed successfully"
        );
        Ok(())
    }
}
