//! Progress tracking for ingestion operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
            progress_percentage: 5,  // Start at 5% for ValidatingConfig step
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
    pub fn update_step_with_percentage(&mut self, step: IngestionStep, message: String, percentage: u8) {
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

/// Global progress tracker
pub type ProgressTracker = Arc<Mutex<HashMap<String, IngestionProgress>>>;

/// Create a new progress tracker
pub fn create_progress_tracker() -> ProgressTracker {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Progress tracking service
pub struct ProgressService {
    tracker: ProgressTracker,
}

impl ProgressService {
    /// Create a new progress service
    pub fn new(tracker: ProgressTracker) -> Self {
        Self { tracker }
    }

    /// Start tracking progress for an ingestion operation
    pub fn start_progress(&self, id: String) -> IngestionProgress {
        let progress = IngestionProgress::new(id.clone());
        let mut tracker = self.tracker.lock().unwrap();
        tracker.insert(id, progress.clone());
        progress
    }

    /// Update progress for an operation
    pub fn update_progress(&self, id: &str, step: IngestionStep, message: String) -> Option<IngestionProgress> {
        let mut tracker = self.tracker.lock().unwrap();
        if let Some(progress) = tracker.get_mut(id) {
            progress.update_step(step, message);
            Some(progress.clone())
        } else {
            None
        }
    }

    /// Update progress with custom percentage (for granular progress within a step)
    pub fn update_progress_with_percentage(&self, id: &str, step: IngestionStep, message: String, percentage: u8) -> Option<IngestionProgress> {
        let mut tracker = self.tracker.lock().unwrap();
        if let Some(progress) = tracker.get_mut(id) {
            progress.update_step_with_percentage(step, message, percentage);
            Some(progress.clone())
        } else {
            None
        }
    }

    /// Mark progress as completed
    pub fn complete_progress(&self, id: &str, results: IngestionResults) -> Option<IngestionProgress> {
        let mut tracker = self.tracker.lock().unwrap();
        if let Some(progress) = tracker.get_mut(id) {
            progress.mark_completed(results);
            Some(progress.clone())
        } else {
            None
        }
    }

    /// Mark progress as failed
    pub fn fail_progress(&self, id: &str, error_message: String) -> Option<IngestionProgress> {
        let mut tracker = self.tracker.lock().unwrap();
        if let Some(progress) = tracker.get_mut(id) {
            progress.mark_failed(error_message);
            Some(progress.clone())
        } else {
            None
        }
    }

    /// Get current progress for an operation
    pub fn get_progress(&self, id: &str) -> Option<IngestionProgress> {
        let tracker = self.tracker.lock().unwrap();
        tracker.get(id).cloned()
    }

    /// Remove completed progress (cleanup)
    pub fn remove_progress(&self, id: &str) -> Option<IngestionProgress> {
        let mut tracker = self.tracker.lock().unwrap();
        tracker.remove(id)
    }

    /// Get all active progress operations
    pub fn get_all_progress(&self) -> Vec<IngestionProgress> {
        let tracker = self.tracker.lock().unwrap();
        tracker.values().cloned().collect()
    }
}
