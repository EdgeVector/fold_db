//! Generic progress tracking system for DataFold
//!
//! Provides a unified way to track long-running jobs (Ingestion, Backfill, etc.) 
//! with pluggable persistence (InMemory, DynamoDB).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use utoipa::ToSchema;
#[cfg(feature = "aws-backend")]
use aws_sdk_dynamodb::types::AttributeValue;
#[cfg(feature = "aws-backend")]
use std::time::SystemTime;

/// Type of job being tracked
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub enum JobType {
    Ingestion,
    Backfill,
    Indexing,
    Other(String),
}

/// Current status of the job
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Generic job structure
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Job {
    /// Unique identifier for the job
    pub id: String,
    /// Type of the job
    pub job_type: JobType,
    /// Current status
    pub status: JobStatus,
    /// Progress percentage (0-100)
    pub progress_percentage: u8,
    /// User-facing status message
    pub message: String,
    
    /// User ID who owns this job
    #[serde(default)]
    pub user_id: Option<String>,

    /// Metadata specific to the job type (stored as JSON)
    pub metadata: serde_json::Value,
    
    /// Results when completed (stored as JSON)
    pub result: Option<serde_json::Value>,
    
    /// Timestamp when created (Unix seconds)
    pub created_at: u64,
    /// Timestamp when last updated (Unix seconds)
    pub updated_at: u64,
    /// Timestamp when completed (Unix seconds)
    pub completed_at: Option<u64>,
    /// Error message if failed
    pub error: Option<String>,
}

impl Job {
    pub fn new(id: String, job_type: JobType) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        Self {
            id,
            job_type,
            status: JobStatus::Queued,
            progress_percentage: 0,
            message: "Initialized".to_string(),
            user_id: None,
            metadata: serde_json::Value::Null,
            result: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            error: None,
        }
    }
    
    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn update_progress(&mut self, percentage: u8, message: String) {
        self.progress_percentage = percentage.min(100);
        self.message = message;
        self.status = JobStatus::Running;
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    pub fn complete(&mut self, result: Option<serde_json::Value>) {
        self.status = JobStatus::Completed;
        self.progress_percentage = 100;
        self.message = "Completed".to_string();
        self.result = result;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.updated_at = now;
        self.completed_at = Some(now);
    }

    pub fn fail(&mut self, error: String) {
        self.status = JobStatus::Failed;
        self.error = Some(error.clone());
        self.message = format!("Failed: {}", error);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.updated_at = now;
        self.completed_at = Some(now);
    }
}

/// Abstract storage for job progress
#[async_trait]
pub trait ProgressStore: Send + Sync {
    async fn save(&self, job: &Job) -> Result<(), String>;
    async fn load(&self, id: &str) -> Result<Option<Job>, String>;
    async fn list_by_user(&self, user_id: &str) -> Result<Vec<Job>, String>;
    async fn delete(&self, id: &str) -> Result<(), String>;
}

/// In-memory implementation (for testing/single-tenant)
pub struct InMemoryProgressStore {
    store: Mutex<HashMap<String, Job>>,
}

impl Default for InMemoryProgressStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryProgressStore {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ProgressStore for InMemoryProgressStore {
    async fn save(&self, job: &Job) -> Result<(), String> {
        let mut store = self.store.lock().unwrap();
        store.insert(job.id.clone(), job.clone());
        Ok(())
    }

    async fn load(&self, id: &str) -> Result<Option<Job>, String> {
        let store = self.store.lock().unwrap();
        Ok(store.get(id).cloned())
    }

    async fn list_by_user(&self, user_id: &str) -> Result<Vec<Job>, String> {
        let store = self.store.lock().unwrap();
        Ok(store.values()
            .filter(|j| j.user_id.as_deref() == Some(user_id) || j.user_id.is_none())
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &str) -> Result<(), String> {
        let mut store = self.store.lock().unwrap();
        store.remove(id);
        Ok(())
    }
}

/// DynamoDB implementation
#[cfg(feature = "aws-backend")]
pub struct DynamoDbProgressStore {
    client: aws_sdk_dynamodb::Client,
    table_name: String,
}

#[cfg(feature = "aws-backend")]
impl DynamoDbProgressStore {
    pub fn new(client: aws_sdk_dynamodb::Client, table_name: String) -> Self {
        Self { client, table_name }
    }
    
