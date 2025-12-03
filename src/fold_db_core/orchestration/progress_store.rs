use async_trait::async_trait;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client as DynamoClient;
use std::sync::{Arc, RwLock};
use super::index_status::IndexingStatus;
use crate::error::{FoldDbError, FoldDbResult};

#[async_trait]
pub trait ProgressStore: Send + Sync {
    async fn save_status(&self, status: &IndexingStatus) -> FoldDbResult<()>;
    async fn load_status(&self) -> FoldDbResult<IndexingStatus>;
}

pub struct InMemoryProgressStore {
    status: Arc<RwLock<IndexingStatus>>,
}

impl InMemoryProgressStore {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(IndexingStatus::default())),
        }
    }
}

#[async_trait]
impl ProgressStore for InMemoryProgressStore {
    async fn save_status(&self, status: &IndexingStatus) -> FoldDbResult<()> {
        let mut guard = self.status.write().unwrap();
        *guard = status.clone();
        Ok(())
    }

    async fn load_status(&self) -> FoldDbResult<IndexingStatus> {
        let guard = self.status.read().unwrap();
        Ok(guard.clone())
    }
}

pub struct DynamoDbProgressStore {
    client: DynamoClient,
    table_name: String,
    pk: String,
}

use aws_sdk_dynamodb::types::{AttributeDefinition, KeySchemaElement, KeyType, ScalarAttributeType, BillingMode, TableStatus};

// ... (existing imports)

impl DynamoDbProgressStore {
    pub fn new(client: DynamoClient, table_name: String, pk: String) -> Self {
        Self {
            client,
            table_name,
            pk,
        }
    }

    async fn ensure_table_exists(&self) -> FoldDbResult<()> {
        // Check if table exists
        match self.client.describe_table().table_name(&self.table_name).send().await {
            Ok(resp) => {
                if let Some(table) = resp.table {
                    if let Some(status) = table.table_status {
                        log::info!("Table {} exists, status: {:?}", self.table_name, status);
                        if let Some(schema) = table.key_schema {
                            log::info!("Table {} key schema: {:?}", self.table_name, schema);
                        }
                        if matches!(status, TableStatus::Active) {
                            return Ok(());
                        } else {
                            // Wait for table to become active
                            log::info!("Table {} is not active, waiting...", self.table_name);
                        }
                    }
                } else {
                     log::warn!("Table {} exists but description is empty", self.table_name);
                }
                // If we are here, table exists but might not be active. 
                // We should probably wait, but for now let's fall through to wait logic below
                // actually, we should jump to wait logic.
            },
            Err(e) => {
                let error_str = e.to_string(); // Use to_string() for simple check
                // Check debug string too just in case
                let error_debug = format!("{:?}", e);
                
                if !error_str.contains("ResourceNotFoundException") && !error_debug.contains("ResourceNotFoundException") {
                    return Err(FoldDbError::Database(format!("Failed to check table existence: {}", error_str)));
                }
                // Table doesn't exist, proceed to create
                log::info!("Creating DynamoDB table: {}", self.table_name);
        
                // Create table
                let _ = self.client.create_table()
                    .table_name(&self.table_name)
                    .attribute_definitions(
                        AttributeDefinition::builder()
                            .attribute_name("PK")
                            .attribute_type(ScalarAttributeType::S)
                            .build()
                            .unwrap()
                    )
                    .attribute_definitions(
                        AttributeDefinition::builder()
                            .attribute_name("SK")
                            .attribute_type(ScalarAttributeType::S)
                            .build()
                            .unwrap()
                    )
                    .key_schema(
                        KeySchemaElement::builder()
                            .attribute_name("PK")
                            .key_type(KeyType::Hash)
                            .build()
                            .unwrap()
                    )
                    .key_schema(
                        KeySchemaElement::builder()
                            .attribute_name("SK")
                            .key_type(KeyType::Range)
                            .build()
                            .unwrap()
                    )
                    .billing_mode(BillingMode::PayPerRequest)
                    .send()
                    .await
                    .map_err(|e| FoldDbError::Database(format!("Failed to create table: {}", e)))?;
            }
        }

        // Wait for table to be active (shared logic for create and existing-but-not-active)
        let mut attempts = 0;
        while attempts < 30 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            match self.client.describe_table().table_name(&self.table_name).send().await {
                Ok(resp) => {
                    if let Some(table) = resp.table {
                        if let Some(status) = table.table_status {
                            if matches!(status, TableStatus::Active) {
                                log::info!("DynamoDB table {} is now ACTIVE", self.table_name);
                                return Ok(());
                            } else {
                                log::info!("Waiting for table {} to be ACTIVE, current status: {:?}", self.table_name, status);
                            }
                        }
                    }
                },
                Err(_) => {}
            }
            attempts += 1;
        }

        Err(FoldDbError::Database("Table creation/activation timed out".to_string()))
    }

    }


#[async_trait]
impl ProgressStore for DynamoDbProgressStore {
    async fn save_status(&self, status: &IndexingStatus) -> FoldDbResult<()> {
        let json = serde_json::to_string(status).map_err(|e| FoldDbError::Serialization(e.to_string()))?;
        
        let result = self.client.put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(self.pk.clone()))
            .item("SK", AttributeValue::S("indexing_status".to_string()))
            .item("StatusJson", AttributeValue::S(json.clone()))
            .item("UpdatedAt", AttributeValue::N(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string()))
            .send()
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                let error_str = format!("{:?}", e);
                
                if error_str.contains("ResourceNotFoundException") {
                    // Try to create table and retry
                    self.ensure_table_exists().await?;
                    
                    self.client.put_item()
                        .table_name(&self.table_name)
                        .item("PK", AttributeValue::S(self.pk.clone()))
                        .item("SK", AttributeValue::S("indexing_status".to_string()))
                        .item("StatusJson", AttributeValue::S(json))
                        .item("UpdatedAt", AttributeValue::N(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string()))
                        .send()
                        .await
                        .map_err(|e| FoldDbError::Database(format!("Failed to save status after table creation: {:?}", e)))?;
                    
                    Ok(())
                } else {
                    Err(FoldDbError::Database(format!("Failed to save status: {:?}", e)))
                }
            }
        }
    }

    async fn load_status(&self) -> FoldDbResult<IndexingStatus> {
        let result = self.client.get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(self.pk.clone()))
            .key("SK", AttributeValue::S("indexing_status".to_string()))
            .send()
            .await;
            
        match result {
            Ok(output) => {
                if let Some(item) = output.item {
                    if let Some(json_attr) = item.get("StatusJson") {
                        if let Ok(json) = json_attr.as_s() {
                            let status: IndexingStatus = serde_json::from_str(json)
                                .map_err(|e| FoldDbError::Serialization(e.to_string()))?;
                            return Ok(status);
                        }
                    }
                }
                Ok(IndexingStatus::default())
            },
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("ResourceNotFoundException") {
                    // Table doesn't exist, return default status (it will be created on first save)
                    Ok(IndexingStatus::default())
                } else {
                    Err(FoldDbError::Database(format!("Failed to load status: {:?}", e)))
                }
            }
        }
    }
}
