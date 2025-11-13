//! DynamoDB storage backend for schema service
//!
//! Uses DynamoDB as the primary storage for schemas, eliminating the need
//! for distributed locking since topology hashes are deterministic and unique.

use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client as DynamoClient;
use std::collections::HashMap;

use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::Schema;

/// DynamoDB-backed schema storage
pub struct DynamoDbSchemaStore {
    client: DynamoClient,
    table_name: String,
}

/// Configuration for DynamoDB schema storage
#[derive(Debug, Clone)]
pub struct DynamoDbConfig {
    /// DynamoDB table name
    pub table_name: String,
    /// AWS region
    pub region: String,
}

impl DynamoDbConfig {
    pub fn new(table_name: String, region: String) -> Self {
        Self { table_name, region }
    }

    /// Create from environment variables:
    /// - DATAFOLD_DYNAMODB_TABLE (required)
    /// - DATAFOLD_DYNAMODB_REGION (required)
    pub fn from_env() -> Result<Self, String> {
        let table_name = std::env::var("DATAFOLD_DYNAMODB_TABLE")
            .map_err(|_| "Missing DATAFOLD_DYNAMODB_TABLE environment variable".to_string())?;
        
        let region = std::env::var("DATAFOLD_DYNAMODB_REGION")
            .map_err(|_| "Missing DATAFOLD_DYNAMODB_REGION environment variable".to_string())?;

        Ok(Self { table_name, region })
    }
}

impl DynamoDbSchemaStore {
    /// Create a new DynamoDB schema store
    pub async fn new(config: DynamoDbConfig) -> FoldDbResult<Self> {
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_sdk_dynamodb::config::Region::new(config.region))
            .load()
            .await;

        let client = DynamoClient::new(&aws_config);

        Ok(Self {
            client,
            table_name: config.table_name,
        })
    }

    /// Get a schema by its topology hash
    pub async fn get_schema(&self, schema_name: &str) -> FoldDbResult<Option<Schema>> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("SchemaName", AttributeValue::S(schema_name.to_string()))
            .send()
            .await
            .map_err(|e| FoldDbError::Database(format!("DynamoDB get_item failed: {}", e)))?;

        if let Some(item) = result.item {
            let schema_json = item
                .get("SchemaJson")
                .and_then(|v| v.as_s().ok())
                .ok_or_else(|| FoldDbError::Database("Missing SchemaJson attribute".to_string()))?;

            let schema: Schema = serde_json::from_str(schema_json)
                .map_err(|e| FoldDbError::Serialization(format!("Failed to parse schema: {}", e)))?;

            Ok(Some(schema))
        } else {
            Ok(None)
        }
    }

    /// Put a schema into DynamoDB
    /// Note: This is idempotent - writing the same schema (same topology_hash) multiple times is safe
    pub async fn put_schema(
        &self,
        schema: &Schema,
        mutation_mappers: &HashMap<String, String>,
    ) -> FoldDbResult<()> {
        let schema_json = serde_json::to_string(schema)
            .map_err(|e| FoldDbError::Serialization(format!("Failed to serialize schema: {}", e)))?;

        let mutation_mappers_json = serde_json::to_string(mutation_mappers)
            .map_err(|e| FoldDbError::Serialization(format!("Failed to serialize mutation_mappers: {}", e)))?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("SchemaName", AttributeValue::S(schema.name.clone()))
            .item("SchemaJson", AttributeValue::S(schema_json))
            .item("MutationMappers", AttributeValue::S(mutation_mappers_json))
            .item("CreatedAt", AttributeValue::N(timestamp.to_string()))
            .item("UpdatedAt", AttributeValue::N(timestamp.to_string()))
            .send()
            .await
            .map_err(|e| FoldDbError::Database(format!("DynamoDB put_item failed: {}", e)))?;

        Ok(())
    }

    /// List all schema names
    pub async fn list_schema_names(&self) -> FoldDbResult<Vec<String>> {
        let mut schema_names = Vec::new();
        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut scan_request = self
                .client
                .scan()
                .table_name(&self.table_name)
                .projection_expression("SchemaName");

            if let Some(key) = last_evaluated_key {
                scan_request = scan_request.set_exclusive_start_key(Some(key));
            }

            let result = scan_request
                .send()
                .await
                .map_err(|e| FoldDbError::Database(format!("DynamoDB scan failed: {}", e)))?;

            if let Some(items) = result.items {
                for item in items {
                    if let Some(name) = item.get("SchemaName").and_then(|v| v.as_s().ok()) {
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
    pub async fn get_all_schemas(&self) -> FoldDbResult<Vec<Schema>> {
        let mut schemas = Vec::new();
        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut scan_request = self.client.scan().table_name(&self.table_name);

            if let Some(key) = last_evaluated_key {
                scan_request = scan_request.set_exclusive_start_key(Some(key));
            }

            let result = scan_request
                .send()
                .await
                .map_err(|e| FoldDbError::Database(format!("DynamoDB scan failed: {}", e)))?;

            if let Some(items) = result.items {
                for item in items {
                    if let Some(schema_json) = item.get("SchemaJson").and_then(|v| v.as_s().ok()) {
                        let schema: Schema = serde_json::from_str(schema_json)
                            .map_err(|e| FoldDbError::Serialization(format!("Failed to parse schema: {}", e)))?;
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
    pub async fn clear_all_schemas(&self) -> FoldDbResult<()> {
        let schema_names = self.list_schema_names().await?;

        for name in schema_names {
            self.client
                .delete_item()
                .table_name(&self.table_name)
                .key("SchemaName", AttributeValue::S(name))
                .send()
                .await
                .map_err(|e| FoldDbError::Database(format!("DynamoDB delete_item failed: {}", e)))?;
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
        };

        let store = DynamoDbSchemaStore::new(config).await.unwrap();

        let schemas = store.list_schema_names().await.unwrap();
        assert!(schemas.iter().all(|schema| !schema.is_empty()));
    }
}

