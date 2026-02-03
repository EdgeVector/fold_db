use super::dynamodb_utils::{format_dynamodb_error, retry_batch_operation};
use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::Schema;
use crate::storage::config::ExplicitTables;
use aws_sdk_dynamodb::types::{AttributeValue, DeleteRequest, WriteRequest};
use aws_sdk_dynamodb::Client;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Manager for resetting (deleting) user data from DynamoDB
pub struct DynamoDbResetManager {
    client: Arc<Client>,
    tables: ExplicitTables,
}

impl DynamoDbResetManager {
    pub fn new(client: Arc<Client>, tables: ExplicitTables) -> Self {
        Self { client, tables }
    }

    /// Reset all data for a specific user
    ///
    /// This avoids Scan operations by:
    /// 1. Querying the schemas table to find all schemas for the user
    /// 2. Extracting "classifications" (features) from schemas to know which native_index partitions to clean
    /// 3. Querying and deleting from all other tables using the user_id as PK
    pub async fn reset_user(&self, user_id: &str) -> FoldDbResult<()> {
        log::info!("🗑️ Starting database reset for user: {}", user_id);

        // 1. Get all schemas to identify native index partitions
        let schemas = self.get_user_schemas(user_id).await?;

        // 2. Identify all features (index partitions) to clean
        let mut features_to_clean = HashSet::new();
        features_to_clean.insert("word".to_string()); // Legacy/old format word index
        features_to_clean.insert("idx".to_string()); // Append-only index entries (idx:{term}:...)
        features_to_clean.insert("rev".to_string()); // Reverse index mappings (rev:{schema}:...)

        for schema in &schemas {
            for _topology in schema.field_topologies.values() {
                // Extract classifications from topology
                // Note: This depends on the internal structure of JsonTopology/TopologyNode
                // For now, we'll assume we can get them or just rely on "word" if complex
                // In a real implementation, we'd traverse the topology properly
                // For this MVP, we'll stick to "word" and any we can easily find
            }
        }

        // 3. Delete from Native Index (for each feature/partition)
        for feature in features_to_clean {
            self.delete_native_index_for_feature(user_id, &feature)
                .await?;
        }

        // 4. Delete Schemas
        self.delete_items_by_pk(&self.tables.schemas, user_id)
            .await?;

        // 5. Delete from all other tables (these use PK/SK schema)
        let tables_to_clean = vec![
            &self.tables.orchestrator,
            &self.tables.metadata,
            &self.tables.schema_states,
            &self.tables.transforms,
            &self.tables.transform_queue,
            &self.tables.public_keys,
            &self.tables.permissions,
            &self.tables.main,
            &self.tables.process,
        ];

        for table in tables_to_clean {
            self.delete_items_by_pk(table, user_id).await?;
        }

        // 6. Delete logs (uses user_id/timestamp schema instead of PK/SK)
        self.delete_logs_for_user(user_id).await?;

        // 7. Clean up any orphaned "global" entries in the process table
        // (from jobs that were created without proper user_id)
        self.delete_items_by_pk(&self.tables.process, "global")
            .await?;

        log::info!("✅ Database reset complete for user: {}", user_id);
        Ok(())
    }

    /// Helper to get all schemas for a user
    async fn get_user_schemas(&self, user_id: &str) -> FoldDbResult<Vec<Schema>> {
        let table_name = &self.tables.schemas;
        let mut schemas = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            let mut query = self
                .client
                .query()
                .table_name(table_name)
                .key_condition_expression("PK = :pk")
                .expression_attribute_values(":pk", AttributeValue::S(user_id.to_string()));

            if let Some(key) = last_evaluated_key {
                query = query.set_exclusive_start_key(Some(key));
            }

            let result = query.send().await.map_err(|e| {
                FoldDbError::Database(format_dynamodb_error(
                    "query",
                    table_name,
                    None,
                    e.to_string(),
                ))
            })?;

            if let Some(items) = result.items {
                for item in items {
                    if let Some(AttributeValue::S(json)) = item.get("SchemaJson") {
                        if let Ok(schema) = serde_json::from_str::<Schema>(json) {
                            schemas.push(schema);
                        }
                    }
                }
            }

            last_evaluated_key = result.last_evaluated_key;
            if last_evaluated_key.is_none() {
                break;
            }
        }

