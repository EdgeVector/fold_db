//! Progress tracking for ingestion operations
//!
//! Adapts the unified progress tracking (JobTracker) for ingestion workflows.

use crate::progress::{Job, JobStatus, JobType};
use crate::schema::types::KeyValue;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// Re-export ProgressTracker and create_tracker for backward compatibility
pub use crate::progress::{
    create_tracker as create_progress_tracker, ProgressTracker,
    ProgressTracker as IngestionProgressStore,
};

/// Steps in the ingestion process
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub enum IngestionStep {
    ValidatingConfig,
    FlatteningData,
    GettingAIRecommendation,
    SettingUpSchema,
    GeneratingMutations,
    ExecutingMutations,
    Completed,
    Failed,
}

/// A single schema and the keys that were written to it during ingestion.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SchemaWriteRecord {
    pub schema_name: String,
    pub keys_written: Vec<KeyValue>,
}

/// Results of completed ingestion operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IngestionResults {
    pub schema_name: String,
    pub new_schema_created: bool,
    pub mutations_generated: usize,
    pub mutations_executed: usize,
    /// All schemas and keys written during this ingestion (covers decomposition).
    pub schemas_written: Vec<SchemaWriteRecord>,
}

/// Helper struct to map generic Job to IngestionProgress shape for API compatibility
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IngestionProgress {
    pub id: String,
    /// Job type: "ingestion", "indexing", or custom
    pub job_type: String,
    pub current_step: IngestionStep,
    pub progress_percentage: u8,
    pub status_message: String,
    pub is_complete: bool,
    pub is_failed: bool,
    pub error_message: Option<String>,
    pub results: Option<IngestionResults>,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}

impl From<Job> for IngestionProgress {
    fn from(job: Job) -> Self {
        let current_step: IngestionStep = if let Some(step_val) = job.metadata.get("step") {
            serde_json::from_value(step_val.clone()).unwrap_or(IngestionStep::ValidatingConfig)
        } else {
            match job.status {
                JobStatus::Completed => IngestionStep::Completed,
                JobStatus::Failed => IngestionStep::Failed,
                _ => IngestionStep::ValidatingConfig,
            }
        };

        let job_type = match &job.job_type {
            JobType::Ingestion => "ingestion".to_string(),
            JobType::Indexing => "indexing".to_string(),
            JobType::Other(s) => s.clone(),
        };

        IngestionProgress {
            id: job.id,
            job_type,
            current_step,
            progress_percentage: job.progress_percentage,
            status_message: job.message,
            is_complete: matches!(job.status, JobStatus::Completed | JobStatus::Failed),
            is_failed: matches!(job.status, JobStatus::Failed),
            error_message: job.error,
            results: job.result.and_then(|r| serde_json::from_value(r).ok()),
            started_at: job.created_at,
            completed_at: job.completed_at,
        }
    }
}

/// Progress tracking service wrapper
#[derive(Clone)]
pub struct ProgressService {
    tracker: ProgressTracker,
}

impl ProgressService {
    pub fn new(tracker: ProgressTracker) -> Self {
        Self { tracker }
    }

    pub async fn start_progress(&self, id: String, user_id: String) -> IngestionProgress {
        let mut job = Job::new(id, JobType::Ingestion).with_user(user_id);
        job.update_progress(5, "Starting ingestion process...".to_string());
        Self::set_job_step(&mut job, &IngestionStep::ValidatingConfig);
        self.save_job(&job).await;
        job.into()
    }

    pub async fn update_progress(
        &self,
        id: &str,
        step: IngestionStep,
        message: String,
    ) -> Option<IngestionProgress> {
        let pct = Self::step_to_percentage(&step);
        self.update_progress_with_percentage(id, step, message, pct).await
    }

    pub async fn update_progress_with_percentage(
        &self,
        id: &str,
        step: IngestionStep,
        message: String,
        percentage: u8,
    ) -> Option<IngestionProgress> {
        let Ok(Some(mut job)) = self.tracker.load(id).await else { return None };
        job.update_progress(percentage, message);
        Self::set_job_step(&mut job, &step);
        self.save_job(&job).await;
        Some(job.into())
    }

    pub async fn complete_progress(
        &self,
        id: &str,
        results: IngestionResults,
    ) -> Option<IngestionProgress> {
        let Ok(Some(mut job)) = self.tracker.load(id).await else { return None };
        job.complete(serde_json::to_value(results).ok());
        Self::set_job_step(&mut job, &IngestionStep::Completed);
        self.save_job(&job).await;
        Some(job.into())
    }

    pub async fn fail_progress(
        &self,
        id: &str,
        error_message: String,
    ) -> Option<IngestionProgress> {
        let Ok(Some(mut job)) = self.tracker.load(id).await else { return None };
        job.fail(error_message);
        Self::set_job_step(&mut job, &IngestionStep::Failed);
        self.save_job(&job).await;
        Some(job.into())
    }

    pub async fn get_progress(&self, id: &str) -> Option<IngestionProgress> {
        self.tracker.load(id).await.unwrap_or(None).map(|j| j.into())
    }

    pub async fn get_all_progress(&self) -> Vec<IngestionProgress> {
        let Some(user_id) = crate::logging::core::get_current_user_id() else {
            return vec![]; // No user context = no jobs to return
        };

        self.tracker
            .list_by_user(&user_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|j| {
                matches!(j.job_type, JobType::Ingestion | JobType::Indexing)
                    || matches!(&j.job_type, JobType::Other(s) if s == "database_reset")
            })
            .map(|j| j.into())
            .collect()
    }

    fn set_job_step(job: &mut Job, step: &IngestionStep) {
        let step_json = serde_json::to_value(step).unwrap_or_default();
        match job.metadata {
            serde_json::Value::Object(ref mut map) => {
                map.insert("step".to_string(), step_json);
            }
            _ => job.metadata = serde_json::json!({ "step": step_json }),
        }
    }

    async fn save_job(&self, job: &Job) {
        if let Err(e) = self.tracker.save(job).await {
            log::warn!("Failed to save progress: {}", e);
        }
    }

    fn step_to_percentage(step: &IngestionStep) -> u8 {
        match step {
            IngestionStep::ValidatingConfig => 5,
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
