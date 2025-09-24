use super::{DataFoldNode, TcpServer, OperationProcessor};
use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::operations::MutationType;
use crate::schema::Schema;
use libp2p::PeerId;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

impl TcpServer {
    /// Process a request from a client.
    pub(crate) async fn process_request(
        request: &Value,
        node: Arc<Mutex<DataFoldNode>>,
    ) -> FoldDbResult<Value> {
        log_feature!(
            LogFeature::TcpServer,
            info,
            "Processing request: {}",
            serde_json::to_string_pretty(request).unwrap_or_else(|_| request.to_string())
        );

        let operation = request
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::FoldDbError::Config("Missing operation".to_string()))?;

        if let Some(target_node_id) = request.get("target_node_id").and_then(|v| v.as_str()) {
            let local_node_id = {
                let node_guard = node.lock().await;
                node_guard.get_node_id().to_string()
            };

            if target_node_id != local_node_id {
                log_feature!(
                    LogFeature::TcpServer,
                    info,
                    "Request targeted for node {}, forwarding...",
                    target_node_id
                );
                return Self::forward_request(request, target_node_id, node.clone()).await;
            }
        }

        match operation {
            "list_schemas" => {
                let node_guard = node.lock().await;
                let schemas = node_guard.list_schemas()?;
                Ok(serde_json::to_value(schemas)?)
            }
            "list_available_schemas" => {
                let node_guard = node.lock().await;
                let names = node_guard.list_available_schemas()?;
                Ok(serde_json::to_value(names)?)
            }
            "get_schema" => {
                let schema_name = request
                    .get("params")
                    .and_then(|v| v.get("schema_name"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config(
                            "Missing schema_name parameter".to_string(),
                        )
                    })?;

                let node_guard = node.lock().await;
                let schema = node_guard.get_schema(schema_name)?;

                match schema {
                    Some(s) => Ok(serde_json::to_value(s)?),
                    None => Err(crate::error::FoldDbError::Config(format!(
                        "Schema not found: {}",
                        schema_name
                    ))),
                }
            }
            "create_schema" => {
                let schema_json = request
                    .get("params")
                    .and_then(|v| v.get("schema"))
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config("Missing schema parameter".to_string())
                    })?;

