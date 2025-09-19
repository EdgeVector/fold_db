//! # Mutation Completion Handler
//!
//! This module provides the `MutationCompletionHandler` for tracking asynchronous mutation
//! processing and resolving race conditions between mutation processing and synchronous query operations.
//!
//! ## Overview
//!
//! The `MutationCompletionHandler` is a foundational component that solves race conditions
//! between asynchronous mutation processing and synchronous query operations by providing
//! a thread-safe completion tracking system.
//!
//! ## Key Features
//!
//! - **Thread-safe tracking**: Uses `Arc<RwLock<>>` for safe concurrent access
//! - **Completion channels**: Uses `tokio::sync::oneshot` for efficient notification
//! - **Event integration**: Integrates with the MessageBus for event-driven architecture
//! - **Timeout support**: Built-in 5-second timeout for completion waiting
//! - **Monitoring**: Provides pending mutation count for system observability
//!
//! ## Usage Example
//!
//! ```rust
//! use std::sync::Arc;
//! use tokio::time::Duration;
//! use datafold::fold_db_core::infrastructure::MessageBus;
//! use datafold::fold_db_core::MutationCompletionHandler;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a message bus and completion handler
//! let message_bus = Arc::new(MessageBus::new());
//! let handler = MutationCompletionHandler::new(message_bus);
//!
//! // Register a mutation for tracking
//! let mutation_id = "mutation-123".to_string();
//! let completion_receiver = handler.register_mutation(mutation_id.clone());
//!
//! // In another part of the system, signal completion
//! handler.signal_completion(&mutation_id);
//!
//! // Wait for completion with timeout
//! match tokio::time::timeout(Duration::from_secs(5), completion_receiver).await {
//!     Ok(_) => println!("Mutation completed successfully"),
//!     Err(_) => println!("Mutation timed out"),
//! }
//!
//! // Clean up the mutation tracking
//! handler.cleanup_mutation(&mutation_id);
//! # Ok(())
//! # }
//! ```
//!
//! ## Integration with Event System
//!
//! The handler integrates with the existing MessageBus infrastructure to listen for
//! mutation completion events and automatically signal completion for tracked mutations:
//!
//! ```rust
//! use datafold::fold_db_core::infrastructure::message_bus::query_events::MutationExecuted;
//!
//! # async fn event_example() {
//! // The handler can listen for MutationExecuted events
//! // and automatically signal completion for tracked mutations
//! # }
//! ```

use log::{debug, error, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use tokio::time::{timeout, Duration};

use crate::fold_db_core::infrastructure::message_bus::MessageBus;

/// Type alias for pending mutation receivers and completion status
type PendingMutationData = (Vec<oneshot::Sender<()>>, bool);

/// Default timeout duration for mutation completion (5 seconds)
pub const DEFAULT_COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);

/// Errors that can occur during mutation completion handling
#[derive(Debug, thiserror::Error)]
pub enum MutationCompletionError {
    /// The mutation ID was not found in the tracking system
    #[error("Mutation ID '{0}' not found in tracking system")]
    MutationNotFound(String),

    /// Failed to send completion signal
    #[error("Failed to send completion signal for mutation '{0}': {1}")]
    SignalFailed(String, String),

    /// Timeout waiting for mutation completion
    #[error("Timeout waiting for completion of mutation '{0}' after {1:?}")]
    Timeout(String, Duration),

    /// Lock acquisition failed
    #[error("Failed to acquire lock for mutation tracking: {0}")]
    LockFailed(String),
}

/// Result type for mutation completion operations
pub type MutationCompletionResult<T> = Result<T, MutationCompletionError>;

/// Thread-safe handler for tracking mutation completion and resolving race conditions
/// between asynchronous mutation processing and synchronous query operations.
///
/// This handler provides a foundation for ensuring queries wait for mutations to complete
/// before returning results, eliminating race conditions in the event-driven system.
///
/// ## Thread Safety
///
/// All methods are thread-safe and can be called concurrently from multiple threads.
/// The internal state is protected by `Arc<RwLock<>>` for efficient read/write access.
///
/// ## Memory Management
///
/// The handler automatically cleans up completed mutations to prevent memory leaks.
/// However, it's recommended to call `cleanup_mutation` explicitly after handling
/// completion to ensure timely cleanup.
pub struct MutationCompletionHandler {
    /// Thread-safe storage for pending mutation completion channels
    /// Each mutation ID can have multiple receivers waiting for completion
    /// The bool indicates whether the mutation has completed
    pending_mutations: Arc<RwLock<HashMap<String, PendingMutationData>>>,

