use super::error::{StorageError, StorageResult};
use super::traits::{KvStore, NamespacedStore};
use async_trait::async_trait;
use aws_sdk_dynamodb::{types::AttributeValue, Client};
use std::collections::HashMap;
use std::sync::Arc;

/// DynamoDB-backed KvStore implementation
/// 
/// Uses a single DynamoDB table with:
/// - PK: namespace#key (or user_id#namespace#key for multi-tenant)
/// - Value: binary data
pub struct DynamoDbKvStore {
    client: Arc<Client>,
    table_name: String,
    namespace: String,
    /// Optional user_id for multi-tenant isolation
    user_id: Option<String>,
}

impl DynamoDbKvStore {
    pub fn new(client: Arc<Client>, table_name: String, namespace: String) -> Self {
        Self {
            client,
            table_name,
            namespace,
            user_id: None,
        }
    }
    
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    /// Generate the full partition key including user_id if present
    fn make_pk(&self, key: &[u8]) -> String {
        let key_str = String::from_utf8_lossy(key);
        
        if let Some(user_id) = &self.user_id {
            // Multi-tenant: user_id#namespace#key
            format!("{}#{}#{}", user_id, self.namespace, key_str)
        } else {
            // Single-tenant: namespace#key
            format!("{}#{}", self.namespace, key_str)
        }
    }
    
    /// Extract the key part from a full PK
    fn extract_key(&self, pk: &str) -> Option<String> {
        if let Some(user_id) = &self.user_id {
            pk.strip_prefix(&format!("{}#{}#", user_id, self.namespace))
                .map(|s| s.to_string())
        } else {
            pk.strip_prefix(&format!("{}#", self.namespace))
                .map(|s| s.to_string())
        }
    }
}

#[async_trait]
impl KvStore for DynamoDbKvStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let pk = self.make_pk(key);
        
        let result = self.client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
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
        let pk = self.make_pk(key);
        
        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(pk))
            .item("Value", AttributeValue::B(value.into()))
            .send()
            .await
            .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let pk = self.make_pk(key);
        
        let result = self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .return_values(aws_sdk_dynamodb::types::ReturnValue::AllOld)
            .send()
            .await
            .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;
        
        Ok(result.attributes.is_some())
    }
    
    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let pk = self.make_pk(key);
        
        let result = self.client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .projection_expression("PK") // Only fetch key, not value
            .send()
            .await
            .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;
        
        Ok(result.item.is_some())
    }
    
    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let prefix_str = String::from_utf8_lossy(prefix);
        let pk_prefix = if let Some(user_id) = &self.user_id {
            format!("{}#{}#{}", user_id, self.namespace, prefix_str)
        } else {
            format!("{}#{}", self.namespace, prefix_str)
        };
        
        let mut results = Vec::new();
        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;
        
        loop {
            let mut query = self.client
                .scan()
                .table_name(&self.table_name)
                .filter_expression("begins_with(PK, :prefix)")
                .expression_attribute_values(":prefix", AttributeValue::S(pk_prefix.clone()));
            
            if let Some(key) = last_evaluated_key {
                query = query.set_exclusive_start_key(Some(key));
            }
            
            let response = query.send().await
                .map_err(|e| StorageError::DynamoDbError(e.to_string()))?;
            
            if let Some(items) = response.items {
                for item in items {
                    if let (Some(AttributeValue::S(pk)), Some(AttributeValue::B(value))) = 
                        (item.get("PK"), item.get("Value")) {
                        // Extract the key part from PK (strip user_id and namespace)
                        if let Some(key_str) = self.extract_key(pk) {
                            results.push((key_str.as_bytes().to_vec(), value.as_ref().to_vec()));
                        }
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
        for chunk in items.chunks(25) {
            let mut write_requests = Vec::new();
            
            for (key, value) in chunk {
                let pk = self.make_pk(key);
                let mut item = HashMap::new();
                item.insert("PK".to_string(), AttributeValue::S(pk));
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
        for chunk in keys.chunks(25) {
            let mut write_requests = Vec::new();
            
            for key in chunk {
                let pk = self.make_pk(key);
                let mut key_map = HashMap::new();
                key_map.insert("PK".to_string(), AttributeValue::S(pk));
                
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
pub struct DynamoDbNamespacedStore {
    client: Arc<Client>,
    table_name: String,
    user_id: Option<String>,
}

impl DynamoDbNamespacedStore {
    pub fn new(client: Client, table_name: String) -> Self {
        Self {
            client: Arc::new(client),
            table_name,
            user_id: None,
        }
    }
    
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

#[async_trait]
impl NamespacedStore for DynamoDbNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let mut store = DynamoDbKvStore::new(
            self.client.clone(),
            self.table_name.clone(),
            name.to_string()
        );
        
        if let Some(user_id) = &self.user_id {
            store = store.with_user_id(user_id.clone());
        }
        
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