    // Legacy constructor for backward compatibility or ease of use (optional)
    pub async fn from_config(table_name: String, region: String) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_sdk_dynamodb::config::Region::new(region))
            .load()
            .await;
        let client = aws_sdk_dynamodb::Client::new(&config);
        Self { client, table_name }
    }
    
    // Additional helpers for DynamoDB could be added here (e.g. ensure_table_exists)
    
    fn item_to_job(&self, item: &HashMap<String, AttributeValue>) -> Option<Job> {
        let json = item.get("data")?.as_s().ok()?;
        serde_json::from_str(json).ok()
    }
}

#[cfg(feature = "aws-backend")]
#[async_trait]
impl ProgressStore for DynamoDbProgressStore {
    async fn save(&self, job: &Job) -> Result<(), String> {
        // We use user_id as partition key, and job_id as sort key
        // If user_id is missing, we use "global" or similar
        let pk = job.user_id.clone().unwrap_or_else(|| "global".to_string());
        
        let json = serde_json::to_string(job).map_err(|e| e.to_string())?;

        // TTL: 24 hours
        let ttl = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + (24 * 60 * 60)) as i64;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(pk))
            .item("SK", AttributeValue::S(job.id.clone()))
            .item("data", AttributeValue::S(json))
            .item("ttl", AttributeValue::N(ttl.to_string()))
             // Indexed fields for filtering could be added here
            .send()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    async fn load(&self, id: &str) -> Result<Option<Job>, String> {
        // This is tricky because we need the PK (user_id) to look up by SK (id).
        // If we don't know the User ID, we might need a GSI or to Query.
        // For strict multi-tenancy we SHOULD know the user_id.
        // However, the interface `load(id)` implies global uniqueness lookup.
        
        // If we assume the ID is unique enough, we might need a GSI on SK?
        // OR we change the interface to `load(id, user_id)`.
        
        // FOR NOW: We will assume we can't easily implement efficient global load without GSI.
        // We will fallback to a Scan if really needed, OR we rely on the caller knowing the context?
        // Actually, let's keep it simple: WE require user_id for scalable lookups.
        
        // But the trait is `load(id)`.
        // Let's rely on a convention: if we are in a context where we know the user, we should use `list_by_user` and filter.
        // Or we implement a GSI lookup.
        
        // Given existing Lambda/Ingestion code often just passes ID...
        // The previous implementation used user_id from "current_user()" helper.
        // That WAS context aware.
        
        // We can replicate that pattern:
        let user_id = crate::logging::core::get_current_user_id().unwrap_or_else(|| "default".to_string());
        
        let result = self.client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(user_id))
            .key("SK", AttributeValue::S(id.to_string()))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if let Some(item) = result.item {
            Ok(self.item_to_job(&item))
        } else {
            Ok(None)
        }
    }

    async fn list_by_user(&self, user_id: &str) -> Result<Vec<Job>, String> {
        let result = self.client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :uid")
            .expression_attribute_values(":uid", AttributeValue::S(user_id.to_string()))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let items = result.items.unwrap_or_default();
        Ok(items
            .iter()
            .filter_map(|i| self.item_to_job(i))
            .collect())
    }

    async fn delete(&self, id: &str) -> Result<(), String> {
         let user_id = crate::logging::core::get_current_user_id().unwrap_or_else(|| "default".to_string());
         
         self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(user_id))
            .key("SK", AttributeValue::S(id.to_string()))
            .send()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

pub type ProgressTracker = Arc<dyn ProgressStore>;

pub async fn create_tracker(dynamo_config: Option<(String, String)>) -> ProgressTracker {
    if let Some((table_name, region)) = dynamo_config {
        #[cfg(feature = "aws-backend")]
        {
            let store = DynamoDbProgressStore::from_config(table_name.clone(), region).await;
            Arc::new(store)
        }
        #[cfg(not(feature = "aws-backend"))]
        {
            let _ = (table_name, region);
            Arc::new(InMemoryProgressStore::new())
        }
    } else {
        Arc::new(InMemoryProgressStore::new())
    }
}
