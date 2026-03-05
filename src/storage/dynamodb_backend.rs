use super::dynamodb_utils::{retry_batch_operation, MAX_RETRIES};
use super::error::{StorageError, StorageResult};
use super::traits::KvStore;
use crate::retry_operation;
use async_trait::async_trait;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use std::collections::HashMap;
use std::sync::Arc;

// Re-export from sibling modules for backward compatibility
pub use super::dynamodb_namespaced::{DynamoDbNamespacedStore, TableNameResolver};
pub use super::dynamodb_native_index::DynamoDbNativeIndexStore;

/// DynamoDB-backed KvStore implementation
///
/// Uses a separate DynamoDB table per namespace with:
/// - Partition Key (PK): user_id:key (format: user_id:actual_key)
/// - Sort Key (SK): actual_key
/// - Value: binary data
///
/// The user_id is obtained dynamically from request context (`get_current_user_id()`).
/// If no user context is available, operations return an error.
/// This design enables efficient Query operations instead of expensive Scans.
pub struct DynamoDbKvStore {
    client: Arc<Client>,
    table_name: String,
}

impl DynamoDbKvStore {
    /// Create a new DynamoDB KvStore for a specific table
    ///
    /// - `table_name`: The DynamoDB table name (typically namespace-specific)
    pub fn new(client: Arc<Client>, table_name: String) -> Self {
        Self { client, table_name }
    }

    fn get_partition_key_impl(&self) -> StorageResult<String> {
        super::dynamodb_utils::require_user_context()
    }

    /// Get partition key (user_id)
    /// Note: This is a change from previous implementation where PK was user_id:key
    /// This change enables Query operations with SK prefix
    fn get_partition_key_with_key(&self, _key: &[u8]) -> StorageResult<String> {
        super::dynamodb_utils::require_user_context()
    }

    /// Convert a byte key to a string for the sort key (no user_id prefixing)
    #[cfg(test)]
    pub fn make_sort_key(&self, key: &[u8]) -> String {
        self.make_sort_key_impl(key)
    }

    fn make_sort_key_impl(&self, key: &[u8]) -> String {
        String::from_utf8_lossy(key).to_string()
    }
}

