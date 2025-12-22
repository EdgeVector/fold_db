//! Progress tracking for ingestion operations
//!
//! Now supports pluggable backends (InMemory, DynamoDB) via ProgressStore trait.

use crate::logging::core::get_current_user_id;
use async_trait::async_trait;
#[cfg(feature = "aws-backend")]
use aws_sdk_dynamodb::types::{
    AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ScalarAttributeType,
};
#[cfg(feature = "aws-backend")]
use aws_sdk_dynamodb::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use utoipa::ToSchema;

/// Progress tracking for ingestion operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IngestionProgress {
    /// Unique identifier for this ingestion operation
    pub id: String,
    /// Current step in the ingestion process
    pub current_step: IngestionStep,
    /// Overall progress percentage (0-100)
    pub progress_percentage: u8,
    /// Status message describing current operation
    pub status_message: String,
    /// Whether the operation is complete
    pub is_complete: bool,
    /// Whether the operation failed
    pub is_failed: bool,
    /// Error message if operation failed
    pub error_message: Option<String>,
    /// Timestamp when operation started
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp when operation completed (if applicable)
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Results of the ingestion operation
    pub results: Option<IngestionResults>,
}

/// Steps in the ingestion process
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub enum IngestionStep {
    /// Validating configuration
    ValidatingConfig,
    /// Preparing schemas
    PreparingSchemas,
    /// Flattening data structure
    FlatteningData,
    /// Getting AI recommendation
    GettingAIRecommendation,
    /// Setting up schema
    SettingUpSchema,
    /// Generating mutations
    GeneratingMutations,
    /// Executing mutations
    ExecutingMutations,
    /// Completed
    Completed,
    /// Failed
    Failed,
}

/// Results of completed ingestion operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IngestionResults {
    /// Schema name used
    pub schema_name: String,
    /// Whether a new schema was created
    pub new_schema_created: bool,
    /// Number of mutations generated
    pub mutations_generated: usize,
    /// Number of mutations executed
    pub mutations_executed: usize,
}

impl IngestionProgress {
    /// Create a new progress tracker
    pub fn new(id: String) -> Self {
        Self {
            id,
            current_step: IngestionStep::ValidatingConfig,
            progress_percentage: 5, // Start at 5% for ValidatingConfig step
            status_message: "Starting ingestion process...".to_string(),
            is_complete: false,
            is_failed: false,
            error_message: None,
            started_at: chrono::Utc::now(),
            completed_at: None,
            results: None,
        }
    }

    /// Update progress to a specific step
    pub fn update_step(&mut self, step: IngestionStep, message: String) {
        self.current_step = step.clone();
        self.status_message = message;
        self.progress_percentage = self.step_to_percentage(&step);
    }

    /// Update progress with a custom percentage (for granular progress within a step)
    pub fn update_step_with_percentage(
        &mut self,
        step: IngestionStep,
        message: String,
        percentage: u8,
    ) {
        self.current_step = step;
        self.status_message = message;
        self.progress_percentage = percentage.min(100);
    }

    /// Mark as completed with results
    pub fn mark_completed(&mut self, results: IngestionResults) {
        self.is_complete = true;
        self.current_step = IngestionStep::Completed;
        self.progress_percentage = 100;
        self.status_message = "Ingestion completed successfully".to_string();
        self.completed_at = Some(chrono::Utc::now());
        self.results = Some(results);
    }

    /// Mark as failed with error message
    pub fn mark_failed(&mut self, error_message: String) {
        self.is_failed = true;
        self.is_complete = true;
        self.current_step = IngestionStep::Failed;
        self.status_message = "Ingestion failed".to_string();
        self.error_message = Some(error_message);
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Convert step to percentage
    fn step_to_percentage(&self, step: &IngestionStep) -> u8 {
        match step {
            IngestionStep::ValidatingConfig => 5,
            IngestionStep::PreparingSchemas => 15,
            IngestionStep::FlatteningData => 25,
            IngestionStep::GettingAIRecommendation => 40,
            IngestionStep::SettingUpSchema => 55,
            IngestionStep::GeneratingMutations => 75,
            IngestionStep::ExecutingMutations => 90,
            IngestionStep::Completed => 100,
            IngestionStep::Failed => 100,
        }
    }
}

/// Helper to get current user ID
fn current_user() -> String {
    get_current_user_id().unwrap_or_else(|| "default".to_string())
}

/// Abstract storage for ingestion progress
#[async_trait]
pub trait ProgressStore: Send + Sync {
    async fn save(&self, progress: &IngestionProgress) -> Result<(), String>;
    async fn load(&self, id: &str) -> Result<Option<IngestionProgress>, String>;
    async fn list(&self) -> Result<Vec<IngestionProgress>, String>;
    async fn delete(&self, id: &str) -> Result<(), String>;
}

/// In-memory implementation (for testing/single-tenant)
pub struct InMemoryProgressStore {
    store: Mutex<HashMap<String, IngestionProgress>>,
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
    async fn save(&self, progress: &IngestionProgress) -> Result<(), String> {
        let mut store = self.store.lock().unwrap();
        store.insert(progress.id.clone(), progress.clone());
        Ok(())
    }

