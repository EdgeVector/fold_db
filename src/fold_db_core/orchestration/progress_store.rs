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

impl DynamoDbProgressStore {
    pub fn new(client: DynamoClient, table_name: String, pk: String) -> Self {
        Self {
            client,
            table_name,
            pk,
        }
    }
}

#[async_trait]
impl ProgressStore for DynamoDbProgressStore {
    async fn save_status(&self, status: &IndexingStatus) -> FoldDbResult<()> {
        let json = serde_json::to_string(status).map_err(|e| FoldDbError::Serialization(e.to_string()))?;
        
        self.client.put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(self.pk.clone()))
            .item("SK", AttributeValue::S("indexing_status".to_string()))
            .item("StatusJson", AttributeValue::S(json))
            .item("UpdatedAt", AttributeValue::N(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string()))
            .send()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
            
        Ok(())
    }

    async fn load_status(&self) -> FoldDbResult<IndexingStatus> {
        let result = self.client.get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(self.pk.clone()))
            .key("SK", AttributeValue::S("indexing_status".to_string()))
            .send()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
            
        if let Some(item) = result.item {
            if let Some(json_attr) = item.get("StatusJson") {
                if let Ok(json) = json_attr.as_s() {
                    let status: IndexingStatus = serde_json::from_str(json)
                        .map_err(|e| FoldDbError::Serialization(e.to_string()))?;
                    return Ok(status);
                }
            }
        }
        
        Ok(IndexingStatus::default())
    }
}