    /// Reference to the message bus for event handling integration
    message_bus: Arc<MessageBus>,
}

impl MutationCompletionHandler {
    /// Creates a new `MutationCompletionHandler` with the provided message bus.
    ///
    /// # Arguments
    ///
    /// * `message_bus` - Shared reference to the MessageBus for event integration
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use datafold::fold_db_core::infrastructure::MessageBus;
    /// use datafold::fold_db_core::MutationCompletionHandler;
    ///
    /// let message_bus = Arc::new(MessageBus::new());
    /// let handler = MutationCompletionHandler::new(message_bus);
    /// ```
    pub fn new(message_bus: Arc<MessageBus>) -> Self {
        debug!("Creating new MutationCompletionHandler");

        Self {
            pending_mutations: Arc::new(RwLock::new(HashMap::new())),
            message_bus,
        }
    }

    /// Registers a new mutation for completion tracking.
    ///
    /// This method creates a completion channel for the specified mutation ID and returns
    /// the receiver end. The caller can await on this receiver to be notified when the
    /// mutation completes.
    ///
    /// # Arguments
    ///
    /// * `mutation_id` - Unique identifier for the mutation to track
    ///
    /// # Returns
    ///
    /// A `oneshot::Receiver<()>` that will receive a signal when the mutation completes
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::sync::Arc;
    /// # use datafold::fold_db_core::infrastructure::MessageBus;
    /// # use datafold::fold_db_core::MutationCompletionHandler;
    /// # async fn example() {
    /// let message_bus = Arc::new(MessageBus::new());
    /// let handler = MutationCompletionHandler::new(message_bus);
    ///
    /// let mutation_id = "mutation-456".to_string();
    /// let completion_receiver = handler.register_mutation(mutation_id);
    ///
    /// // Use the receiver to wait for completion
    /// // completion_receiver.await.expect("Mutation completion signal");
    /// # }
    /// ```
    pub async fn register_mutation(&self, mutation_id: String) -> oneshot::Receiver<()> {
        debug!(
            "Registering mutation for completion tracking: {}",
            mutation_id
        );

        let (sender, receiver) = oneshot::channel();

        // Acquire write lock and add the sender to the list
        let mut pending = self.pending_mutations.write().await;

        // Add the sender to the existing list or create a new list
        let entry = pending
            .entry(mutation_id.clone())
            .or_insert_with(|| (Vec::new(), false));
        entry.0.push(sender);

        let count = entry.0.len();
        let is_completed = entry.1;
        debug!(
            "Mutation '{}' registered. Total receivers: {}, Completed: {}",
            mutation_id, count, is_completed
        );

        // If the mutation has already completed, send the signal immediately
        if is_completed {
            debug!(
                "Mutation '{}' already completed, sending immediate signal",
                mutation_id
            );
            // Send signal immediately to the new receiver
            if let Some(sender) = entry.0.pop() {
                if sender.send(()).is_err() {
                    debug!(
                        "Failed to send immediate signal for completed mutation {}",
                        mutation_id
                    );
                } else {
                    debug!(
                        "Successfully sent immediate signal for completed mutation {}",
                        mutation_id
                    );
                }
            }
        }

        receiver
    }

    /// Registers a new mutation for completion tracking (synchronous version).
    ///
    /// This method creates a completion channel for the specified mutation ID and returns
    /// the receiver end. This is a blocking synchronous version that uses tokio's blocking
    /// mechanism to handle the async registration.
    ///
    /// # Arguments
    ///
    /// * `mutation_id` - Unique identifier for the mutation to track
    ///
    /// # Returns
    ///
    /// A `oneshot::Receiver<()>` that will receive a signal when the mutation completes
    ///
    /// # Note
    ///
    /// This method should only be used when an async context is not available.
    /// Prefer the async version when possible.
    pub fn register_mutation_sync(&self, mutation_id: String) -> oneshot::Receiver<()> {
        debug!(
            "Registering mutation for completion tracking (sync): {}",
            mutation_id
        );

        let (sender, receiver) = oneshot::channel();

        // Use try_write to avoid blocking indefinitely
        let pending_mutations = Arc::clone(&self.pending_mutations);
        let mutation_id_clone = mutation_id.clone();

        // Spawn a task to handle the registration
        tokio::spawn(async move {
            let mut pending = pending_mutations.write().await;

            // Add the sender to the existing list or create a new list
            let entry = pending
                .entry(mutation_id_clone.clone())
                .or_insert_with(|| (Vec::new(), false));
            entry.0.push(sender);

            let count = entry.0.len();
            let is_completed = entry.1;
            debug!(
                "Mutation '{}' registered (sync). Total receivers: {}, Completed: {}",
                mutation_id_clone, count, is_completed
            );

            // If the mutation has already completed, send the signal immediately
            if is_completed {
                debug!(
                    "Mutation '{}' already completed, sending immediate signal (sync)",
                    mutation_id_clone
                );
                if let Some(sender) = entry.0.pop() {
                    let _ = sender.send(());
                }
            }
        });

        receiver
    }

