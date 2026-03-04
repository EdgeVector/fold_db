//! DynamoDB-backed native index store with simplified key structure.
//!
//! Uses `user_id:feature` (classification) as partition key and `term` as sort key.
//! This enables efficient queries like "all words starting with 'hel'".

use super::dynamodb_utils::retry_batch_operation;
use super::error::{StorageError, StorageResult};
use super::traits::KvStore;
use async_trait::async_trait;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use std::collections::HashMap;
use std::sync::Arc;

/// Specialized DynamoDB store for native index with simplified key structure
/// Uses user_id:feature (classification) as partition key and term as sort key
/// Format: PK = user_id:feature, SK = term
/// This enables efficient queries like "all words starting with 'hel'"
/// The user_id is obtained dynamically from request context for multi-tenancy.
pub struct DynamoDbNativeIndexStore {
    client: Arc<Client>,
    table_name: String,
}

impl DynamoDbNativeIndexStore {
    pub(crate) fn new(client: Arc<Client>, table_name: String) -> Self {
        Self { client, table_name }
    }

    fn get_current_user_id(&self) -> StorageResult<String> {
        super::dynamodb_utils::require_user_context()
    }

    /// Parse key to extract feature and term
    /// Keys are in format: "feature:term" (e.g., "word:hello", "email:test@example.com")
    fn parse_key(&self, key: &[u8]) -> StorageResult<(String, String)> {
        let key_str = String::from_utf8_lossy(key);
        if let Some(colon_pos) = key_str.find(':') {
            let feature = key_str[..colon_pos].to_string();
            let term = key_str[colon_pos + 1..].to_string();
            Ok((feature, term))
        } else {
            Err(StorageError::SerializationError(format!(
                "Invalid key format: missing colon in '{}'",
                key_str
            )))
        }
    }

    /// Get partition key (feature) for native index
    /// Format: user_id:feature
    fn get_partition_key(&self, feature: &str) -> StorageResult<String> {
        Ok(format!("{}:{}", self.get_current_user_id()?, feature))
    }
}

