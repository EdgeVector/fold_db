//! Utility functions for DynamoDB operations
//! Provides retry logic, error detection, and helper functions

use super::error::{StorageError, StorageResult};
use std::time::Duration;

/// Maximum number of retries for transient failures
pub const MAX_RETRIES: u32 = 3;

/// Maximum number of retries for batch operations (more retries for unprocessed items)
pub const MAX_BATCH_RETRIES: u32 = 10;

/// DynamoDB batch write limit (maximum items per BatchWriteItem call)
pub const DYNAMODB_BATCH_SIZE: usize = 25;

/// Base delay for exponential backoff (in milliseconds)
/// Higher for throttling scenarios
const BASE_DELAY_MS: u64 = 500;

/// Get the current user_id from request context.
/// Returns an error if no user context is available — all DynamoDB operations
/// MUST run within a `run_with_user` scope to ensure proper tenant isolation.
pub fn require_user_context() -> StorageResult<String> {
    crate::logging::core::get_current_user_id().ok_or_else(|| {
        StorageError::ConfigurationError(
            "Missing user context for DynamoDB operation. \
             Ensure this code runs within a run_with_user scope."
                .to_string(),
        )
    })
}

/// Check if an error is retryable (throttling, service errors, etc.)
pub fn is_retryable_error(error_msg: &str) -> bool {
    error_msg.contains("ProvisionedThroughputExceededException")
        || error_msg.contains("ThrottlingException")
        || error_msg.contains("ServiceUnavailable")
        || error_msg.contains("InternalServerError")
        || error_msg.contains("ServiceError")
        || error_msg.contains("RequestLimitExceeded")
}

/// Calculate exponential backoff delay
pub fn exponential_backoff(retry_count: u32) -> Duration {
    let delay_ms = BASE_DELAY_MS * (1u64 << retry_count.min(10)); // Cap at 2^10 = 1024x base delay
    Duration::from_millis(delay_ms)
}

/// Helper macro to reduce duplication in retry logic
/// Usage: retry_operation!(operation, operation_name, table_name, key, max_retries, error_converter)
#[macro_export]
macro_rules! retry_operation {
    ($op:expr, $op_name:expr, $table:expr, $key:expr, $max_retries:expr, $err_conv:expr) => {{
        use $crate::storage::dynamodb_utils::{is_retryable_error, exponential_backoff, format_dynamodb_error};
        let mut retries = 0;
        loop {
            match $op.await {
                Ok(result) => break Ok(result),
                Err(e) => {
                    let error_str = e.to_string();
                    // Log detailed error information for debugging
                    if retries == 0 {
                        // Only log on first attempt to avoid spam
                        log::debug!(
                            "DynamoDB {} error (attempt {}): {}",
                            $op_name,
                            retries + 1,
                            error_str
                        );
                        // Log the full error for debugging (the error string should contain details)
                        log::debug!("Full error details: {:?}", e);
                    }
                    if retries >= $max_retries {
                        let detailed_error = format_dynamodb_error($op_name, $table, $key, &error_str);
                        log::error!("DynamoDB {} failed after {} retries: {}", $op_name, retries + 1, detailed_error);
                        break Err($err_conv(detailed_error));
                    }
                    if is_retryable_error(&error_str) {
                        let delay = exponential_backoff(retries);
                        log::debug!("Retrying {} after {:?} delay (attempt {}/{})", $op_name, delay, retries + 1, $max_retries);
                        tokio::time::sleep(delay).await;
                        retries += 1;
                        continue;
                    }
                    // Non-retryable error
                    let detailed_error = format_dynamodb_error($op_name, $table, $key, &error_str);
                    log::error!("DynamoDB {} failed with non-retryable error: {}", $op_name, detailed_error);
                    break Err($err_conv(detailed_error));
                }
            }
        }
    }};
}

/// Format error with context (table name, operation, etc.)
pub fn format_dynamodb_error(
    operation: &str,
    table_name: &str,
    key: Option<&str>,
    error: impl std::fmt::Display,
) -> String {
    if let Some(k) = key {
        format!(
            "DynamoDB {} failed for table '{}', key '{}': {}",
            operation, table_name, k, error
        )
    } else {
        format!(
            "DynamoDB {} failed for table '{}': {}",
            operation, table_name, error
        )
    }
}

/// Helper to handle batch write operations with unprocessed items retry logic
/// Takes a closure that executes the batch operation and returns the result
pub async fn retry_batch_operation<F>(
    mut batch_operation: F,
    table_name: &str,
    initial_requests: Vec<aws_sdk_dynamodb::types::WriteRequest>,
) -> Result<(), String>
where
    F: FnMut(
        &[aws_sdk_dynamodb::types::WriteRequest],
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        aws_sdk_dynamodb::operation::batch_write_item::BatchWriteItemOutput,
                        aws_sdk_dynamodb::error::SdkError<
                            aws_sdk_dynamodb::operation::batch_write_item::BatchWriteItemError,
                        >,
                    >,
                > + Send,
        >,
    >,
{
    let mut remaining_requests = initial_requests;
    let mut retries = 0;

    while !remaining_requests.is_empty() && retries < MAX_BATCH_RETRIES {
        let result = batch_operation(&remaining_requests).await;

        match result {
            Ok(response) => {
                if let Some(unprocessed) = response.unprocessed_items {
                    if let Some(unprocessed_reqs) = unprocessed.get(table_name) {
                        if !unprocessed_reqs.is_empty() {
                            remaining_requests = unprocessed_reqs.clone();
                            let delay = exponential_backoff(retries);
                            tokio::time::sleep(delay).await;
                            retries += 1;
                            continue;
                        }
                    }
                }
                return Ok(()); // All items processed
            }
            Err(e) => {
                let error_str = e.to_string();
                // Log full error details for debugging
                log::error!(
                    "DynamoDB batch_write_item error for table '{}': {} | Full error: {:?}",
                    table_name,
                    error_str,
                    e
                );
                let error_msg = format_dynamodb_error(
                    "batch_write_item",
                    table_name,
                    None,
                    format!("{:?}", e),
                );

                if retries < MAX_BATCH_RETRIES
                    && (is_retryable_error(&error_str) || is_retryable_error(&format!("{:?}", e)))
                {
                    let delay = exponential_backoff(retries);
                    log::info!(
                        "Retrying batch_write_item after {:?} (attempt {}/{})",
                        delay,
                        retries + 1,
                        MAX_BATCH_RETRIES
                    );
                    tokio::time::sleep(delay).await;
                    retries += 1;
                    continue;
                }

                return Err(error_msg);
            }
        }
    }

    if !remaining_requests.is_empty() {
        return Err(format!(
            "Failed to process {} items in table '{}' after {} retries",
            remaining_requests.len(),
            table_name,
            MAX_BATCH_RETRIES
        ));
    }

    Ok(())
}