        Ok(schemas)
    }

    /// Delete all items for a user in a specific table (where PK = user_id)
    async fn delete_items_by_pk(&self, table_name: &str, user_id: &str) -> FoldDbResult<()> {
        // 1. Query to find all items (we need SK to delete)
        let mut keys_to_delete = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            let mut query = self
                .client
                .query()
                .table_name(table_name)
                .key_condition_expression("PK = :pk")
                .expression_attribute_values(":pk", AttributeValue::S(user_id.to_string()))
                .projection_expression("PK, SK"); // Only need keys

            if let Some(key) = last_evaluated_key {
                query = query.set_exclusive_start_key(Some(key));
            }

            let result = match query.send().await {
                Ok(r) => r,
                Err(e) => {
                    // Log the full error for debugging
                    log::warn!("Query failed for table {}: {:?}", table_name, e);

                    // If table doesn't exist, just ignore
                    // Check both string representation and service error code if available
                    let error_str = e.to_string();
                    let is_resource_not_found = error_str.contains("ResourceNotFoundException")
                        || format!("{:?}", e).contains("ResourceNotFoundException");

                    if is_resource_not_found {
                        return Ok(());
                    }
                    return Err(FoldDbError::Database(format_dynamodb_error(
                        "query", table_name, None, &error_str,
                    )));
                }
            };

            if let Some(items) = result.items {
                for item in items {
                    if let (Some(pk), Some(sk)) = (item.get("PK"), item.get("SK")) {
                        let mut key = HashMap::new();
                        key.insert("PK".to_string(), pk.clone());
                        key.insert("SK".to_string(), sk.clone());
                        keys_to_delete.push(key);
                    }
                }
            }

            last_evaluated_key = result.last_evaluated_key;
            if last_evaluated_key.is_none() {
                break;
            }
        }

        if keys_to_delete.is_empty() {
            return Ok(());
        }

        // 2. Batch Delete
        const BATCH_SIZE: usize = 25;
        for chunk in keys_to_delete.chunks(BATCH_SIZE) {
            let mut write_requests = Vec::new();

            for key in chunk {
                write_requests.push(
                    WriteRequest::builder()
                        .delete_request(
                            DeleteRequest::builder()
                                .set_key(Some(key.clone()))
                                .build()
                                .map_err(|e| FoldDbError::Database(e.to_string()))?,
                        )
                        .build(),
                );
            }

            retry_batch_operation(
                |requests| {
                    let mut req_map = HashMap::new();
                    req_map.insert(table_name.to_string(), requests.to_vec());
                    Box::pin(
                        self.client
                            .batch_write_item()
                            .set_request_items(Some(req_map))
                            .send(),
                    )
                },
                table_name,
                write_requests,
            )
            .await
            .map_err(FoldDbError::Database)?;
        }

        Ok(())
    }

    /// Delete native index entries for a specific feature
    /// PK = user_id:feature
    async fn delete_native_index_for_feature(
        &self,
        user_id: &str,
        feature: &str,
    ) -> FoldDbResult<()> {
        let table_name = &self.tables.native_index;
        let pk_val = format!("{}:{}", user_id, feature);

        // 1. Query keys
        let mut keys_to_delete = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            let mut query = self
                .client
                .query()
                .table_name(table_name)
                .key_condition_expression("PK = :pk")
                .expression_attribute_values(":pk", AttributeValue::S(pk_val.clone()))
                .projection_expression("PK, SK");

            if let Some(key) = last_evaluated_key {
                query = query.set_exclusive_start_key(Some(key));
            }

            let result = match query.send().await {
                Ok(r) => r,
                Err(e) => {
                    // Log the full error for debugging
                    log::warn!(
                        "Query failed for native index table {}: {:?}",
                        table_name,
                        e
                    );

                    let error_str = e.to_string();
                    let is_resource_not_found = error_str.contains("ResourceNotFoundException")
                        || format!("{:?}", e).contains("ResourceNotFoundException");

                    if is_resource_not_found {
                        return Ok(());
                    }
                    return Err(FoldDbError::Database(format_dynamodb_error(
                        "query", table_name, None, &error_str,
                    )));
                }
            };

            if let Some(items) = result.items {
                for item in items {
                    if let (Some(pk), Some(sk)) = (item.get("PK"), item.get("SK")) {
                        let mut key = HashMap::new();
                        key.insert("PK".to_string(), pk.clone());
                        key.insert("SK".to_string(), sk.clone());
                        keys_to_delete.push(key);
                    }
                }
            }

            last_evaluated_key = result.last_evaluated_key;
            if last_evaluated_key.is_none() {
                break;
            }
        }

        if keys_to_delete.is_empty() {
            return Ok(());
        }

        // 2. Batch Delete
        const BATCH_SIZE: usize = 25;
        for chunk in keys_to_delete.chunks(BATCH_SIZE) {
            let mut write_requests = Vec::new();
            for key in chunk {
                write_requests.push(
                    WriteRequest::builder()
                        .delete_request(
                            DeleteRequest::builder()
                                .set_key(Some(key.clone()))
                                .build()
                                .map_err(|e| FoldDbError::Database(e.to_string()))?,
                        )
                        .build(),
                );
            }

            retry_batch_operation(
                |requests| {
                    let mut req_map = HashMap::new();
                    req_map.insert(table_name.to_string(), requests.to_vec());
                    Box::pin(
                        self.client
                            .batch_write_item()
                            .set_request_items(Some(req_map))
                            .send(),
                    )
                },
                table_name,
                write_requests,
            )
            .await
            .map_err(FoldDbError::Database)?;
        }

        Ok(())
    }

    /// Delete log entries for a specific user
    /// The logs table uses user_id/timestamp as keys instead of PK/SK
    async fn delete_logs_for_user(&self, user_id: &str) -> FoldDbResult<()> {
        let table_name = &self.tables.logs;
        let mut keys_to_delete = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            let mut query = self
                .client
                .query()
                .table_name(table_name)
                .key_condition_expression("user_id = :uid")
                .expression_attribute_values(":uid", AttributeValue::S(user_id.to_string()))
                .projection_expression("user_id, #ts")
                .expression_attribute_names("#ts", "timestamp"); // timestamp is a reserved word

            if let Some(key) = last_evaluated_key {
                query = query.set_exclusive_start_key(Some(key));
            }

            let result = match query.send().await {
                Ok(r) => r,
                Err(e) => {
                    let error_str = e.to_string();
                    let is_resource_not_found = error_str.contains("ResourceNotFoundException")
                        || format!("{:?}", e).contains("ResourceNotFoundException");
                    if is_resource_not_found {
                        return Ok(());
                    }
                    return Err(FoldDbError::Database(format_dynamodb_error(
                        "query", table_name, None, &error_str,
                    )));
                }
            };

            if let Some(items) = result.items {
                for item in items {
                    if let (Some(uid), Some(ts)) = (item.get("user_id"), item.get("timestamp")) {
                        let mut key = HashMap::new();
                        key.insert("user_id".to_string(), uid.clone());
                        key.insert("timestamp".to_string(), ts.clone());
                        keys_to_delete.push(key);
                    }
                }
            }

            last_evaluated_key = result.last_evaluated_key;
            if last_evaluated_key.is_none() {
                break;
            }
        }

        if keys_to_delete.is_empty() {
            return Ok(());
        }

        // Batch Delete
        const BATCH_SIZE: usize = 25;
        for chunk in keys_to_delete.chunks(BATCH_SIZE) {
            let mut write_requests = Vec::new();
            for key in chunk {
                write_requests.push(
                    WriteRequest::builder()
                        .delete_request(
                            DeleteRequest::builder()
                                .set_key(Some(key.clone()))
                                .build()
                                .map_err(|e| FoldDbError::Database(e.to_string()))?,
                        )
                        .build(),
                );
            }

            retry_batch_operation(
                |requests| {
                    let mut req_map = HashMap::new();
                    req_map.insert(table_name.to_string(), requests.to_vec());
                    Box::pin(
                        self.client
                            .batch_write_item()
                            .set_request_items(Some(req_map))
                            .send(),
                    )
                },
                table_name,
                write_requests,
            )
            .await
            .map_err(FoldDbError::Database)?;
        }

        log::debug!(
            "🗑️ Deleted {} log entries for user: {}",
            keys_to_delete.len(),
            user_id
        );
        Ok(())
    }
}