#[async_trait]
impl KvStore for DynamoDbNativeIndexStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let (feature, term) = self.parse_key(key)?;
        let pk = self.get_partition_key(&feature)?;

        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(term))
            .send()
            .await
            .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;

        if let Some(item) = result.item {
            if let Some(AttributeValue::S(json_str)) = item.get("Value") {
                return Ok(Some(json_str.as_bytes().to_vec()));
            }
        }

        Ok(None)
    }

    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        let (feature, term) = self.parse_key(key)?;
        let pk = self.get_partition_key(&feature)?;

        let json_str = String::from_utf8(value).map_err(|e| {
            StorageError::SerializationError(format!("Invalid UTF-8 in value: {}", e))
        })?;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(pk))
            .item("SK", AttributeValue::S(term))
            .item("Value", AttributeValue::S(json_str))
            .send()
            .await
            .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let (feature, term) = self.parse_key(key)?;
        let pk = self.get_partition_key(&feature)?;

        let result = self
            .client
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(term))
            .return_values(aws_sdk_dynamodb::types::ReturnValue::AllOld)
            .send()
            .await
            .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;

        Ok(result.attributes.is_some())
    }

    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let (feature, term) = self.parse_key(key)?;
        let pk = self.get_partition_key(&feature)?;

        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(term))
            .projection_expression("PK")
            .send()
            .await
            .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;

        Ok(result.item.is_some())
    }

    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let prefix_str = String::from_utf8_lossy(prefix);

        // Parse prefix to extract feature
        // Prefixes are in format: "feature:" or "feature:term_prefix"
        let (feature, term_prefix) = if let Some(colon_pos) = prefix_str.find(':') {
            let feature = prefix_str[..colon_pos].to_string();
            let term_prefix = prefix_str[colon_pos + 1..].to_string();
            (feature, term_prefix)
        } else {
            return Err(StorageError::SerializationError(format!(
                "Invalid prefix format: missing colon in '{}'",
                prefix_str
            )));
        };

        let pk = self.get_partition_key(&feature)?;

        log::debug!(
            "[DynamoDbNativeIndexStore] scan_prefix: PK='{}', SK begins_with '{}'",
            pk,
            term_prefix
        );

        // Query with feature as PK and term prefix on SK
        let mut results = Vec::new();
        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut query = self
                .client
                .query()
                .table_name(&self.table_name)
                .key_condition_expression("PK = :pk AND begins_with(SK, :prefix)")
                .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                .expression_attribute_values(":prefix", AttributeValue::S(term_prefix.clone()));

            if let Some(key) = last_evaluated_key.take() {
                query = query.set_exclusive_start_key(Some(key));
            }

            let response = match query.send().await {
                Ok(r) => r,
                Err(e) => {
                    let error_str = e.to_string();
                    if error_str.contains("ResourceNotFoundException")
                        || error_str.contains("ResourceInUseException")
                        || error_str.contains("cannot do operations on a non-existent table")
                    {
                        return Ok(Vec::new());
                    }
                    return Err(StorageError::DynamoDbError(error_str));
                }
            };

            if let Some(items) = response.items {
                for item in items {
                    if let (Some(AttributeValue::S(sk)), Some(AttributeValue::S(json_str))) =
                        (item.get("SK"), item.get("Value"))
                    {
                        // Reconstruct full key: "feature:term"
                        let full_key = format!("{}:{}", feature, sk);
                        results.push((full_key.as_bytes().to_vec(), json_str.as_bytes().to_vec()));
                    }
                }
            }

            last_evaluated_key = response.last_evaluated_key;
            if last_evaluated_key.is_none() {
                break;
            }
        }

        Ok(results)
    }

    async fn batch_put(&self, items: Vec<(Vec<u8>, Vec<u8>)>) -> StorageResult<()> {
        // DynamoDB has a 25-item batch limit
        for chunk in items.chunks(super::dynamodb_utils::DYNAMODB_BATCH_SIZE) {
            let mut write_requests = Vec::new();

            for (key, value) in chunk {
                let (feature, term) = self.parse_key(key)?;
                let pk = self.get_partition_key(&feature)?;

                let json_str = String::from_utf8(value.clone()).map_err(|e| {
                    StorageError::SerializationError(format!("Invalid UTF-8 in batch value: {}", e))
                })?;

                let put_request = aws_sdk_dynamodb::types::PutRequest::builder()
                    .item("PK", AttributeValue::S(pk))
                    .item("SK", AttributeValue::S(term))
                    .item("Value", AttributeValue::S(json_str))
                    .build()
                    .map_err(|e| {
                        StorageError::DynamoDbError(format!(
                            "Failed to build put request for table '{}': {}",
                            self.table_name, e
                        ))
                    })?;

                let write_request = aws_sdk_dynamodb::types::WriteRequest::builder()
                    .put_request(put_request)
                    .build();

                write_requests.push(write_request);
            }

            retry_batch_operation(
                |requests| {
                    Box::pin(
                        self.client
                            .batch_write_item()
                            .request_items(&self.table_name, requests.to_vec())
                            .send(),
                    )
                },
                &self.table_name,
                write_requests,
            )
            .await
            .map_err(StorageError::DynamoDbError)?;
        }

        Ok(())
    }

    async fn batch_delete(&self, keys: Vec<Vec<u8>>) -> StorageResult<()> {
        // DynamoDB has a 25-item batch limit
        for chunk in keys.chunks(super::dynamodb_utils::DYNAMODB_BATCH_SIZE) {
            let mut write_requests = Vec::new();

            for key in chunk {
                let (feature, term) = self.parse_key(key)?;
                let pk = self.get_partition_key(&feature)?;

                let delete_request = aws_sdk_dynamodb::types::DeleteRequest::builder()
                    .key("PK", AttributeValue::S(pk))
                    .key("SK", AttributeValue::S(term))
                    .build()
                    .map_err(|e| {
                        StorageError::DynamoDbError(format!(
                            "Failed to build delete request for table '{}': {}",
                            self.table_name, e
                        ))
                    })?;

                let write_request = aws_sdk_dynamodb::types::WriteRequest::builder()
                    .delete_request(delete_request)
                    .build();

                write_requests.push(write_request);
            }

            retry_batch_operation(
                |requests| {
                    Box::pin(
                        self.client
                            .batch_write_item()
                            .request_items(&self.table_name, requests.to_vec())
                            .send(),
                    )
                },
                &self.table_name,
                write_requests,
            )
            .await
            .map_err(StorageError::DynamoDbError)?;
        }

        Ok(())
    }

    async fn flush(&self) -> StorageResult<()> {
        // DynamoDB is eventually consistent, no explicit flush needed
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "dynamodb-native-index"
    }

    fn execution_model(&self) -> super::traits::ExecutionModel {
        // DynamoDB is truly async (network I/O)
        super::traits::ExecutionModel::Async
    }

    fn flush_behavior(&self) -> super::traits::FlushBehavior {
        // DynamoDB is eventually consistent, flush is a no-op
        super::traits::FlushBehavior::NoOp
    }
}
