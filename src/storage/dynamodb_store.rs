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
use crate::storage::DynamoDbConfig;

/// DynamoDB-backed schema storage
pub struct DynamoDbSchemaStore {
    client: DynamoClient,
    table_name: String,
    /// Optional user_id that will be used as part of the partition key
    user_id: Option<String>,
}



impl DynamoDbSchemaStore {
    /// Create a new DynamoDB schema store
    /// Validates that the table exists before returning
    /// Create a new DynamoDB schema store
    /// Validates that the table exists before returning
    pub async fn new(config: DynamoDbConfig) -> FoldDbResult<Self> {
        // Resolve table name from explicit tables config
        let table_name = config.tables.schemas.clone();

        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_sdk_dynamodb::config::Region::new(config.region.clone()))
            .load()
            .await;

        let client = DynamoClient::new(&aws_config);

        // Validate table exists or create it
        match client.describe_table().table_name(&table_name).send().await {
            Ok(resp) => {
                if let Some(table) = resp.table {
                    if let Some(status) = table.table_status {
                        log::info!("Table {} exists, status: {:?}", table_name, status);
                    }
                }
            },
            Err(e) => {
                log::warn!("Describe table failed: {}. Attempting to create table '{}'...", e, table_name);
                
                // Try to create table regardless of the specific error from describe
                // If it already exists, create will fail with ResourceInUse, which we can handle
                use aws_sdk_dynamodb::types::{AttributeDefinition, KeySchemaElement, KeyType, ScalarAttributeType, BillingMode};
                
                match client.create_table()
                    .table_name(&table_name)
                    .attribute_definitions(
                        AttributeDefinition::builder()
                            .attribute_name("PK")
                            .attribute_type(ScalarAttributeType::S)
                            .build()
                            .map_err(|e| FoldDbError::Config(e.to_string()))?
                    )
                    .attribute_definitions(
                        AttributeDefinition::builder()
                            .attribute_name("SK")
                            .attribute_type(ScalarAttributeType::S)
                            .build()
                            .map_err(|e| FoldDbError::Config(e.to_string()))?
                    )
                    .key_schema(
                        KeySchemaElement::builder()
                            .attribute_name("PK")
                            .key_type(KeyType::Hash)
                            .build()
                            .map_err(|e| FoldDbError::Config(e.to_string()))?
                    )
                    .key_schema(
                        KeySchemaElement::builder()
                            .attribute_name("SK")
                            .key_type(KeyType::Range)
                            .build()
                            .map_err(|e| FoldDbError::Config(e.to_string()))?
                    )
                    .billing_mode(BillingMode::PayPerRequest)
                    .send()
                    .await 
                {
                    Ok(_) => {
                        log::info!("Table creation initiated for {}", table_name);
                    },
                    Err(e) => {
                        let error_str = e.to_string();
                        if error_str.contains("ResourceInUseException") {
                             log::info!("Table {} already exists (ResourceInUse), waiting for it to be active...", table_name);
                        } else {
                             // If create failed for another reason, we return that error
                             return Err(FoldDbError::Config(format!("Failed to create table: {}", e)));
                        }
                    }
                }
                    
                // Wait for table to be active
                let mut attempts = 0;
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    match client.describe_table().table_name(&table_name).send().await {
                        Ok(resp) => {
                            if let Some(table) = resp.table {
                                if let Some(status) = table.table_status {
                                    use aws_sdk_dynamodb::types::TableStatus;
                                    if matches!(status, TableStatus::Active) {
                                        log::info!("Table {} is now ACTIVE", table_name);
                                        break;
                                    }
                                }
                            }
                        },
                        Err(_) => {
                            // Ignore errors while waiting (e.g. still creating)
                        }
                    }
                    attempts += 1;
                    if attempts >= 60 {
                         return Err(FoldDbError::Config("Table creation timed out".to_string()));
                    }
                }
            }
        }

        Ok(Self {
            client,
            table_name,
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


    // Note: These tests require a real DynamoDB table or LocalStack
    // They are integration tests and should be run with proper AWS credentials


}



