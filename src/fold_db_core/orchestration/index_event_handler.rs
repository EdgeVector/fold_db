//! Index Event Handler - Handles background indexing requests
//!
//! This module listens for BatchIndexRequest events and processes them
//! asynchronously to avoid blocking mutation operations.

use std::sync::Arc;

use std::time::Duration;
use log::{error, info};

use crate::db_operations::DbOperations;
use crate::fold_db_core::infrastructure::MessageBus;
use crate::fold_db_core::infrastructure::message_bus::request_events::BatchIndexRequest;
use crate::schema::SchemaError;
use super::index_status::IndexStatusTracker;

pub struct IndexEventHandler {
    _monitoring_task: Option<tokio::task::JoinHandle<()>>,
    status_tracker: IndexStatusTracker,
}

impl IndexEventHandler {
    /// Create a new IndexEventHandler and start monitoring
    pub fn new(
        message_bus: Arc<MessageBus>,
        db_ops: Arc<DbOperations>,
        status_tracker: Option<IndexStatusTracker>,
    ) -> Self {
        let status_tracker = status_tracker.unwrap_or_else(|| IndexStatusTracker::new(None));
        
        let monitoring_task = Self::start_monitoring(
            Arc::clone(&message_bus),
            Arc::clone(&db_ops),
            status_tracker.clone(),
        );

        Self {
            _monitoring_task: Some(monitoring_task),
            status_tracker,
        }
    }
    
    /// Get the current indexing status
    pub async fn get_status(&self) -> super::index_status::IndexingStatus {
        self.status_tracker.get_status().await
    }
    
    /// Check if indexing is currently in progress
    pub async fn is_indexing(&self) -> bool {
        self.status_tracker.is_indexing().await
    }

    /// Start monitoring for BatchIndexRequest events
    fn start_monitoring(
        message_bus: Arc<MessageBus>,
        db_ops: Arc<DbOperations>,
        status_tracker: IndexStatusTracker,
    ) -> tokio::task::JoinHandle<()> {
        let mut consumer = message_bus.subscribe::<BatchIndexRequest>();
        
        tokio::spawn(async move {
            info!("🔍 IndexEventHandler: Starting monitoring for BatchIndexRequest events");

            loop {
                // Check for BatchIndexRequest events
                // Note: try_recv is non-blocking, so we still need to poll
                // In a real async system, we'd want an async channel
                match consumer.try_recv() {
                    Ok(event) => {
                        if let Err(e) = Self::handle_batch_index_request(&event, &db_ops, &status_tracker).await {
                            error!("❌ IndexEventHandler: Error handling batch index request: {}", e);
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // No events available, sleep briefly to avoid busy waiting
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        error!("❌ IndexEventHandler: Message bus consumer disconnected");
                        break;
                    }
                }
            }
        })
    }

    /// Handle a BatchIndexRequest event by processing all index operations
    async fn handle_batch_index_request(
        event: &BatchIndexRequest,
        db_ops: &Arc<DbOperations>,
        status_tracker: &IndexStatusTracker,
    ) -> Result<(), SchemaError> {
        let operation_count = event.operations.len();
        
        // Update status: indexing started
        status_tracker.start_batch(operation_count).await;
        
        let start_time = std::time::Instant::now();
        
        // Convert IndexRequest events to the format expected by batch_index_field_values_with_classifications
        let index_operations: Vec<_> = event.operations.iter()
            .map(|req| {
                (
                    req.schema_name.clone(),
                    req.field_name.clone(),
                    req.key_value.clone(),
                    req.value.clone(),
                    req.classifications.clone(),
                )
            })
            .collect();
        
        // Process all index operations in a batch
        // We need to handle both sync and async index managers
        let result = if let Some(native_index_mgr) = db_ops.native_index_manager() {
            if native_index_mgr.is_async() {
                native_index_mgr.batch_index_field_values_with_classifications_async(&index_operations).await
            } else {
                native_index_mgr.batch_index_field_values_with_classifications(&index_operations)
            }
        } else {
            Err(SchemaError::InvalidData("Native index manager not available".to_string()))
        };
        
        let elapsed = start_time.elapsed();
        
        // Keep the "Indexing" state visible for at least 500ms so UI can display it
        // This is purely for UI feedback - the actual indexing is already done
        if elapsed.as_millis() < 500 {
            tokio::time::sleep(Duration::from_millis(500 - elapsed.as_millis() as u64)).await;
        }
        
        // Update status: indexing completed
        status_tracker.complete_batch(operation_count, elapsed.as_millis()).await;
        
        match result {
            Ok(_) => {
                info!("✅ IndexEventHandler: Processed {} index operations in {:.2}ms", 
                    operation_count, elapsed.as_millis());
                Ok(())
            }
            Err(e) => {
                error!("❌ IndexEventHandler: Failed to process {} index operations: {}", 
                    operation_count, e);
                Err(e)
            }
        }
    }
}

