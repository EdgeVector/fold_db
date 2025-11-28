use super::error::{StorageError, StorageResult};
use super::traits::{KvStore, NamespacedStore};
use async_trait::async_trait;
use aws_sdk_dynamodb::{types::AttributeValue, Client};
use std::collections::HashMap;
use std::sync::Arc;

/// DynamoDB-backed KvStore implementation
///
/// Uses a separate DynamoDB table per namespace with:
/// - Partition Key (PK): user_id (for multi-tenant) or "default" (for single-tenant)
/// - Sort Key (SK): actual_key
/// - Value: binary data
///
/// The user_id is used as the partition key for multi-tenant isolation.
/// This design enables efficient Query operations instead of expensive Scans.
pub struct DynamoDbKvStore {
    client: Arc<Client>,
    table_name: String,
    /// Optional user_id that will be used as the partition key
    user_id: Option<String>,
}

impl DynamoDbKvStore {
    /// Create a new DynamoDB KvStore for a specific table
    ///
    /// - `table_name`: The DynamoDB table name (typically namespace-specific)
    /// - `user_id`: Optional user_id that will be used as the partition key (for multi-tenant isolation)
    pub fn new(client: Arc<Client>, table_name: String, user_id: Option<String>) -> Self {
        Self {
            client,
            table_name,
            user_id,
        }
    }
    
    /// Get the partition key to use for this store
    #[cfg(test)]
    pub fn get_partition_key(&self) -> String {
        self.get_partition_key_impl()
    }