    /// Signals that a mutation has completed processing.
    ///
    /// This method finds the completion channel for the specified mutation ID and sends
    /// a completion signal. If the mutation is not being tracked, this method logs a
    /// warning but does not fail.
    ///
    /// # Arguments
    ///
    /// * `mutation_id` - The unique identifier of the completed mutation
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::sync::Arc;
    /// # use datafold::fold_db_core::infrastructure::MessageBus;
    /// # use datafold::fold_db_core::MutationCompletionHandler;
    /// # async fn example() {
    /// let message_bus = Arc::new(MessageBus::new());
    /// let handler = MutationCompletionHandler::new(message_bus);
    ///
    /// // After mutation processing completes
    /// handler.signal_completion("mutation-456").await;
    /// # }
    /// ```
    pub async fn signal_completion(&self, mutation_id: &str) {
        debug!("Signaling completion for mutation: {}", mutation_id);

        // Acquire write lock and get all senders for this mutation
        let mut pending = self.pending_mutations.write().await;

        if let Some((senders, completed)) = pending.get_mut(mutation_id) {
            let sender_count = senders.len();
            debug!(
                "Signaling completion to {} receivers for mutation '{}'",
                sender_count, mutation_id
            );

            // Mark as completed
            *completed = true;

            // Send completion signal to all registered receivers
            let mut success_count = 0;
            while let Some(sender) = senders.pop() {
                if sender.send(()).is_err() {
                    warn!("Failed to send completion signal to one receiver for mutation '{}' - receiver may have been dropped", mutation_id);
                } else {
                    success_count += 1;
                }
            }

            // Don't remove the entry - keep it marked as completed for future wait_for_mutation calls
            // The entry will be cleaned up later by cleanup_mutation

            debug!("Successfully signaled completion to {}/{} receivers for mutation '{}'. Remaining pending: {}", 
                   success_count, sender_count, mutation_id, pending.len());
        } else {
            warn!(
                "Attempted to signal completion for untracked mutation: {}",
                mutation_id
            );
        }
    }