#[async_trait]
impl KvStore for DynamoDbKvStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let pk = self.get_partition_key_with_key(key)?;
        let sk = self.make_sort_key_impl(key);
        let key_str = String::from_utf8_lossy(key);

        let result = retry_operation!(
            self.client
                .get_item()
                .table_name(&self.table_name)
                .key("PK", AttributeValue::S(pk.clone()))
                .key("SK", AttributeValue::S(sk.clone()))
                .send(),
            "get_item",
            &self.table_name,
            Some(&key_str),
            MAX_RETRIES,
            StorageError::DynamoDbError
        )?;

        if let Some(item) = result.item {
            if let Some(AttributeValue::S(json_str)) = item.get("Value") {
                return Ok(Some(json_str.as_bytes().to_vec()));
            }
        }

        Ok(None)
    }

    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        let pk = self.get_partition_key_with_key(key)?;
        let sk = self.make_sort_key_impl(key);
        let key_str = String::from_utf8_lossy(key);

        let json_str = String::from_utf8(value.clone()).map_err(|e| {
            StorageError::SerializationError(format!("Invalid UTF-8 in value: {}", e))
        })?;

        retry_operation!(
            self.client
                .put_item()
                .table_name(&self.table_name)
                .item("PK", AttributeValue::S(pk.clone()))
                .item("SK", AttributeValue::S(sk.clone()))
                .item("Value", AttributeValue::S(json_str.clone()))
                .send(),
            "put_item",
            &self.table_name,
            Some(&key_str),
            MAX_RETRIES,
            StorageError::DynamoDbError
        )?;

        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let pk = self.get_partition_key_with_key(key)?;
        let sk = self.make_sort_key_impl(key);
        let key_str = String::from_utf8_lossy(key);

        let result = retry_operation!(
            self.client
                .delete_item()
                .table_name(&self.table_name)
                .key("PK", AttributeValue::S(pk.clone()))
                .key("SK", AttributeValue::S(sk.clone()))
                .return_values(aws_sdk_dynamodb::types::ReturnValue::AllOld)
                .send(),
            "delete_item",
            &self.table_name,
            Some(&key_str),
            MAX_RETRIES,
            StorageError::DynamoDbError
        )?;

        Ok(result.attributes.is_some())
    }

    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let pk = self.get_partition_key_with_key(key)?;
        let sk = self.make_sort_key_impl(key);
        let key_str = String::from_utf8_lossy(key);

        let result = retry_operation!(
            self.client
                .get_item()
                .table_name(&self.table_name)
                .key("PK", AttributeValue::S(pk.clone()))
                .key("SK", AttributeValue::S(sk.clone()))
                .projection_expression("PK") // Only fetch key, not value
                .send(),
            "get_item",
            &self.table_name,
            Some(&key_str),
            MAX_RETRIES,
            StorageError::DynamoDbError
        )?;

        Ok(result.item.is_some())
    }

    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let prefix_str = String::from_utf8_lossy(prefix).to_string();
        let pk = self.get_partition_key_impl()?;

        log::debug!(
            "[DynamoDbKvStore] scan_prefix: table='{}', PK='{}', SK begins_with '{}'",
            self.table_name,
            pk,
            prefix_str
        );

        // Use Query instead of Scan for efficiency
        // PK = user_id, SK begins_with prefix
        let mut results = Vec::new();
        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut query = self
                .client
                .query()
                .table_name(&self.table_name)
                .key_condition_expression("PK = :pk AND begins_with(SK, :prefix)")
                .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                .expression_attribute_values(":prefix", AttributeValue::S(prefix_str.clone()));

            if let Some(key) = last_evaluated_key.take() {
                query = query.set_exclusive_start_key(Some(key));
            }

            let response = match query.send().await {
                Ok(r) => r,
                Err(e) => {
                    let error_str = e.to_string();
                    // If table doesn't exist or is still being created, return empty results
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
                        // The sort key is the actual key (no user_id prefix to remove)
                        results.push((sk.as_bytes().to_vec(), json_str.as_bytes().to_vec()));
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
        // DynamoDB batch write supports up to 25 items per request
        for chunk in items.chunks(super::dynamodb_utils::DYNAMODB_BATCH_SIZE) {
            let mut write_requests = Vec::new();

            for (key, value) in chunk {
                let pk = self.get_partition_key_with_key(key)?;
                let sk = self.make_sort_key_impl(key);
                let mut item = HashMap::new();
                let json_str = String::from_utf8(value.clone()).map_err(|e| {
                    StorageError::SerializationError(format!("Invalid UTF-8 in batch value: {}", e))
                })?;

                item.insert("PK".to_string(), AttributeValue::S(pk));
                item.insert("SK".to_string(), AttributeValue::S(sk));
                item.insert("Value".to_string(), AttributeValue::S(json_str));

                write_requests.push(
                    aws_sdk_dynamodb::types::WriteRequest::builder()
                        .put_request(
                            aws_sdk_dynamodb::types::PutRequest::builder()
                                .set_item(Some(item))
                                .build()
                                .map_err(|e| {
                                    StorageError::DynamoDbError(format!(
                                        "Failed to build put request for table '{}': {}",
                                        self.table_name, e
                                    ))
                                })?,
                        )
                        .build(),
                );
            }

            retry_batch_operation(
                |requests| {
                    let mut req_map = HashMap::new();
                    req_map.insert(self.table_name.clone(), requests.to_vec());
                    Box::pin(
                        self.client
                            .batch_write_item()
                            .set_request_items(Some(req_map))
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
        for chunk in keys.chunks(super::dynamodb_utils::DYNAMODB_BATCH_SIZE) {
            let mut write_requests = Vec::new();

            for key in chunk {
                let pk = self.get_partition_key_with_key(key)?;
                let sk = self.make_sort_key_impl(key);
                let mut key_map = HashMap::new();
                key_map.insert("PK".to_string(), AttributeValue::S(pk));
                key_map.insert("SK".to_string(), AttributeValue::S(sk));

                write_requests.push(
                    aws_sdk_dynamodb::types::WriteRequest::builder()
                        .delete_request(
                            aws_sdk_dynamodb::types::DeleteRequest::builder()
                                .set_key(Some(key_map))
                                .build()
                                .map_err(|e| {
                                    StorageError::DynamoDbError(format!(
                                        "Failed to build delete request for table '{}': {}",
                                        self.table_name, e
                                    ))
                                })?,
                        )
                        .build(),
                );
            }

            retry_batch_operation(
                |requests| {
                    let mut req_map = HashMap::new();
                    req_map.insert(self.table_name.clone(), requests.to_vec());
                    Box::pin(
                        self.client
                            .batch_write_item()
                            .set_request_items(Some(req_map))
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
        // DynamoDB is always consistent, no flush needed
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "dynamodb"
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

// (DynamoDbNativeIndexStore extracted to dynamodb_native_index.rs)
// (DynamoDbNamespacedStore + TableNameResolver extracted to dynamodb_namespaced.rs)

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_kv_store_sort_key() {
        let client = Arc::new(Client::from_conf(
            aws_sdk_dynamodb::Config::builder()
                .behavior_version(aws_sdk_dynamodb::config::BehaviorVersion::latest())
                .region(aws_sdk_dynamodb::config::Region::new("us-east-1"))
                .build(),
        ));
        let store = DynamoDbKvStore::new(client, "TestTable".to_string());

        let key = b"my_key";
        let sk = store.make_sort_key_impl(key);
        assert_eq!(sk, "my_key");
    }
}