                let schema: Schema = serde_json::from_value(schema_json.clone())?;
                let mut node_guard = node.lock().await;
                node_guard.load_schema(schema)?;
                Ok(serde_json::json!({ "success": true }))
            }
            "unload_schema" => {
                let schema_name = request
                    .get("params")
                    .and_then(|v| v.get("schema_name"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config(
                            "Missing schema_name parameter".to_string(),
                        )
                    })?;

                let node_guard = node.lock().await;
                node_guard.unload_schema(schema_name)?;

                Ok(serde_json::json!({ "success": true }))
            }
            "query" => {
                let schema = request
                    .get("params")
                    .and_then(|v| v.get("schema"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config("Missing schema parameter".to_string())
                    })?;

                let fields = request
                    .get("params")
                    .and_then(|v| v.get("fields"))
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config("Missing fields parameter".to_string())
                    })?
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();

                let filter = request.get("params").and_then(|v| v.get("filter")).cloned();

                let operation = crate::schema::types::Operation::Query {
                    schema: schema.to_string(),
                    fields,
                    filter,
                };

                // Create processor with the node
                let processor = OperationProcessor::new(node.clone());
                let result = processor.execute(operation).await?;

                Ok(serde_json::json!({
                    "results": result,
                    "errors": []
                }))
            }
            "mutation" => {
                let schema = request
                    .get("params")
                    .and_then(|v| v.get("schema"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config("Missing schema parameter".to_string())
                    })?;

                // TOOD: MAKE SURE UI PASSES IN KEYS AS A HASHMAP OF STRING, STRING and FIELDS AS A HASHMAP OF STRING, VALUE.  THIS IS A MAJOR CHANGE.
                let fields_and_values: HashMap<String, Value> = request
                    .get("params")
                    .and_then(|v| v.get("fields_and_values"))
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config("Fields_and_values must be an object".to_string())
                    })?
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect();

                let keys_and_values = request
                    .get("params")
                    .and_then(|v| v.get("keys_and_values"))
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config("Keys_and_values must be an object".to_string())
                    })?
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.as_str().unwrap_or("").to_string()))
                    .collect();

                let mutation_type_str = request
                    .get("params")
                    .and_then(|v| v.get("mutation_type"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config(
                            "Missing mutation_type parameter".to_string(),
                        )
                    })?;

                let mutation_type = match mutation_type_str {
                    "create" => MutationType::Create,
                    "update" => MutationType::Update,
                    "delete" => MutationType::Delete,
                    _ => {
                        return Err(crate::error::FoldDbError::Config(format!(
                            "Invalid mutation type: {}",
                            mutation_type_str
                        )))
                    }
                };

                let operation = crate::schema::types::Operation::Mutation {
                    schema: schema.to_string(),
                    fields_and_values,
                    keys_and_values,
                    mutation_type,
                };

                // Create processor with the node
                let processor = OperationProcessor::new(node.clone());
                let _ = processor.execute(operation).await?;

                Ok(serde_json::json!({ "success": true }))
            }
            "discover_nodes" => {
                let node_guard = node.lock().await;
                let nodes = node_guard.discover_nodes().await?;

                let node_infos = nodes
                    .iter()
                    .map(|peer_id| {
                        serde_json::json!({
                            "id": peer_id.to_string(),
                            "trust_distance": 1
                        })
                    })
                    .collect::<Vec<_>>();

                Ok(serde_json::to_value(node_infos)?)
            }
            "list_schemas_by_state" => {
                let state_str = request
                    .get("params")
                    .and_then(|v| v.get("state"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config("Missing state parameter".to_string())
                    })?;

                let state = match state_str {
                    "available" => crate::schema::SchemaState::Available,
                    "approved" => crate::schema::SchemaState::Approved,
                    "blocked" => crate::schema::SchemaState::Blocked,
                    _ => {
                        return Err(crate::error::FoldDbError::Config(format!(
                            "Invalid state: {}. Use: available, approved, or blocked",
                            state_str
                        )));
                    }
                };

                let node_guard = node.lock().await;
                let schemas = node_guard.list_schemas_by_state(state)?;
                Ok(serde_json::to_value(schemas)?)
            }
            "approve_schema" => {
                let schema_name = request
                    .get("params")
                    .and_then(|v| v.get("schema_name"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config(
                            "Missing schema_name parameter".to_string(),
                        )
                    })?;

                let mut node_guard = node.lock().await;
                node_guard.approve_schema(schema_name)?;
                Ok(serde_json::json!({
                    "success": true,
                    "message": format!("Schema '{}' approved successfully", schema_name),
                    "schema": schema_name,
                    "state": "approved"
                }))
            }
            "block_schema" => {
                let schema_name = request
                    .get("params")
                    .and_then(|v| v.get("schema_name"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config(
                            "Missing schema_name parameter".to_string(),
                        )
                    })?;

                let mut node_guard = node.lock().await;
                node_guard.block_schema(schema_name)?;
                Ok(serde_json::json!({
                    "success": true,
                    "message": format!("Schema '{}' blocked successfully", schema_name),
                    "schema": schema_name,
                    "state": "blocked"
                }))
            }
            "get_schema_state" => {
                let schema_name = request
                    .get("params")
                    .and_then(|v| v.get("schema_name"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        crate::error::FoldDbError::Config(
                            "Missing schema_name parameter".to_string(),
                        )
                    })?;

                let node_guard = node.lock().await;
                let state = node_guard.get_schema_state(schema_name)?;
                let state_str = match state {
                    crate::schema::SchemaState::Available => "available",
                    crate::schema::SchemaState::Approved => "approved",
                    crate::schema::SchemaState::Blocked => "blocked",
                };
                Ok(serde_json::json!({
                    "schema": schema_name,
                    "state": state_str
                }))
            }
            _ => Err(crate::error::FoldDbError::Config(format!(
                "Unknown operation: {}",
                operation
            ))),
        }
    }

    async fn forward_request(
        request: &Value,
        target_node_id: &str,
        node: Arc<Mutex<DataFoldNode>>,
    ) -> FoldDbResult<Value> {
        let node_guard = node.lock().await;
        let mut network = node_guard.get_network_mut().await?;

        let peer_id = match network.get_peer_id_for_node(target_node_id) {
            Some(id) => {
                log_feature!(
                    LogFeature::TcpServer,
                    info,
                    "Found PeerId {} for node ID {}",
                    id,
                    target_node_id
                );
                id
            }
            None => match target_node_id.parse::<PeerId>() {
                Ok(id) => {
                    log_feature!(
                        LogFeature::TcpServer,
                        info,
                        "Parsed node ID {} as PeerId {}",
                        target_node_id,
                        id
                    );
                    network.register_node_id(target_node_id, id);
                    id
                }
                Err(_) => {
                    let id = PeerId::random();
                    log_feature!(
                        LogFeature::TcpServer,
                        info,
                        "Using placeholder PeerId {} for node ID {}",
                        id,
                        target_node_id
                    );
                    network.register_node_id(target_node_id, id);
                    id
                }
            },
        };

        drop(network);

        let mut forwarded_request = request.clone();
        if let Some(obj) = forwarded_request.as_object_mut() {
            obj.remove("target_node_id");
        }

        let operation = request
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FoldDbError::Config("Missing operation".to_string()))?;

        let schema_name = if operation == "query" || operation == "mutation" {
            request
                .get("params")
                .and_then(|v| v.get("schema"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| FoldDbError::Config("Missing schema parameter".to_string()))?
                .to_string()
        } else {
            "unknown".to_string()
        };

        log_feature!(
            LogFeature::TcpServer,
            info,
            "Assuming schema {} is available on target node",
            schema_name
        );
        log_feature!(
            LogFeature::TcpServer,
            info,
            "Forwarding request to node {} (peer {})",
            target_node_id,
            peer_id
        );

        let response = node_guard
            .forward_request(peer_id, forwarded_request)
            .await?;

        log_feature!(
            LogFeature::TcpServer,
            info,
            "Received response from node {} (peer {})",
            target_node_id,
            peer_id
        );
        Ok(response)
    }
}