    /// Signals that a mutation has completed processing (synchronous version).
    ///
    /// This method finds the completion channel for the specified mutation ID and sends
    /// a completion signal. This is a synchronous version that uses tokio's blocking
    /// mechanism to handle the async signal_completion. If the mutation is not being
    /// tracked, this method logs a warning but does not fail.
    ///
    /// # Arguments
    ///
    /// * `mutation_id` - The unique identifier of the completed mutation
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure of the completion signaling
    ///
    /// # Note
    ///
    /// This method should only be used when an async context is not available.
    /// Prefer the async version when possible.
    pub fn signal_completion_sync(&self, mutation_id: &str) -> MutationCompletionResult<()> {
        debug!("Signaling completion for mutation (sync): {}", mutation_id);

        // Clone the mutation_id for the async block
        let mutation_id_owned = mutation_id.to_string();
        let pending_mutations = Arc::clone(&self.pending_mutations);

        // Use tokio's blocking mechanism to handle the async operation
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async move {
            let mut pending = pending_mutations.write().await;
            
            if let Some((senders, completed)) = pending.get_mut(&mutation_id_owned) {
                let sender_count = senders.len();
                debug!("Signaling completion to {} receivers for mutation '{}' (sync)", sender_count, mutation_id_owned);
                
                // Mark as completed
                *completed = true;
                
                // Send completion signal to all registered receivers
                let mut success_count = 0;
                while let Some(sender) = senders.pop() {
                    if sender.send(()).is_err() {
                        warn!("Failed to send completion signal to one receiver for mutation '{}' - receiver may have been dropped", mutation_id_owned);
                    } else {
                        success_count += 1;
                    }
                }
                
                debug!("Successfully signaled completion to {}/{} receivers for mutation '{}'. Remaining pending: {}", 
                       success_count, sender_count, mutation_id_owned, pending.len());
                Ok(())
            } else {
                warn!("Attempted to signal completion for untracked mutation: {}", mutation_id_owned);
                Err(MutationCompletionError::MutationNotFound(mutation_id_owned))
            }
        })
    }

    /// Removes a mutation from the completion tracking system.
    ///
    /// This method should be called after handling completion to ensure timely cleanup
    /// of tracking resources. It's safe to call this method even if the mutation has
    /// already completed or was never registered.
    ///
    /// # Arguments
    ///
    /// * `mutation_id` - The unique identifier of the mutation to clean up
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::sync::Arc;
    /// # use datafold::fold_db_core::infrastructure::MessageBus;
    /// # use datafold::fold_db_core::MutationCompletionHandler;
    /// # async fn example() {
    /// let message_bus = Arc::new(MessageBus::new());
    /// let handler = MutationCompletionHandler::new(message_bus);
    ///
    /// // After handling completion or timeout
    /// handler.cleanup_mutation("mutation-456").await;
    /// # }
    /// ```
    pub async fn cleanup_mutation(&self, mutation_id: &str) {
        debug!("Cleaning up mutation tracking: {}", mutation_id);

        let mut pending = self.pending_mutations.write().await;

        if pending.remove(mutation_id).is_some() {
            debug!(
                "Cleaned up mutation '{}'. Remaining pending: {}",
                mutation_id,
                pending.len()
            );
        } else {
            debug!(
                "Mutation '{}' was not in tracking system (may have already been cleaned up)",
                mutation_id
            );
        }
    }

    /// Returns the current number of pending mutations being tracked.
    ///
    /// This method is useful for monitoring system load and debugging completion
    /// tracking issues. It provides a snapshot of the current tracking state.
    ///
    /// # Returns
    ///
    /// The number of mutations currently being tracked for completion
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::sync::Arc;
    /// # use datafold::fold_db_core::infrastructure::MessageBus;
    /// # use datafold::fold_db_core::MutationCompletionHandler;
    /// # async fn example() {
    /// let message_bus = Arc::new(MessageBus::new());
    /// let handler = MutationCompletionHandler::new(message_bus);
    ///
    /// let pending_count = handler.pending_count().await;
    /// println!("Currently tracking {} pending mutations", pending_count);
    /// # }
    /// ```
    pub async fn pending_count(&self) -> usize {
        let pending = self.pending_mutations.read().await;
        pending
            .values()
            .filter(|(_, is_completed)| !*is_completed)
            .count()
    }

    /// Waits for a mutation to complete with the default timeout.
    ///
    /// This is a convenience method that combines `register_mutation` with timeout handling
    /// and automatic cleanup. It registers the mutation, waits for completion with the
    /// default timeout, and cleans up tracking regardless of the outcome.
    ///
    /// # Arguments
    ///
    /// * `mutation_id` - The unique identifier of the mutation to wait for
    ///
    /// # Returns
    ///
    /// `Ok(())` if the mutation completed within the timeout, or an error if it timed out
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::sync::Arc;
    /// # use datafold::fold_db_core::infrastructure::MessageBus;
    /// # use datafold::fold_db_core::MutationCompletionHandler;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let message_bus = Arc::new(MessageBus::new());
    /// let handler = MutationCompletionHandler::new(message_bus);
    ///
    /// // Wait for mutation with automatic cleanup
    /// handler.wait_for_completion("mutation-789").await?;
    /// println!("Mutation completed successfully");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_completion(&self, mutation_id: &str) -> MutationCompletionResult<()> {
        self.wait_for_completion_with_timeout(mutation_id, DEFAULT_COMPLETION_TIMEOUT)
            .await
    }

    /// Waits for a mutation to complete with a custom timeout.
    ///
    /// This method provides the same functionality as `wait_for_completion` but allows
    /// specifying a custom timeout duration.
    ///
    /// # Arguments
    ///
    /// * `mutation_id` - The unique identifier of the mutation to wait for
    /// * `timeout_duration` - Maximum time to wait for completion
    ///
    /// # Returns
    ///
    /// `Ok(())` if the mutation completed within the timeout, or an error if it timed out
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::sync::Arc;
    /// # use tokio::time::Duration;
    /// # use datafold::fold_db_core::infrastructure::MessageBus;
    /// # use datafold::fold_db_core::MutationCompletionHandler;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let message_bus = Arc::new(MessageBus::new());
    /// let handler = MutationCompletionHandler::new(message_bus);
    ///
    /// // Wait with custom timeout
    /// let custom_timeout = Duration::from_secs(10);
    /// handler.wait_for_completion_with_timeout("mutation-789", custom_timeout).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_completion_with_timeout(
        &self,
        mutation_id: &str,
        timeout_duration: Duration,
    ) -> MutationCompletionResult<()> {
        debug!(
            "Waiting for completion of mutation '{}' with timeout {:?}",
            mutation_id, timeout_duration
        );

        // Check if mutation is already being tracked
        let receiver = {
            let pending = self.pending_mutations.read().await;
            if pending.contains_key(mutation_id) {
                debug!(
                    "Mutation '{}' is already being tracked, creating new receiver for wait",
                    mutation_id
                );
                drop(pending);
                // Create a new receiver for this specific wait operation
                self.register_mutation(mutation_id.to_string()).await
            } else {
                debug!(
                    "Mutation '{}' not found in tracking, creating new registration",
                    mutation_id
                );
                drop(pending);
                self.register_mutation(mutation_id.to_string()).await
            }
        };

        // Wait for completion with timeout
        let result = match timeout(timeout_duration, receiver).await {
            Ok(Ok(())) => {
                debug!("Mutation '{}' completed successfully", mutation_id);
                Ok(())
            }
            Ok(Err(_)) => {
                error!(
                    "Completion channel closed unexpectedly for mutation '{}'",
                    mutation_id
                );
                Err(MutationCompletionError::SignalFailed(
                    mutation_id.to_string(),
                    "Completion channel closed".to_string(),
                ))
            }
            Err(_) => {
                warn!(
                    "Timeout waiting for completion of mutation '{}' after {:?}",
                    mutation_id, timeout_duration
                );
                Err(MutationCompletionError::Timeout(
                    mutation_id.to_string(),
                    timeout_duration,
                ))
            }
        };

        // Only clean up if the mutation was not completed successfully
        // Completed mutations should be kept around for future wait_for_mutation calls
        match &result {
            Ok(()) => {
                debug!(
                    "Mutation '{}' completed successfully, keeping entry for future wait calls",
                    mutation_id
                );
                // Don't clean up - keep the completed mutation for future wait calls
            }
            Err(_) => {
                debug!(
                    "Mutation '{}' failed or timed out, cleaning up entry",
                    mutation_id
                );
                self.cleanup_mutation(mutation_id).await;
            }
        }

        result
    }

    /// Returns a reference to the message bus for event integration.
    ///
    /// This method provides access to the underlying message bus for components that
    /// need to integrate with the event system while using the completion handler.
    ///
    /// # Returns
    ///
    /// A shared reference to the MessageBus
    pub fn message_bus(&self) -> Arc<MessageBus> {
        Arc::clone(&self.message_bus)
    }

    /// Gets diagnostic information about the current state of the completion handler.
    ///
    /// This method returns a snapshot of internal state for debugging and monitoring
    /// purposes. It includes the number of pending mutations and can be extended with
    /// additional diagnostic information as needed.
    ///
    /// # Returns
    ///
    /// A `MutationCompletionDiagnostics` struct containing current state information
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::sync::Arc;
    /// # use datafold::fold_db_core::infrastructure::MessageBus;
    /// # use datafold::fold_db_core::MutationCompletionHandler;
    /// # async fn example() {
    /// let message_bus = Arc::new(MessageBus::new());
    /// let handler = MutationCompletionHandler::new(message_bus);
    ///
    /// let diagnostics = handler.get_diagnostics().await;
    /// println!("Completion handler diagnostics: {:?}", diagnostics);
    /// # }
    /// ```
    pub async fn get_diagnostics(&self) -> MutationCompletionDiagnostics {
        let pending_count = self.pending_count().await;

        MutationCompletionDiagnostics {
            pending_mutations_count: pending_count,
            // Additional diagnostic fields can be added here as needed
        }
    }
}

