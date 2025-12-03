//! DynamoDB storage backend for schema service
//!
//! Uses DynamoDB as the primary storage for schemas, eliminating the need
//! for distributed locking since topology hashes are deterministic and unique.
//!
//! Key structure:
//! - Partition Key (PK): user_id (or "default" for single-tenant)
//! - Sort Key (SK): schema_name (topology_hash)
//! - Value: SchemaJson, MutationMappers, CreatedAt, UpdatedAt

use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client as DynamoClient;
use std::collections::HashMap;

use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::Schema;

use super::dynamodb_utils::{MAX_RETRIES, format_dynamodb_error};
use crate::retry_operation;

/// DynamoDB-backed schema storage
pub struct DynamoDbSchemaStore {
    client: DynamoClient,
    table_name: String,
    /// Optional user_id that will be used as part of the partition key
    user_id: Option<String>,
}

/// Configuration for DynamoDB schema storage
#[derive(Debug, Clone)]
pub struct DynamoDbConfig {
    /// DynamoDB table name
    pub table_name: String,
    /// AWS region
    pub region: String,
    /// Optional user_id that will be used as the partition key (hash key)
    pub user_id: Option<String>,
}

impl DynamoDbConfig {
    pub fn new(table_name: String, region: String) -> Self {
        Self { 
            table_name, 
            region,
            user_id: None,
        }
    }

    /// Create from environment variables:
    /// - DATAFOLD_DYNAMODB_TABLE (required)
    /// - DATAFOLD_DYNAMODB_REGION (required)
    /// - DATAFOLD_DYNAMODB_USER_ID (optional)
    pub fn from_env() -> Result<Self, String> {
        let table_name = std::env::var("DATAFOLD_DYNAMODB_TABLE")
            .map_err(|_| "Missing DATAFOLD_DYNAMODB_TABLE environment variable".to_string())?;
        
        let region = std::env::var("DATAFOLD_DYNAMODB_REGION")
            .map_err(|_| "Missing DATAFOLD_DYNAMODB_REGION environment variable".to_string())?;

        let user_id = std::env::var("DATAFOLD_DYNAMODB_USER_ID").ok();

        Ok(Self { 
            table_name, 
            region,
            user_id,
        })
    }

    /// Set user_id for multi-tenant isolation
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

impl DynamoDbSchemaStore {
    /// Create a new DynamoDB schema store
    /// Validates that the table exists before returning
    pub async fn new(config: DynamoDbConfig) -> FoldDbResult<Self> {
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_sdk_dynamodb::config::Region::new(config.region.clone()))
            .load()
            .await;

        let client = DynamoClient::new(&aws_config);

        // Validate table exists
        client
            .describe_table()
            .table_name(&config.table_name)
            .send()
            .await
            .map_err(|e| FoldDbError::Config(format!(
                "DynamoDB table '{}' does not exist or is not accessible: {}",
                config.table_name, e
            )))?;

        Ok(Self {
            client,
            table_name: config.table_name,
            user_id: config.user_id,
        })
    }

    /// Get the partition key (hash key) for schemas
    /// Format: user_id (or "default" if no user_id)
    /// The schema_name goes in the sort key (SK)
    fn get_partition_key(&self) -> String {
        self.user_id.clone().unwrap_or_else(|| "default".to_string())
    }

    /// Get a schema by its topology hash
    /// Includes retry logic for transient failures
    pub async fn get_schema(&self, schema_name: &str) -> FoldDbResult<Option<Schema>> {
        let pk = self.get_partition_key();
        let result = retry_operation!(
            self.client
                .get_item()
                .table_name(&self.table_name)
                .key("PK", AttributeValue::S(pk.clone()))
                .key("SK", AttributeValue::S(schema_name.to_string()))
                .send(),
            "get_item",
            &self.table_name,
            Some(schema_name),
            MAX_RETRIES,
            FoldDbError::Database
        )?;

        if let Some(item) = result.item {
            let schema_json = item
                .get("SchemaJson")
                .and_then(|v| v.as_s().ok())
                .ok_or_else(|| FoldDbError::Database(format!(
                    "Missing SchemaJson attribute in table '{}' for key '{}'",
                    self.table_name, schema_name
                )))?;

            let mut schema: Schema = serde_json::from_str(schema_json)
                .map_err(|e| FoldDbError::Serialization(format!(
                    "Failed to parse schema '{}' from table '{}': {}",
                    schema_name, self.table_name, e
                )))?;

            // Ensure schema name matches the requested schema_name (sort key) - this is the source of truth
            schema.name = schema_name.to_string();
            Ok(Some(schema))
        } else {
            Ok(None)
        }
    }

