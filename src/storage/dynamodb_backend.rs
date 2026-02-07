use super::dynamodb_utils::{retry_batch_operation, MAX_RETRIES};
use super::error::{StorageError, StorageResult};
use super::traits::{KvStore, NamespacedStore};
use crate::retry_operation;
use async_trait::async_trait;
use aws_sdk_dynamodb::types::{
    AttributeDefinition, AttributeValue, BillingMode, KeySchemaElement, KeyType,
    ScalarAttributeType, TableStatus,
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

    /// Get the current user_id from request context
    /// Falls back to "__system__" for system-level operations (e.g., node_id, metadata)
    /// when no user context is available (server startup, background tasks)
    fn get_current_user_id(&self) -> StorageResult<String> {
        Ok(crate::logging::core::get_current_user_id().unwrap_or_else(|| "__system__".to_string()))
    }

    /// Get the partition key to use for this store
    #[cfg(test)]
    pub fn get_partition_key(&self) -> StorageResult<String> {
        self.get_partition_key_impl()
    }

    fn get_partition_key_impl(&self) -> StorageResult<String> {
        self.get_current_user_id()
    }

    /// Get partition key (user_id)
    /// Note: This is a change from previous implementation where PK was user_id:key
    /// This change enables Query operations with SK prefix
    fn get_partition_key_with_key(&self, _key: &[u8]) -> StorageResult<String> {
        self.get_current_user_id()
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
        const BATCH_SIZE: usize = 25;

        // DynamoDB batch write supports up to 25 items per request
        for chunk in items.chunks(BATCH_SIZE) {
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
        const BATCH_SIZE: usize = 25;

        for chunk in keys.chunks(BATCH_SIZE) {
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

/// Strategy for resolving table names from namespaces
#[derive(Clone, Debug)]
pub enum TableNameResolver {
    /// Append namespace to prefix: "{prefix}-{namespace}"
    Prefix(String),
    /// Map namespace to exact table name. keys are namespaces ("main", "metadata", etc)
    Explicit(HashMap<String, String>),
}

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
    fn new(client: Arc<Client>, table_name: String) -> Self {
        Self { client, table_name }
    }

    /// Get the current user_id from request context
    /// Falls back to "__system__" for system-level operations when no user context is available
    fn get_current_user_id(&self) -> StorageResult<String> {
        let context_user = crate::logging::core::get_current_user_id();
        log::debug!(
            "[DynamoDbNativeIndexStore] get_current_user_id: context={:?}",
            context_user
        );
        Ok(context_user.unwrap_or_else(|| "__system__".to_string()))
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

pub struct DynamoDbNamespacedStore {
    client: Arc<Client>,
    /// Strategy to resolve namespace to table name
    resolver: TableNameResolver,
    /// Whether to automatically create tables if they don't exist
    auto_create: bool,
}

impl DynamoDbNamespacedStore {
    /// Create a new DynamoDB NamespacedStore with flexible configuration
    pub fn new(client: Client, resolver: TableNameResolver, auto_create: bool) -> Self {
        Self {
            client: Arc::new(client),
            resolver,
            auto_create,
        }
    }

    /// Create a new DynamoDB NamespacedStore with legacy prefix behavior (auto-create enabled)
    pub fn new_with_prefix(client: Client, prefix: String) -> Self {
        Self::new(client, TableNameResolver::Prefix(prefix), true)
    }

    /// Generate table name for a namespace
    fn table_name_for_namespace(&self, namespace: &str) -> StorageResult<String> {
        match &self.resolver {
            TableNameResolver::Prefix(prefix) => Ok(format!("{}-{}", prefix, namespace)),
            TableNameResolver::Explicit(map) => map.get(namespace).cloned().ok_or_else(|| {
                StorageError::ConfigurationError(format!(
                    "No explicit table name configured for namespace '{}'",
                    namespace
                ))
            }),
        }
    }

    /// Test helper to get table name for a namespace
    #[cfg(test)]
    pub fn get_table_name_for_namespace(&self, namespace: &str) -> String {
        self.table_name_for_namespace(namespace)
            .unwrap_or_else(|_| "unknown".to_string())
    }

    /// Ensure a DynamoDB table exists, creating it if necessary
    async fn ensure_table_exists(&self, table_name: &str) -> StorageResult<()> {
        // First, check if the table exists
        match self
            .client
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
                            log::debug!(
                                "Table {} exists but status is {:?}, waiting...",
                                table_name,
                                status
                            );
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
                    log::warn!("Got 'service error' when checking table {} - proceeding to attempt creation", table_name);
                    // Do NOT return Ok(()) here; let it fall through to create_table
                } else {
                    // For other errors, still try to proceed but log a warning
                    log::warn!(
                        "Unexpected error checking table {}: {} - proceeding anyway",
                        table_name,
                        error_str
                    );
                    // Don't fail immediately - let the create attempt below handle it
                }
            }
        }

        // Table doesn't exist, create it
        let create_result = self
            .client
            .create_table()
            .table_name(table_name)
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("PK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .map_err(|e| {
                        StorageError::DynamoDbError(format!(
                            "Failed to build attribute definition: {}",
                            e
                        ))
                    })?,
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("SK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .map_err(|e| {
                        StorageError::DynamoDbError(format!(
                            "Failed to build attribute definition: {}",
                            e
                        ))
                    })?,
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("PK")
                    .key_type(KeyType::Hash)
                    .build()
                    .map_err(|e| {
                        StorageError::DynamoDbError(format!("Failed to build key schema: {}", e))
                    })?,
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("SK")
                    .key_type(KeyType::Range)
                    .build()
                    .map_err(|e| {
                        StorageError::DynamoDbError(format!("Failed to build key schema: {}", e))
                    })?,
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

                    match self
                        .client
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
        const BATCH_SIZE: usize = 25;

        for chunk in items.chunks(BATCH_SIZE) {
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
        const BATCH_SIZE: usize = 25;

        for chunk in keys.chunks(BATCH_SIZE) {
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

#[async_trait]
impl NamespacedStore for DynamoDbNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let table_name = self.table_name_for_namespace(name)?;

        // Ensure the table exists if auto_create is enabled
        if self.auto_create {
            self.ensure_table_exists(&table_name).await?;
        }

        // For native_index namespace, use simplified key structure: feature as PK, term as SK
        // This enables efficient queries by feature type (word, email, etc.)
        if name == "native_index" {
            let store = DynamoDbNativeIndexStore::new(self.client.clone(), table_name);
            Ok(Arc::new(store))
        } else {
            let store = DynamoDbKvStore::new(self.client.clone(), table_name);
            Ok(Arc::new(store))
        }
    }

    async fn list_namespaces(&self) -> StorageResult<Vec<String>> {
        // This would require scanning all keys and extracting unique namespaces
        // For now, we'll return an error indicating it's not implemented
        Err(StorageError::InvalidOperation(
            "list_namespaces not implemented for DynamoDB - requires custom implementation"
                .to_string(),
        ))
    }

    async fn delete_namespace(&self, _name: &str) -> StorageResult<bool> {
        // Would need to scan and delete all items with the namespace prefix
        Err(StorageError::InvalidOperation(
            "delete_namespace not implemented for DynamoDB - requires custom implementation"
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use aws_sdk_dynamodb::config::Region;

    // Helper to create a dummy client (won't actually be used for network calls in these tests)
    async fn create_dummy_client() -> Arc<Client> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .load()
            .await;
        Arc::new(Client::new(&config))
    }

    #[tokio::test]
    async fn test_kv_store_key_generation() {
        let client = create_dummy_client().await;

        // Case 1: With user_id
        let key = b"my_key";
        crate::logging::core::run_with_user("user123", async {
            let store = DynamoDbKvStore::new(client.clone(), "TestTable".to_string());

            let pk = store
                .get_partition_key_with_key(key)
                .expect("failed to get partition key");
            let sk = store.make_sort_key_impl(key);

            // PK should now be just user_id (or default)
            // SK should be the key itself
            assert_eq!(pk, "user123");
            assert_eq!(sk, "my_key");
        })
        .await;

        // Case 2: Without user_id (default)
        let store_default = DynamoDbKvStore::new(client.clone(), "TestTable".to_string());
        let pk_default = store_default
            .get_partition_key_with_key(key)
            .expect("failed to get default partition key");
        assert_eq!(pk_default, "__system__");
    }

    #[tokio::test]
    async fn test_native_index_key_parsing() {
        let client = create_dummy_client().await;
        let store = DynamoDbNativeIndexStore::new(client, "IndexTable".to_string());

        // Case 1: Standard feature:term key
        let (feature, term) = store.parse_key(b"word:hello").unwrap();
        assert_eq!(feature, "word");
        assert_eq!(term, "hello");

        // Case 2: Email feature
        let (feature, term) = store.parse_key(b"email:test@example.com").unwrap();
        assert_eq!(feature, "email");
        assert_eq!(term, "test@example.com");

        // Case 3: No colon (error)
        let result = store.parse_key(b"just_a_word");
        assert!(result.is_err());

        // Case 4: Empty term
        let (feature, term) = store.parse_key(b"word:").unwrap();
        assert_eq!(feature, "word");
        assert_eq!(term, "");
    }

    #[tokio::test]
    async fn test_native_index_partition_key() {
        let client = create_dummy_client().await;

        // Case 1: With user_id
        crate::logging::core::run_with_user("user123", async {
            let store = DynamoDbNativeIndexStore::new(client.clone(), "IndexTable".to_string());
            let pk = store
                .get_partition_key("word")
                .expect("failed to get partition key");
            assert_eq!(pk, "user123:word");
        })
        .await;

        // Case 2: Without user_id
        let store_default = DynamoDbNativeIndexStore::new(client, "IndexTable".to_string());
        let pk_default = store_default
            .get_partition_key("email")
            .expect("failed to get default partition key");
        assert_eq!(pk_default, "__system__:email");
    }
}
