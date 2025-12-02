use super::error::{StorageError, StorageResult};
use super::traits::{KvStore, NamespacedStore};
use super::dynamodb_utils::{MAX_RETRIES, retry_batch_operation};
use crate::retry_operation;
use async_trait::async_trait;
use aws_sdk_dynamodb::types::{
    AttributeDefinition, AttributeValue, BillingMode, KeySchemaElement, KeyType, ScalarAttributeType, TableStatus,
};
use aws_sdk_dynamodb::Client;
use std::collections::HashMap;
use std::sync::Arc;

/// DynamoDB-backed KvStore implementation
///
/// Uses a separate DynamoDB table per namespace with:
/// - Partition Key (PK): user_id:key (format: user_id:actual_key)
/// - Sort Key (SK): actual_key
/// - Value: binary data
///
/// The user_id:key format ensures all tables use consistent partition key structure.
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
        // For backward compatibility, return just user_id or "default"
        // The actual key will be in the sort key
        self.user_id.clone().unwrap_or_else(|| "default".to_string())
    }
    
    /// Get partition key in user_id:key format
    fn get_partition_key_with_key(&self, key: &[u8]) -> String {
        let key_str = String::from_utf8_lossy(key);
        let user_id = self.user_id.clone().unwrap_or_else(|| "default".to_string());
        format!("{}:{}", user_id, key_str)
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
        let pk = self.get_partition_key_with_key(key);
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
            if let Some(AttributeValue::B(data)) = item.get("Value") {
                return Ok(Some(data.as_ref().to_vec()));
            }
        }

        Ok(None)
    }
    
    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        let pk = self.get_partition_key_with_key(key);
        let sk = self.make_sort_key_impl(key);
        let key_str = String::from_utf8_lossy(key);

        retry_operation!(
            self.client
                .put_item()
                .table_name(&self.table_name)
                .item("PK", AttributeValue::S(pk.clone()))
                .item("SK", AttributeValue::S(sk.clone()))
                .item("Value", AttributeValue::B(value.clone().into()))
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
        let pk = self.get_partition_key_with_key(key);
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
        let pk = self.get_partition_key_with_key(key);
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
        
        // For prefix scans, we need to scan all partitions that match the prefix
        // Since PK is now user_id:key, we can't efficiently query by prefix on PK
        // We'll use a scan with filter expression instead
        let mut results = Vec::new();
        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut scan = self.client
                .scan()
                .table_name(&self.table_name)
                .filter_expression("begins_with(SK, :prefix)")
                .expression_attribute_values(":prefix", AttributeValue::S(prefix_str.clone()));

            if let Some(key) = last_evaluated_key.take() {
                scan = scan.set_exclusive_start_key(Some(key));
            }

            let response = match scan.send().await {
                Ok(r) => r,
                Err(e) => {
                    let error_str = e.to_string();
                    // If table doesn't exist or is still being created, return empty results
                    if error_str.contains("ResourceNotFoundException") 
                        || error_str.contains("ResourceInUseException")
                        || error_str.contains("cannot do operations on a non-existent table") {
                        return Ok(Vec::new());
                    }
                    return Err(StorageError::DynamoDbError(error_str));
                }
            };

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
        const BATCH_SIZE: usize = 25;

        // DynamoDB batch write supports up to 25 items per request
        for chunk in items.chunks(BATCH_SIZE) {
            let mut write_requests = Vec::new();

            for (key, value) in chunk {
                let pk = self.get_partition_key_with_key(key);
                let sk = self.make_sort_key_impl(key);
                let mut item = HashMap::new();
                item.insert("PK".to_string(), AttributeValue::S(pk));
                item.insert("SK".to_string(), AttributeValue::S(sk));
                item.insert("Value".to_string(), AttributeValue::B(value.clone().into()));

                write_requests.push(
                    aws_sdk_dynamodb::types::WriteRequest::builder()
                        .put_request(
                            aws_sdk_dynamodb::types::PutRequest::builder()
                                .set_item(Some(item))
                                .build()
                                .map_err(|e| StorageError::DynamoDbError(format!(
                                    "Failed to build put request for table '{}': {}",
                                    self.table_name, e
                                )))?
                        )
                        .build()
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
                            .send()
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
        const BATCH_SIZE: usize = 25;

        for chunk in keys.chunks(BATCH_SIZE) {
            let mut write_requests = Vec::new();

            for key in chunk {
                let pk = self.get_partition_key_with_key(key);
                let sk = self.make_sort_key_impl(&key);
                let mut key_map = HashMap::new();
                key_map.insert("PK".to_string(), AttributeValue::S(pk));
                key_map.insert("SK".to_string(), AttributeValue::S(sk));

                write_requests.push(
                    aws_sdk_dynamodb::types::WriteRequest::builder()
                        .delete_request(
                            aws_sdk_dynamodb::types::DeleteRequest::builder()
                                .set_key(Some(key_map))
                                .build()
                                .map_err(|e| StorageError::DynamoDbError(format!(
                                    "Failed to build delete request for table '{}': {}",
                                    self.table_name, e
                                )))?
                        )
                        .build()
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
                            .send()
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

/// Specialized DynamoDB store for native index with simplified key structure
/// Uses user_id:feature (classification) as partition key and term as sort key
/// Format: PK = user_id:feature, SK = term
/// This enables efficient queries like "all words starting with 'hel'"
struct DynamoDbNativeIndexStore {
    client: Arc<Client>,
    table_name: String,
    user_id: Option<String>,
}

impl DynamoDbNativeIndexStore {
    fn new(client: Arc<Client>, table_name: String, user_id: Option<String>) -> Self {
        Self {
            client,
            table_name,
            user_id,
        }
    }
    
    /// Parse key to extract feature and term
    /// Keys are in format: "feature:term" (e.g., "word:hello", "email:test@example.com")
    fn parse_key(&self, key: &[u8]) -> (String, String) {
        let key_str = String::from_utf8_lossy(key);
        if let Some(colon_pos) = key_str.find(':') {
            let feature = key_str[..colon_pos].to_string();
            let term = key_str[colon_pos + 1..].to_string();
            (feature, term)
        } else {
            // Fallback: treat entire key as term, use "word" as default feature
            ("word".to_string(), key_str.to_string())
        }
    }
    
    /// Get partition key (feature) for native index
    /// Format: user_id:feature (or default:feature if no user_id)
    fn get_partition_key(&self, feature: &str) -> String {
        if let Some(ref user_id) = self.user_id {
            format!("{}:{}", user_id, feature)
        } else {
            format!("default:{}", feature)
        }
    }
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
    
    /// Ensure a DynamoDB table exists, creating it if necessary
    async fn ensure_table_exists(&self, table_name: &str) -> StorageResult<()> {
        // First, check if the table exists
        match self.client
            .describe_table()
            .table_name(table_name)
            .send()
            .await
        {
            Ok(response) => {
                // Table exists, check if it's active
                if let Some(table) = response.table() {
                    if let Some(status) = table.table_status() {
                        if status == &aws_sdk_dynamodb::types::TableStatus::Active {
                            // Table exists and is active, we're good
                            return Ok(());
                        } else {
                            // Table exists but not active yet - wait a bit
                            log::debug!("Table {} exists but status is {:?}, waiting...", table_name, status);
                            // For now, we'll proceed anyway as the table will become active soon
                            return Ok(());
                        }
                    }
                }
                // Table exists (even if we couldn't check status), we're good
                return Ok(());
            }
            Err(e) => {
                let error_str = e.to_string();
                // Check for ResourceNotFoundException specifically
                if error_str.contains("ResourceNotFoundException") {
                    // Table doesn't exist, we'll create it below
                } else if error_str.contains("service error") {
                    // "service error" is often a transient error or permissions issue
                    // Try to proceed - if the table doesn't exist, creation will fail
                    // If it does exist, operations will work
                    log::warn!("Got 'service error' when checking table {} - assuming table exists and proceeding", table_name);
                    return Ok(());
                } else {
                    // For other errors, still try to proceed but log a warning
                    log::warn!("Unexpected error checking table {}: {} - proceeding anyway", table_name, error_str);
                    // Don't fail immediately - let the create attempt below handle it
                }
            }
        }
        
        // Table doesn't exist, create it
        let create_result = self.client
            .create_table()
            .table_name(table_name)
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("PK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .map_err(|e| StorageError::DynamoDbError(format!("Failed to build attribute definition: {}", e)))?
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("SK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .map_err(|e| StorageError::DynamoDbError(format!("Failed to build attribute definition: {}", e)))?
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("PK")
                    .key_type(KeyType::Hash)
                    .build()
                    .map_err(|e| StorageError::DynamoDbError(format!("Failed to build key schema: {}", e)))?
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("SK")
                    .key_type(KeyType::Range)
                    .build()
                    .map_err(|e| StorageError::DynamoDbError(format!("Failed to build key schema: {}", e)))?
            )
            .billing_mode(BillingMode::PayPerRequest)
            .send()
            .await;
        
        match create_result {
            Ok(_) => {
                // Wait for table to be ACTIVE before returning
                // Poll with exponential backoff (max 30 seconds total)
                let mut attempts = 0;
                const MAX_ATTEMPTS: u32 = 30;
                
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    
                    match self.client
                        .describe_table()
                        .table_name(table_name)
                        .send()
                        .await
                    {
                        Ok(response) => {
                            if let Some(table) = response.table {
                                if let Some(status) = table.table_status {
                                    if matches!(status, TableStatus::Active) {
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // Continue polling
                        }
                    }
                    
                    attempts += 1;
                    if attempts >= MAX_ATTEMPTS {
                        return Err(StorageError::DynamoDbError(format!(
                            "Table '{}' did not become ACTIVE within timeout",
                            table_name
                        )));
                    }
                }
            }
            Err(e) => {
                // If table was created by another process between our check and create, that's ok
                if e.to_string().contains("ResourceInUseException") {
                    Ok(())
                } else {
                    Err(StorageError::DynamoDbError(format!(
                        "Failed to create table {}: {}",
                        table_name, e
                    )))
                }
            }
        }
    }
}

#[async_trait]
impl KvStore for DynamoDbNativeIndexStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let (feature, term) = self.parse_key(key);
        let pk = self.get_partition_key(&feature);
        
        let result = self.client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(term))
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
        let (feature, term) = self.parse_key(key);
        let pk = self.get_partition_key(&feature);
        
        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(pk))
            .item("SK", AttributeValue::S(term))
            .item("Value", AttributeValue::B(value.into()))
            .send()
            .await
            .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let (feature, term) = self.parse_key(key);
        let pk = self.get_partition_key(&feature);
        
        let result = self.client
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
        let (feature, term) = self.parse_key(key);
        let pk = self.get_partition_key(&feature);
        
        let result = self.client
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
            // Fallback: treat entire prefix as term prefix, use "word" as default feature
            ("word".to_string(), prefix_str.to_string())
        };
        
        let pk = self.get_partition_key(&feature);
        
        // Query with feature as PK and term prefix on SK
        let mut results = Vec::new();
        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;
        
        loop {
            let mut query = self.client
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
                        || error_str.contains("cannot do operations on a non-existent table") {
                        return Ok(Vec::new());
                    }
                    return Err(StorageError::DynamoDbError(error_str));
                }
            };
            
            if let Some(items) = response.items {
                for item in items {
                    if let (Some(AttributeValue::S(sk)), Some(AttributeValue::B(value))) =
                        (item.get("SK"), item.get("Value")) {
                        // Reconstruct full key: "feature:term"
                        let full_key = format!("{}:{}", feature, sk);
                        results.push((full_key.as_bytes().to_vec(), value.as_ref().to_vec()));
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
        const BATCH_SIZE: usize = 25;

        for chunk in items.chunks(BATCH_SIZE) {
            let mut write_requests = Vec::new();
            
            for (key, value) in chunk {
                let (feature, term) = self.parse_key(key);
                let pk = self.get_partition_key(&feature);
                
                let put_request = aws_sdk_dynamodb::types::PutRequest::builder()
                    .item("PK", AttributeValue::S(pk))
                    .item("SK", AttributeValue::S(term))
                    .item("Value", AttributeValue::B(value.clone().into()))
                    .build()
                    .map_err(|e| StorageError::DynamoDbError(format!(
                        "Failed to build put request for table '{}': {}",
                        self.table_name, e
                    )))?;
                
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
                            .send()
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
        const BATCH_SIZE: usize = 25;

        for chunk in keys.chunks(BATCH_SIZE) {
            let mut write_requests = Vec::new();
            
            for key in chunk {
                let (feature, term) = self.parse_key(key);
                let pk = self.get_partition_key(&feature);
                
                let delete_request = aws_sdk_dynamodb::types::DeleteRequest::builder()
                    .key("PK", AttributeValue::S(pk))
                    .key("SK", AttributeValue::S(term))
                    .build()
                    .map_err(|e| StorageError::DynamoDbError(format!(
                        "Failed to build delete request for table '{}': {}",
                        self.table_name, e
                    )))?;
                
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
                            .send()
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

#[async_trait]
impl NamespacedStore for DynamoDbNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let table_name = self.table_name_for_namespace(name);
        
        // Ensure the table exists, create it if it doesn't
        self.ensure_table_exists(&table_name).await?;
        
        // For native_index namespace, use simplified key structure: feature as PK, term as SK
        // This enables efficient queries by feature type (word, email, etc.)
        if name == "native_index" {
            let store = DynamoDbNativeIndexStore::new(
                self.client.clone(),
                table_name,
                self.user_id.clone()
            );
            Ok(Arc::new(store))
        } else {
            let store = DynamoDbKvStore::new(
                self.client.clone(),
                table_name,
                self.user_id.clone()
            );
            Ok(Arc::new(store))
        }
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