    /// Put a schema into DynamoDB
    /// Note: This is idempotent - writing the same schema (same topology_hash) multiple times is safe
    /// CreatedAt is only set if the schema doesn't exist, UpdatedAt is always set
    pub async fn put_schema(
        &self,
        schema: &Schema,
        mutation_mappers: &HashMap<String, String>,
    ) -> FoldDbResult<()> {
        let schema_json = serde_json::to_string(schema)
            .map_err(|e| FoldDbError::Serialization(format!(
                "Failed to serialize schema '{}': {}",
                schema.name, e
            )))?;

        let mutation_mappers_json = serde_json::to_string(mutation_mappers)
            .map_err(|e| FoldDbError::Serialization(format!(
                "Failed to serialize mutation_mappers for schema '{}': {}",
                schema.name, e
            )))?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check if schema already exists to preserve CreatedAt
        let pk = self.get_partition_key();
        let existing_item = self.client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk.clone()))
            .key("SK", AttributeValue::S(schema.name.clone()))
            .send()
            .await
            .map_err(|e| {
                let error_str = e.to_string();
                let error_msg = format_dynamodb_error("get_item", &self.table_name, Some(&schema.name), &error_str);
                FoldDbError::Database(error_msg)
            })?;

        let created_at = if let Some(item) = existing_item.item {
            // Preserve existing CreatedAt if it exists
            if let Some(AttributeValue::N(existing_created_at)) = item.get("CreatedAt") {
                existing_created_at.clone()
            } else {
                timestamp.to_string()
            }
        } else {
            // New schema, use current timestamp
            timestamp.to_string()
        };

        let pk = self.get_partition_key();
        retry_operation!(
            self.client
                .put_item()
                .table_name(&self.table_name)
                .item("PK", AttributeValue::S(pk.clone()))
                .item("SK", AttributeValue::S(schema.name.clone()))
                .item("SchemaJson", AttributeValue::S(schema_json.clone()))
                .item("MutationMappers", AttributeValue::S(mutation_mappers_json.clone()))
                .item("CreatedAt", AttributeValue::N(created_at.clone()))
                .item("UpdatedAt", AttributeValue::N(timestamp.to_string()))
                .send(),
            "put_item",
            &self.table_name,
            Some(&schema.name),
            MAX_RETRIES,
            FoldDbError::Database
        )?;

        Ok(())
    }

    /// List all schema names
    /// Uses Query operation for efficient retrieval
    pub async fn list_schema_names(&self) -> FoldDbResult<Vec<String>> {
        let mut schema_names = Vec::new();
        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;
        let pk = self.get_partition_key();

        loop {
            let key_to_use = last_evaluated_key.take();
            let mut retries = 0;

            let result = loop {
                let mut query_builder = self
                    .client
                    .query()
                    .table_name(&self.table_name)
                    .key_condition_expression("PK = :pk")
                    .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                    .projection_expression("PK, SK");

                if let Some(ref key) = key_to_use {
                    query_builder = query_builder.set_exclusive_start_key(Some(key.clone()));
                }

                match query_builder.send().await {
                    Ok(r) => break Ok(r),
                    Err(e) => {
                        let error_str = e.to_string();
                        use super::dynamodb_utils::{is_retryable_error, exponential_backoff, format_dynamodb_error};
                        if retries >= MAX_RETRIES {
                            break Err(FoldDbError::Database(format_dynamodb_error("query", &self.table_name, None, &error_str)));
                        }
                        if is_retryable_error(&error_str) {
                            let delay = exponential_backoff(retries);
                            tokio::time::sleep(delay).await;
                            retries += 1;
                            continue;
                        }
                        break Err(FoldDbError::Database(format_dynamodb_error("query", &self.table_name, None, &error_str)));
                    }
                }
            }?;

            if let Some(items) = result.items {
                for item in items {
                    // Extract schema name from SK (sort key)
                    if let Some(name) = item.get("SK").and_then(|v| v.as_s().ok()) {
                        schema_names.push(name.clone());
                    }
                }
            }

            last_evaluated_key = result.last_evaluated_key;
            if last_evaluated_key.is_none() {
                break;
            }
        }

        Ok(schema_names)
    }

    /// Get all schemas
    /// Uses Query operation for efficient retrieval
    pub async fn get_all_schemas(&self) -> FoldDbResult<Vec<Schema>> {
        let mut schemas = Vec::new();
        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;
        let pk = self.get_partition_key();

        loop {
            let key_to_use = last_evaluated_key.take();
            let mut retries = 0;

            let result = loop {
                let mut query_builder = self
                    .client
                    .query()
                    .table_name(&self.table_name)
                    .key_condition_expression("PK = :pk")
                    .expression_attribute_values(":pk", AttributeValue::S(pk.clone()));

                if let Some(ref key) = key_to_use {
                    query_builder = query_builder.set_exclusive_start_key(Some(key.clone()));
                }

                match query_builder.send().await {
                    Ok(r) => break Ok(r),
                    Err(e) => {
                        let error_str = e.to_string();
                        use super::dynamodb_utils::{is_retryable_error, exponential_backoff, format_dynamodb_error};
                        if retries >= MAX_RETRIES {
                            break Err(FoldDbError::Database(format_dynamodb_error("query", &self.table_name, None, &error_str)));
                        }
                        if is_retryable_error(&error_str) {
                            let delay = exponential_backoff(retries);
                            tokio::time::sleep(delay).await;
                            retries += 1;
                            continue;
                        }
                        break Err(FoldDbError::Database(format_dynamodb_error("query", &self.table_name, None, &error_str)));
                    }
                }
            }?;

            if let Some(items) = result.items {
                for item in items {
                    if let Some(schema_json) = item.get("SchemaJson").and_then(|v| v.as_s().ok()) {
                        let schema_name = item.get("SK")
                            .and_then(|v| v.as_s().ok())
                            .map(|s| s.clone())
                            .unwrap_or_else(|| "unknown".to_string());
                        
                        let mut schema: Schema = serde_json::from_str(schema_json)
                            .map_err(|e| FoldDbError::Serialization(format!(
                                "Failed to parse schema '{}' from table '{}': {}",
                                schema_name, self.table_name, e
                            )))?;
                        
                        // Ensure schema name matches the sort key (SK) - this is the source of truth
                        schema.name = schema_name;
                        schemas.push(schema);
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

    /// Delete all schemas (for testing/reset)
    /// Uses batch delete operations for efficiency
    pub async fn clear_all_schemas(&self) -> FoldDbResult<()> {
        let schema_names = self.list_schema_names().await?;

        if schema_names.is_empty() {
            return Ok(());
        }

        // Use batch delete (DynamoDB limit is 25 items per batch)
        const BATCH_SIZE: usize = 25;

        for chunk in schema_names.chunks(BATCH_SIZE) {
            let mut write_requests = Vec::new();

            for name in chunk {
                let pk = self.get_partition_key();
                let mut key_map = HashMap::new();
                key_map.insert("PK".to_string(), AttributeValue::S(pk.clone()));
                key_map.insert("SK".to_string(), AttributeValue::S(name.clone()));

                write_requests.push(
                    aws_sdk_dynamodb::types::WriteRequest::builder()
                        .delete_request(
                            aws_sdk_dynamodb::types::DeleteRequest::builder()
                                .set_key(Some(key_map))
                                .build()
                                .map_err(|e| FoldDbError::Database(format!(
                                    "Failed to build delete request for schema '{}' in table '{}': {}",
                                    name, self.table_name, e
                                )))?
                        )
                        .build()
                );
            }

            // Use helper function for batch retry logic
            use super::dynamodb_utils::retry_batch_operation;
            retry_batch_operation(
                |requests| {
                    let mut req_map = HashMap::new();
                    req_map.insert(self.table_name.clone(), requests.to_vec());
                    Box::pin(
                        self.client
                            .batch_write_item()
                            .set_request_items(Some(req_map))
                            .send()
                    )
                },
                &self.table_name,
                write_requests,
            )
            .await
            .map_err(FoldDbError::Database)?;
        }

        Ok(())
    }

    /// Check if a schema exists
    pub async fn schema_exists(&self, schema_name: &str) -> FoldDbResult<bool> {
        Ok(self.get_schema(schema_name).await?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::{JsonTopology, PrimitiveType, SchemaType, TopologyNode};

    // Note: These tests require a real DynamoDB table or LocalStack
    // They are integration tests and should be run with proper AWS credentials

    #[tokio::test]
    #[ignore] // Run with `cargo test -- --ignored` when DynamoDB is available
    async fn test_put_and_get_schema() {
        let config = DynamoDbConfig {
            table_name: "test-schemas".to_string(),
            region: "us-east-1".to_string(),
            user_id: None,
        };

        let store = DynamoDbSchemaStore::new(config).await.unwrap();

        let mut schema = Schema::new(
            "TestSchema".to_string(),
            SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "name".to_string()]),
            None,
            None,
        );

        schema.set_field_topology(
            "id".to_string(),
            JsonTopology::new(TopologyNode::Primitive {
                value: PrimitiveType::String,
                classifications: Some(vec!["word".to_string()]),
            }),
        );

        schema.set_field_topology(
            "name".to_string(),
            JsonTopology::new(TopologyNode::Primitive {
                value: PrimitiveType::String,
                classifications: Some(vec!["word".to_string()]),
            }),
        );

        // Compute topology hash
        schema.compute_schema_topology_hash();
        let schema_name = schema.get_topology_hash().unwrap().clone();

        // Put schema
        store.put_schema(&schema, &HashMap::new()).await.unwrap();

        // Get schema back
        let retrieved = store.get_schema(&schema_name).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved_schema = retrieved.unwrap();
        assert_eq!(retrieved_schema.name, schema_name);
        assert_eq!(retrieved_schema.field_topologies, schema.field_topologies);
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_schemas() {
        let config = DynamoDbConfig {
            table_name: "test-schemas".to_string(),
            region: "us-east-1".to_string(),
            user_id: None,
        };

        let store = DynamoDbSchemaStore::new(config).await.unwrap();

        let schemas = store.list_schema_names().await.unwrap();
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use std::sync::Mutex;
    use once_cell::sync::Lazy;

    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn test_config_from_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        // Set environment variables
        unsafe {
            std::env::set_var("DATAFOLD_DYNAMODB_TABLE", "EnvTable");
            std::env::set_var("DATAFOLD_DYNAMODB_REGION", "us-west-2");
            std::env::set_var("DATAFOLD_DYNAMODB_USER_ID", "env_user");
        }

        let config = DynamoDbConfig::from_env().expect("Failed to create config from env");
        
        assert_eq!(config.table_name, "EnvTable");
        assert_eq!(config.region, "us-west-2");
        assert_eq!(config.user_id, Some("env_user".to_string()));

        // Clean up
        unsafe {
            std::env::remove_var("DATAFOLD_DYNAMODB_TABLE");
            std::env::remove_var("DATAFOLD_DYNAMODB_REGION");
            std::env::remove_var("DATAFOLD_DYNAMODB_USER_ID");
        }
    }

    #[test]
    fn test_config_missing_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        // Ensure vars are unset
        unsafe {
            std::env::remove_var("DATAFOLD_DYNAMODB_TABLE");
            std::env::remove_var("DATAFOLD_DYNAMODB_REGION");
            std::env::remove_var("DATAFOLD_DYNAMODB_USER_ID");
        }

        let result = DynamoDbConfig::from_env();
        assert!(result.is_err());
    }
}