    fn get_partition_key_impl(&self) -> String {
        self.user_id.clone().unwrap_or_else(|| "default".to_string())
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
        let pk = self.get_partition_key_impl();
        let sk = self.make_sort_key_impl(key);

        let result = self.client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk))
            .send()
            .await
            .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;

        if let Some(item) = result.item {
            if let Some(AttributeValue::B(data)) = item.get("Value") {
                return Ok(Some(data.as_ref().to_vec()));
            }
        }

        Ok(None)
    }
    
    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        let pk = self.get_partition_key_impl();
        let sk = self.make_sort_key_impl(key);

        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(pk))
            .item("SK", AttributeValue::S(sk))
            .item("Value", AttributeValue::B(value.into()))
            .send()
            .await
            .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;

        Ok(())
    }
    
    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let pk = self.get_partition_key_impl();
        let sk = self.make_sort_key_impl(key);

        let result = self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk))
            .return_values(aws_sdk_dynamodb::types::ReturnValue::AllOld)
            .send()
            .await
            .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;

        Ok(result.attributes.is_some())
    }
    
    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let pk = self.get_partition_key_impl();
        let sk = self.make_sort_key_impl(key);

        let result = self.client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk))
            .projection_expression("PK") // Only fetch key, not value
            .send()
            .await
            .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;

        Ok(result.item.is_some())
    }
    
    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let pk = self.get_partition_key_impl();
        let prefix_str = String::from_utf8_lossy(prefix).to_string();

        // Use Query instead of Scan for efficient prefix lookups
        // Query on partition key + begins_with on sort key
        let mut results = Vec::new();
        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut query = self.client
                .query()
                .table_name(&self.table_name)
                .key_condition_expression("PK = :pk AND begins_with(SK, :prefix)")
                .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                .expression_attribute_values(":prefix", AttributeValue::S(prefix_str.clone()));

            if let Some(key) = last_evaluated_key {
                query = query.set_exclusive_start_key(Some(key));
            }

            let response = query.send().await
                .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;

            if let Some(items) = response.items {
                for item in items {
                    if let (Some(AttributeValue::S(sk)), Some(AttributeValue::B(value))) =
                        (item.get("SK"), item.get("Value")) {
                        // The sort key is the actual key (no user_id prefix to remove)
                        results.push((sk.as_bytes().to_vec(), value.as_ref().to_vec()));
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
        let pk = self.get_partition_key_impl();

        // DynamoDB batch write supports up to 25 items per request
        for chunk in items.chunks(25) {
            let mut write_requests = Vec::new();

            for (key, value) in chunk {
                let sk = self.make_sort_key_impl(key);
                let mut item = HashMap::new();
                item.insert("PK".to_string(), AttributeValue::S(pk.clone()));
                item.insert("SK".to_string(), AttributeValue::S(sk));
                item.insert("Value".to_string(), AttributeValue::B(value.clone().into()));

                write_requests.push(
                    aws_sdk_dynamodb::types::WriteRequest::builder()
                        .put_request(
                            aws_sdk_dynamodb::types::PutRequest::builder()
                                .set_item(Some(item))
                                .build()
                                .map_err(|e| StorageError::DynamoDbError(e.to_string()))?
                        )
                        .build()
                );
            }

            let mut requests = HashMap::new();
            requests.insert(self.table_name.clone(), write_requests);

            self.client
                .batch_write_item()
                .set_request_items(Some(requests))
                .send()
                .await
                .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;
        }

        Ok(())
    }
    
    async fn batch_delete(&self, keys: Vec<Vec<u8>>) -> StorageResult<()> {
        let pk = self.get_partition_key_impl();

        for chunk in keys.chunks(25) {
            let mut write_requests = Vec::new();

            for key in chunk {
                let sk = self.make_sort_key_impl(&key);
                let mut key_map = HashMap::new();
                key_map.insert("PK".to_string(), AttributeValue::S(pk.clone()));
                key_map.insert("SK".to_string(), AttributeValue::S(sk));

                write_requests.push(
                    aws_sdk_dynamodb::types::WriteRequest::builder()
                        .delete_request(
                            aws_sdk_dynamodb::types::DeleteRequest::builder()
                                .set_key(Some(key_map))
                                .build()
                                .map_err(|e| StorageError::DynamoDbError(e.to_string()))?
                        )
                        .build()
                );
            }

            let mut requests = HashMap::new();
            requests.insert(self.table_name.clone(), write_requests);

            self.client
                .batch_write_item()
                .set_request_items(Some(requests))
                .send()
                .await
                .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;
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
}

/// DynamoDB-backed NamespacedStore
///
/// Each namespace maps to a separate DynamoDB table for optimal performance.
/// Table names follow the pattern: `{base_table_name}-{namespace}`
/// The user_id is used as the partition key for multi-tenant isolation.
pub struct DynamoDbNamespacedStore {
    client: Arc<Client>,
    /// Base table name prefix (e.g., "DataFoldStorage")
    base_table_name: String,
    /// Optional user_id that will be used as the partition key (for multi-tenant isolation)
    user_id: Option<String>,
}

impl DynamoDbNamespacedStore {
    /// Create a new DynamoDB NamespacedStore
    /// 
    /// - `client`: DynamoDB client
    /// - `base_table_name`: Base name for tables (actual table names will be `{base}-{namespace}`)
    pub fn new(client: Client, base_table_name: String) -> Self {
        Self {
            client: Arc::new(client),
            base_table_name,
            user_id: None,
        }
    }
    
    /// Set user_id for multi-tenant isolation
    /// The user_id will be used as the partition key
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    /// Generate table name for a namespace
    fn table_name_for_namespace(&self, namespace: &str) -> String {
        format!("{}-{}", self.base_table_name, namespace)
    }

    /// Test helper to get table name for a namespace
    #[cfg(test)]
    pub fn get_table_name_for_namespace(&self, namespace: &str) -> String {
        self.table_name_for_namespace(namespace)
    }
}

#[async_trait]
impl NamespacedStore for DynamoDbNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let table_name = self.table_name_for_namespace(name);
        
        let store = DynamoDbKvStore::new(
            self.client.clone(),
            table_name,
            self.user_id.clone()
        );
        
        Ok(Arc::new(store))
    }
    
    async fn list_namespaces(&self) -> StorageResult<Vec<String>> {
        // This would require scanning all keys and extracting unique namespaces
        // For now, we'll return an error indicating it's not implemented
        Err(StorageError::InvalidOperation(
            "list_namespaces not implemented for DynamoDB - requires custom implementation".to_string()
        ))
    }
    
    async fn delete_namespace(&self, _name: &str) -> StorageResult<bool> {
        // Would need to scan and delete all items with the namespace prefix
        Err(StorageError::InvalidOperation(
            "delete_namespace not implemented for DynamoDB - requires custom implementation".to_string()
        ))
    }
}
