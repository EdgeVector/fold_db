//! Database operations for Lambda context (mutations, transforms, backfills)

use crate::ingestion::IngestionError;
use serde_json::Value;

use super::context::LambdaContext;

impl LambdaContext {
    /// Execute a single mutation
    ///
    /// Creates a new record or updates an existing one.
    ///
    /// # Arguments
    ///
    /// * `mutation` - Mutation specification with schema, keys, fields, and values
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    /// use datafold::schema::types::Mutation;
    /// use serde_json::json;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mutation = Mutation {
    ///         schema_name: "users".to_string(),
    ///         keys_and_values: vec![("id".to_string(), json!("user123"))],
    ///         fields_and_values: vec![
    ///             ("name".to_string(), json!("Alice")),
    ///             ("email".to_string(), json!("alice@example.com")),
    ///         ],
    ///         trust_distance: 0,
    ///         pub_key: "default".to_string(),
    ///     };
    ///     
    ///     let mutation_id = LambdaContext::execute_mutation(mutation).await?;
    ///     println!("Mutation ID: {}", mutation_id);
    ///     Ok(())
    /// }
    /// ```
    pub async fn execute_mutation(mutation: crate::schema::types::Mutation) -> Result<String, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        node.mutate_batch(vec![mutation])
            .map_err(|e| IngestionError::InvalidInput(format!("Mutation failed: {}", e)))?
            .into_iter()
            .next()
            .ok_or_else(|| IngestionError::InvalidInput("No mutation ID returned".to_string()))
    }

    /// Execute multiple mutations in a batch
    ///
    /// More efficient than calling `execute_mutation()` multiple times.
    ///
    /// # Arguments
    ///
    /// * `mutations` - Vector of mutations to execute
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    /// use datafold::schema::types::Mutation;
    /// use serde_json::json;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mutations = vec![
    ///         Mutation {
    ///             schema_name: "users".to_string(),
    ///             keys_and_values: vec![("id".to_string(), json!("user1"))],
    ///             fields_and_values: vec![("name".to_string(), json!("Alice"))],
    ///             trust_distance: 0,
    ///             pub_key: "default".to_string(),
    ///         },
    ///         Mutation {
    ///             schema_name: "users".to_string(),
    ///             keys_and_values: vec![("id".to_string(), json!("user2"))],
    ///             fields_and_values: vec![("name".to_string(), json!("Bob"))],
    ///             trust_distance: 0,
    ///             pub_key: "default".to_string(),
    ///         },
    ///     ];
    ///     
    ///     let mutation_ids = LambdaContext::execute_mutations(mutations).await?;
    ///     println!("Created {} mutations", mutation_ids.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn execute_mutations(mutations: Vec<crate::schema::types::Mutation>) -> Result<Vec<String>, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        node.mutate_batch(mutations)
            .map_err(|e| IngestionError::InvalidInput(format!("Batch mutations failed: {}", e)))
    }

    /// List all registered transforms
    ///
    /// Returns a map of transform IDs to their definitions.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let transforms = LambdaContext::list_transforms().await?;
    ///     
    ///     for (id, transform) in transforms {
    ///         println!("Transform: {} - Schema: {}", id, transform.get_schema_name());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn list_transforms() -> Result<std::collections::HashMap<String, crate::schema::types::Transform>, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        node.list_transforms()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to list transforms: {}", e)))
    }

    /// Get transform queue information
    ///
    /// Returns information about the current state of the transform queue.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let queue_info = LambdaContext::get_transform_queue().await?;
    ///     println!("Queue info: {:?}", queue_info);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_transform_queue() -> Result<Value, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        let queue_info = node.get_transform_queue_info()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get transform queue: {}", e)))?;
        
        Ok(serde_json::to_value(queue_info).unwrap_or(serde_json::json!({})))
    }

    /// Add a transform to the processing queue
    ///
    /// # Arguments
    ///
    /// * `transform_id` - ID of the transform to queue
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     LambdaContext::add_to_transform_queue("my_transform").await?;
    ///     println!("Transform queued");
    ///     Ok(())
    /// }
    /// ```
    pub async fn add_to_transform_queue(transform_id: &str) -> Result<(), IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        node.add_transform_to_queue(transform_id)
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to add transform to queue: {}", e)))
    }

    /// Get transform statistics
    ///
    /// Returns statistics about transform execution and performance.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let stats = LambdaContext::get_transform_statistics().await?;
    ///     println!("Transform stats: {:?}", stats);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_transform_statistics() -> Result<Value, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        let stats = node.get_event_statistics()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get transform statistics: {}", e)))?;
        
        Ok(serde_json::to_value(stats).unwrap_or(serde_json::json!({})))
    }

    /// Get backfill status by hash
    ///
    /// # Arguments
    ///
    /// * `backfill_hash` - Hash of the backfill to query
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     if let Some(status) = LambdaContext::get_backfill_status("abc123").await? {
    ///         println!("Backfill status: {:?}", status);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_backfill_status(backfill_hash: &str) -> Result<Option<Value>, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        let db_guard = node.get_fold_db()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
        
        let backfill_info = db_guard.get_backfill_tracker().get_backfill_by_hash(backfill_hash);
        Ok(backfill_info.map(|info| serde_json::to_value(info).unwrap_or(serde_json::json!({}))))
    }

    /// Get all backfills
    ///
    /// Returns all backfills in the system.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let backfills = LambdaContext::get_all_backfills().await?;
    ///     println!("Total backfills: {}", backfills.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_all_backfills() -> Result<Vec<Value>, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        let backfills = node.get_all_backfills()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get backfills: {}", e)))?;
        
        Ok(backfills.into_iter()
            .map(|b| serde_json::to_value(b).unwrap_or(serde_json::json!({})))
            .collect())
    }

    /// Get active backfills
    ///
    /// Returns only backfills that are currently in progress.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let active = LambdaContext::get_active_backfills().await?;
    ///     println!("Active backfills: {}", active.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_active_backfills() -> Result<Vec<Value>, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        let backfills = node.get_active_backfills()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get active backfills: {}", e)))?;
        
        Ok(backfills.into_iter()
            .map(|b| serde_json::to_value(b).unwrap_or(serde_json::json!({})))
            .collect())
    }

    /// Get backfill statistics
    ///
    /// Returns aggregate statistics about all backfills.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let stats = LambdaContext::get_backfill_statistics().await?;
    ///     println!("Backfill statistics: {:?}", stats);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_backfill_statistics() -> Result<Value, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        let backfills = node.get_all_backfills()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get backfills: {}", e)))?;
        
        // Calculate statistics from backfills
        let active_count = backfills.iter()
            .filter(|b| b.status == crate::fold_db_core::infrastructure::backfill_tracker::BackfillStatus::InProgress)
            .count();
        let completed_count = backfills.iter()
            .filter(|b| b.status == crate::fold_db_core::infrastructure::backfill_tracker::BackfillStatus::Completed)
            .count();
        let failed_count = backfills.iter()
            .filter(|b| b.status == crate::fold_db_core::infrastructure::backfill_tracker::BackfillStatus::Failed)
            .count();
        
        let stats = serde_json::json!({
            "total_backfills": backfills.len(),
            "active_backfills": active_count,
            "completed_backfills": completed_count,
            "failed_backfills": failed_count,
            "total_mutations_expected": backfills.iter().map(|b| b.mutations_expected as u64).sum::<u64>(),
            "total_mutations_completed": backfills.iter().map(|b| b.mutations_completed as u64).sum::<u64>(),
            "total_mutations_failed": backfills.iter().map(|b| b.mutations_failed as u64).sum::<u64>(),
            "total_records_produced": backfills.iter().map(|b| b.records_produced).sum::<u64>(),
        });
        
        Ok(stats)
    }

    /// Get backfill information for a specific transform
    ///
    /// # Arguments
    ///
    /// * `transform_id` - ID of the transform
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     if let Some(backfill) = LambdaContext::get_backfill("my_transform").await? {
    ///         println!("Backfill info: {:?}", backfill);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_backfill(transform_id: &str) -> Result<Option<Value>, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        let backfill = node.get_backfill(transform_id)
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get backfill: {}", e)))?;
        
        Ok(backfill.map(|b| serde_json::to_value(b).unwrap_or(serde_json::json!({}))))
    }

    /// Get indexing status
    ///
    /// Returns the current status of background indexing operations.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let status = LambdaContext::get_indexing_status().await?;
    ///     println!("Indexing status: {:?}", status);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_indexing_status() -> Result<Value, IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        let status = node.get_indexing_status();
        Ok(serde_json::to_value(status).unwrap_or(serde_json::json!({})))
    }
}