    async fn load(&self, id: &str) -> Result<Option<IngestionProgress>, String> {
        let store = self.store.lock().unwrap();
        Ok(store.get(id).cloned())
    }

    async fn list(&self) -> Result<Vec<IngestionProgress>, String> {
        let store = self.store.lock().unwrap();
        Ok(store.values().cloned().collect())
    }

    async fn delete(&self, id: &str) -> Result<(), String> {
        let mut store = self.store.lock().unwrap();
        store.remove(id);
        Ok(())
    }
}

/// DynamoDB implementation (for multi-tenant)
#[cfg(feature = "aws-backend")]
pub struct DynamoDbProgressStore {
    client: Client,
    table_name: String,
}

#[cfg(feature = "aws-backend")]
impl DynamoDbProgressStore {
    pub async fn new(table_name: String) -> Result<Self, String> {
        let config = aws_config::load_from_env().await;
        let client = Client::new(&config);
        let store = Self { client, table_name };
        store.ensure_table_exists().await?;
        Ok(store)
    }

    pub fn with_client(table_name: String, client: Client) -> Self {
        Self { client, table_name }
    }

    async fn ensure_table_exists(&self) -> Result<(), String> {
        use aws_sdk_dynamodb::types::BillingMode;

        let result = self
            .client
            .create_table()
            .table_name(&self.table_name)
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("PK")
                    .attribute_type(ScalarAttributeType::S) // Partition Key
                    .build()
                    .unwrap(),
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("SK")
                    .attribute_type(ScalarAttributeType::S) // Sort Key
                    .build()
                    .unwrap(),
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("PK")
                    .key_type(KeyType::Hash)
                    .build()
                    .unwrap(),
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("SK")
                    .key_type(KeyType::Range)
                    .build()
                    .unwrap(),
            )
            .billing_mode(BillingMode::PayPerRequest)
            .send()
            .await;

        match result {
            Ok(_) => {
                // Wait for table to be active
                self.wait_for_table_active().await
            }
            Err(err) => {
                if let Some(service_err) = err.as_service_error() {
                    if service_err.is_resource_in_use_exception() {
                        // Table exists or is being created, wait for it to be active
                        return self.wait_for_table_active().await;
                    }
                }
                Err(format!("Failed to create table: {}", err))
            }
        }
    }

    async fn wait_for_table_active(&self) -> Result<(), String> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(60);

