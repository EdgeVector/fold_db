//! Shared Ingestion Handlers
//!
//! Framework-agnostic handlers for ingestion operations.
//! These can be called by both HTTP server routes and Lambda handlers.

use crate::handlers::response::{ApiResponse, HandlerError, HandlerResult};
use crate::ingestion::progress::{IngestionProgress, ProgressService, ProgressTracker};
use crate::progress::JobType;

/// Response type for get_all_progress
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProgressListResponse {
    /// List of progress items
    pub progress: Vec<IngestionProgress>,
}

/// Get all ingestion/indexing progress for a user
///
/// # Arguments
/// * `user_hash` - The user's hash for isolation
/// * `tracker` - Progress tracker instance
///
/// # Returns
/// * `HandlerResult<ProgressListResponse>` - List of progress items wrapped in standard envelope
pub async fn get_all_progress(
    user_hash: &str,
    tracker: &ProgressTracker,
) -> HandlerResult<ProgressListResponse> {
    let jobs = tracker
        .list_by_user(user_hash)
        .await
        .map_err(|e| HandlerError::Internal(format!("Failed to list progress: {}", e)))?;

    let progress: Vec<IngestionProgress> = jobs
        .into_iter()
        .filter(|j| matches!(j.job_type, JobType::Ingestion | JobType::Indexing))
        .map(|j| j.into())
        .collect();

    Ok(ApiResponse::success_with_user(
        ProgressListResponse { progress },
        user_hash,
    ))
}

/// Get progress for a specific job
///
/// # Arguments
/// * `id` - The progress ID
/// * `user_hash` - The user's hash for isolation
/// * `tracker` - Progress tracker instance
///
/// # Returns
/// * `HandlerResult<IngestionProgress>` - Progress item wrapped in standard envelope
pub async fn get_progress(
    id: &str,
    user_hash: &str,
    tracker: &ProgressTracker,
) -> HandlerResult<IngestionProgress> {
    let progress_service = ProgressService::new(tracker.clone());

    match progress_service.get_progress(id).await {
        Some(progress) => Ok(ApiResponse::success_with_user(progress, user_hash)),
        None => Err(HandlerError::NotFound(format!(
            "Progress not found for ID: {}",
            id
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_list_response_serialization() {
        let response = ProgressListResponse { progress: vec![] };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("progress"));
    }
}