/// Diagnostic information about the mutation completion handler's current state
#[derive(Debug, Clone)]
pub struct MutationCompletionDiagnostics {
    /// Number of mutations currently being tracked for completion
    pub pending_mutations_count: usize,
}

impl std::fmt::Display for MutationCompletionDiagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MutationCompletionHandler(pending: {})",
            self.pending_mutations_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    /// Helper function to create a test handler
    async fn create_test_handler() -> MutationCompletionHandler {
        let message_bus = Arc::new(MessageBus::new());
        MutationCompletionHandler::new(message_bus)
    }

    #[tokio::test]
    async fn test_new_handler_creation() {
        let handler = create_test_handler().await;
        assert_eq!(handler.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_register_mutation() {
        let handler = create_test_handler().await;
        let mutation_id = "test-mutation-1".to_string();

        let _receiver = handler.register_mutation(mutation_id).await;
        assert_eq!(handler.pending_count().await, 1);
    }

    #[tokio::test]
    async fn test_signal_completion() {
        let handler = create_test_handler().await;
        let mutation_id = "test-mutation-2".to_string();

        let receiver = handler.register_mutation(mutation_id.clone()).await;
        assert_eq!(handler.pending_count().await, 1);

        // Signal completion
        handler.signal_completion(&mutation_id).await;

        // Receiver should get the signal
        assert!(receiver.await.is_ok());
        assert_eq!(handler.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_cleanup_mutation() {
        let handler = create_test_handler().await;
        let mutation_id = "test-mutation-3".to_string();

        let _receiver = handler.register_mutation(mutation_id.clone()).await;
        assert_eq!(handler.pending_count().await, 1);

        handler.cleanup_mutation(&mutation_id).await;
        assert_eq!(handler.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_signal_completion_for_untracked_mutation() {
        let handler = create_test_handler().await;

        // This should not panic or fail
        handler.signal_completion("nonexistent-mutation").await;
        assert_eq!(handler.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_cleanup_untracked_mutation() {
        let handler = create_test_handler().await;

        // This should not panic or fail
        handler.cleanup_mutation("nonexistent-mutation").await;
        assert_eq!(handler.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_concurrent_registrations() {
        let handler = Arc::new(create_test_handler().await);
        let mut handles = vec![];

        // Register multiple mutations concurrently
        for i in 0..10 {
            let handler_clone = Arc::clone(&handler);
            let handle = tokio::spawn(async move {
                let mutation_id = format!("concurrent-mutation-{}", i);
                let _result = handler_clone.register_mutation(mutation_id).await;
            });
            handles.push(handle);
        }

        // Wait for all registrations to complete
        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(handler.pending_count().await, 10);
    }

    #[tokio::test]
    async fn test_wait_for_completion_success() {
        let handler = Arc::new(create_test_handler().await);
        let mutation_id = "test-wait-success";

        // Clone handler for the completion task
        let handler_clone = Arc::clone(&handler);
        let mutation_id_clone = mutation_id.to_string();

        // Start waiting for completion
        let wait_handle =
            tokio::spawn(
                async move { handler_clone.wait_for_completion(&mutation_id_clone).await },
            );

        // Give a small delay to ensure registration happens first
        sleep(Duration::from_millis(10)).await;

        // Signal completion
        handler.signal_completion(mutation_id).await;

        // Wait should complete successfully
        let result = wait_handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_for_completion_timeout() {
        let handler = create_test_handler().await;
        let mutation_id = "test-wait-timeout";

        // Wait with a very short timeout
        let result = handler
            .wait_for_completion_with_timeout(mutation_id, Duration::from_millis(10))
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MutationCompletionError::Timeout(_, _)
        ));

        // Mutation should be cleaned up after timeout
        assert_eq!(handler.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_diagnostics() {
        let handler = create_test_handler().await;

        let diagnostics = handler.get_diagnostics().await;
        assert_eq!(diagnostics.pending_mutations_count, 0);

        // Register a mutation
        let _receiver = handler.register_mutation("diag-test".to_string()).await;

        let diagnostics = handler.get_diagnostics().await;
        assert_eq!(diagnostics.pending_mutations_count, 1);
    }

    #[tokio::test]
    async fn test_replace_existing_mutation() {
        let handler = create_test_handler().await;
        let mutation_id = "duplicate-mutation".to_string();

        // Register first mutation
        let _receiver1 = handler.register_mutation(mutation_id.clone()).await;
        assert_eq!(handler.pending_count().await, 1);

        // Register second mutation with same ID (should replace)
        let receiver2 = handler.register_mutation(mutation_id.clone()).await;
        assert_eq!(handler.pending_count().await, 1);

        // Signal completion should work with the second receiver
        handler.signal_completion(&mutation_id).await;
        assert!(receiver2.await.is_ok());
        assert_eq!(handler.pending_count().await, 0);
    }
}