        loop {
            if start.elapsed() > timeout {
                return Err("Timeout waiting for table to become active".to_string());
            }

            let describe = self
                .client
                .describe_table()
                .table_name(&self.table_name)
                .send()
                .await
                .map_err(|e| format!("Failed to describe table: {}", e))?;

            if let Some(table) = describe.table {
                if let Some(status) = table.table_status {
                    if status == aws_sdk_dynamodb::types::TableStatus::Active {
                        return Ok(());
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    fn item_to_progress(
        &self,
        item: &HashMap<String, AttributeValue>,
    ) -> Option<IngestionProgress> {
        let json = item.get("data")?.as_s().ok()?;
        serde_json::from_str(json).ok()
    }
}

#[async_trait]
#[cfg(feature = "aws-backend")]
impl ProgressStore for DynamoDbProgressStore {
    async fn save(&self, progress: &IngestionProgress) -> Result<(), String> {
        let user_id = current_user();
        let json = serde_json::to_string(progress).map_err(|e| e.to_string())?;

        // TTL: 24 hours
        let ttl = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + (24 * 60 * 60)) as i64;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(user_id))
            .item("SK", AttributeValue::S(progress.id.clone()))
            .item("data", AttributeValue::S(json))
            .item("ttl", AttributeValue::N(ttl.to_string()))
            .send()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    async fn load(&self, id: &str) -> Result<Option<IngestionProgress>, String> {
        let user_id = current_user();

        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(user_id))
            .key("SK", AttributeValue::S(id.to_string()))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if let Some(item) = result.item {
            Ok(self.item_to_progress(&item))
        } else {
            Ok(None)
        }
    }

    async fn list(&self) -> Result<Vec<IngestionProgress>, String> {
        let user_id = current_user();

        let result = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :uid")
            .expression_attribute_values(":uid", AttributeValue::S(user_id))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let items = result.items.unwrap_or_default();
        Ok(items
            .iter()
            .filter_map(|i| self.item_to_progress(i))
            .collect())
    }

    async fn delete(&self, id: &str) -> Result<(), String> {
        let user_id = current_user();

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

/// Global progress tracker
pub type ProgressTracker = Arc<dyn ProgressStore>;

/// Create a new progress tracker (in-memory or DynamoDB based on env or config)
pub async fn create_progress_tracker(dynamo_config: Option<(String, String)>) -> ProgressTracker {
    use std::env;

    // Check for provided config first, then env vars
    let config = if let Some((table, region)) = dynamo_config {
        Some((table, region))
    } else {
        // Fallback to env vars
        match env::var("DATAFOLD_INGESTION_PROGRESS_TABLE")
            .or_else(|_| env::var("DATAFOLD_DYNAMODB_TABLE"))
        {
            Ok(table) => {
                let region = env::var("DATAFOLD_DYNAMODB_REGION")
                    .unwrap_or_else(|_| "us-west-2".to_string());
                Some((table, region))
            }
            Err(_) => None,
        }
    };

    if let Some((table_name, region)) = config {
        #[cfg(feature = "aws-backend")]
        {
            log::info!(
                "Initializing DynamoDB Progress Tracker: table={}, region={}",
                table_name,
                region
            );

            // Initialize DynamoDB client with correct region
            let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_sdk_dynamodb::config::Region::new(region))
                .load()
                .await;
            let client = Client::new(&config);

            let store = DynamoDbProgressStore::with_client(table_name, client);
            // Ensure table exists
            if let Err(e) = store.ensure_table_exists().await {
                log::error!("Failed to ensure DynamoDB process table exists: {}", e);
                // If we are explicitly configured we should probably fail?
                // But this function signature returns ProgressTracker, not Result.
                // For now, let's panic to match "no graceful fallback" if configured.
                panic!("Failed to ensure DynamoDB process table exists: {}", e);
            }
            Arc::new(store)
        }
        #[cfg(not(feature = "aws-backend"))]
        {
            log::warn!("DynamoDB progress table configured but aws-backend feature is disabled. Falling back to in-memory.");
            Arc::new(InMemoryProgressStore::new())
        }
    } else {
        log::info!("Initializing In-Memory Progress Tracker (default)");
        Arc::new(InMemoryProgressStore::new())
    }
}

/// Progress tracking service
#[derive(Clone)]
pub struct ProgressService {
    tracker: ProgressTracker,
}

impl ProgressService {
    /// Create a new progress service
    pub fn new(tracker: ProgressTracker) -> Self {
        Self { tracker }
    }

    /// Start tracking progress for an ingestion operation
    pub async fn start_progress(&self, id: String) -> IngestionProgress {
        let progress = IngestionProgress::new(id.clone());
        let _ = self.tracker.save(&progress).await;
        progress
    }

    /// Update progress for an operation
    pub async fn update_progress(
        &self,
        id: &str,
        step: IngestionStep,
        message: String,
    ) -> Option<IngestionProgress> {
        if let Ok(Some(mut progress)) = self.tracker.load(id).await {
            progress.update_step(step, message);
            let _ = self.tracker.save(&progress).await;
            Some(progress)
        } else {
            None
        }
    }

    /// Update progress with custom percentage
    pub async fn update_progress_with_percentage(
        &self,
        id: &str,
        step: IngestionStep,
        message: String,
        percentage: u8,
    ) -> Option<IngestionProgress> {
        if let Ok(Some(mut progress)) = self.tracker.load(id).await {
            progress.update_step_with_percentage(step, message, percentage);
            let _ = self.tracker.save(&progress).await;
            Some(progress)
        } else {
            None
        }
    }

    /// Mark progress as completed
    pub async fn complete_progress(
        &self,
        id: &str,
        results: IngestionResults,
    ) -> Option<IngestionProgress> {
        if let Ok(Some(mut progress)) = self.tracker.load(id).await {
            progress.mark_completed(results);
            let _ = self.tracker.save(&progress).await;
            Some(progress)
        } else {
            None
        }
    }

    /// Mark progress as failed
    pub async fn fail_progress(
        &self,
        id: &str,
        error_message: String,
    ) -> Option<IngestionProgress> {
        if let Ok(Some(mut progress)) = self.tracker.load(id).await {
            progress.mark_failed(error_message);
            let _ = self.tracker.save(&progress).await;
            Some(progress)
        } else {
            None
        }
    }

    /// Get current progress for an operation
    pub async fn get_progress(&self, id: &str) -> Option<IngestionProgress> {
        self.tracker.load(id).await.unwrap_or(None)
    }

    /// Remove completed progress
    pub async fn remove_progress(&self, id: &str) -> Option<IngestionProgress> {
        if let Ok(Some(progress)) = self.tracker.load(id).await {
            let _ = self.tracker.delete(id).await;
            Some(progress)
        } else {
            None
        }
    }

    /// Get all active progress operations
    pub async fn get_all_progress(&self) -> Vec<IngestionProgress> {
        self.tracker.list().await.unwrap_or_default()
    }
}
