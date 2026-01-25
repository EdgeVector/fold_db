//! Progress tracking for ingestion operations
//!
//! Adapts the unified progress tracking (JobTracker) for ingestion workflows.

use crate::progress::{Job, JobStatus, JobType};
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
    PreparingSchemas,
    FlatteningData,
    GettingAIRecommendation,
    SettingUpSchema,
    GeneratingMutations,
    ExecutingMutations,
    Completed,
    Failed,
}

/// Results of completed ingestion operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IngestionResults {
    pub schema_name: String,
    pub new_schema_created: bool,
    pub mutations_generated: usize,
    pub mutations_executed: usize,
}

/// Helper struct to map generic Job to IngestionProgress shape for API compatibility
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IngestionProgress {
    pub id: String,
    /// Job type: "ingestion", "indexing", "backfill", or custom
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
        // Parse current step from metadata if possible, otherwise derive from status
        let current_step: IngestionStep = if let Some(step_val) = job.metadata.get("step") {
            serde_json::from_value(step_val.clone()).unwrap_or(IngestionStep::ValidatingConfig)
        } else {
            match job.status {
                JobStatus::Completed => IngestionStep::Completed,
                JobStatus::Failed => IngestionStep::Failed,
                _ => IngestionStep::ValidatingConfig,
            }
        };

        // Convert JobType to string for API response
        let job_type_str = match &job.job_type {
            JobType::Ingestion => "ingestion".to_string(),
            JobType::Indexing => "indexing".to_string(),
            JobType::Backfill => "backfill".to_string(),
            JobType::Other(s) => s.clone(),
        };

        IngestionProgress {
            id: job.id,
            job_type: job_type_str,
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
        let mut job = Job::new(id, JobType::Ingestion);

        job = job.with_user(user_id);

        // Initial metadata
        job.metadata = serde_json::json!({
            "step": IngestionStep::ValidatingConfig
        });
        job.progress_percentage = 5;
        job.message = "Starting ingestion process...".to_string();

        let _ = self.tracker.save(&job).await;
        job.into()
    }

    pub async fn update_progress(
        &self,
        id: &str,
        step: IngestionStep,
        message: String,
    ) -> Option<IngestionProgress> {
        if let Ok(Some(mut job)) = self.tracker.load(id).await {
            job.update_progress(Self::step_to_percentage(&step), message);

            // Update metadata with step
            if let Ok(step_json) = serde_json::to_value(&step) {
                if let serde_json::Value::Object(ref mut map) = job.metadata {
                    map.insert("step".to_string(), step_json);
                } else {
                    job.metadata = serde_json::json!({ "step": step_json });
                }
            }

            let _ = self.tracker.save(&job).await;
            Some(job.into())
        } else {
            None
        }
    }

    pub async fn update_progress_with_percentage(
        &self,
        id: &str,
        step: IngestionStep,
        message: String,
        percentage: u8,
    ) -> Option<IngestionProgress> {
        if let Ok(Some(mut job)) = self.tracker.load(id).await {
            job.update_progress(percentage, message);

            // Update metadata with step
            if let Ok(step_json) = serde_json::to_value(&step) {
                if let serde_json::Value::Object(ref mut map) = job.metadata {
                    map.insert("step".to_string(), step_json);
                } else {
                    job.metadata = serde_json::json!({ "step": step_json });
                }
            }

            let _ = self.tracker.save(&job).await;
            Some(job.into())
        } else {
            None
        }
    }

    pub async fn complete_progress(
        &self,
        id: &str,
        results: IngestionResults,
    ) -> Option<IngestionProgress> {
        if let Ok(Some(mut job)) = self.tracker.load(id).await {
            let result_json = serde_json::to_value(results).ok();
            job.complete(result_json);

            // Update metadata with step
            let step = IngestionStep::Completed;
            if let Ok(step_json) = serde_json::to_value(&step) {
                if let serde_json::Value::Object(ref mut map) = job.metadata {
                    map.insert("step".to_string(), step_json);
                } else {
                    job.metadata = serde_json::json!({ "step": step_json });
                }
            }

            let _ = self.tracker.save(&job).await;
            Some(job.into())
        } else {
            None
        }
    }

    pub async fn fail_progress(
        &self,
        id: &str,
        error_message: String,
    ) -> Option<IngestionProgress> {
        if let Ok(Some(mut job)) = self.tracker.load(id).await {
            job.fail(error_message);

            // Update metadata with step
            let step = IngestionStep::Failed;
            if let Ok(step_json) = serde_json::to_value(&step) {
                if let serde_json::Value::Object(ref mut map) = job.metadata {
                    map.insert("step".to_string(), step_json);
                } else {
                    job.metadata = serde_json::json!({ "step": step_json });
                }
            }

            let _ = self.tracker.save(&job).await;
            Some(job.into())
        } else {
            None
        }
    }

    pub async fn get_progress(&self, id: &str) -> Option<IngestionProgress> {
        self.tracker
            .load(id)
            .await
            .unwrap_or(None)
            .map(|j| j.into())
    }

    pub async fn remove_progress(&self, id: &str) -> Option<IngestionProgress> {
        if let Ok(Some(job)) = self.tracker.load(id).await {
            let _ = self.tracker.delete(id).await;
            Some(job.into())
        } else {
            None
        }
    }

    pub async fn get_all_progress(&self) -> Vec<IngestionProgress> {
        // Require user context - no default fallback
        let user_id = match crate::logging::core::get_current_user_id() {
            Some(uid) => uid,
            None => return vec![], // No user context = no jobs to return
        };

        self.tracker
            .list_by_user(&user_id)
            .await
            .unwrap_or_default()
            .into_iter()
            // Include both Ingestion and Indexing jobs
            .filter(|j| matches!(j.job_type, JobType::Ingestion | JobType::Indexing))
            .map(|j| j.into())
            .collect()
    }

    fn step_to_percentage(step: &IngestionStep) -> u8 {
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
