//! Schema management operations for Lambda context

use crate::ingestion::IngestionError;

use super::context::LambdaContext;
use crate::datafold_node::OperationProcessor;
use crate::error::FoldDbError;

impl LambdaContext {
    /// List all schemas with their states
    ///
    /// Returns schemas along with their approval/pending states.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let schemas = LambdaContext::list_schemas().await?;
    ///     
    ///     for schema in schemas {
    ///         println!("Schema: {} - State: {:?}", schema.schema.name, schema.state);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn list_schemas() -> Result<Vec<crate::schema::SchemaWithState>, IngestionError> {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());
        
        processor.list_schemas().await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to list schemas: {}", e)))
    }

    /// Get a specific schema by name
    ///
    /// Returns the schema with its state if it exists.
    ///
    /// # Arguments
    ///
    /// * `schema_name` - Name of the schema to retrieve
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     if let Some(schema) = LambdaContext::get_schema("users").await? {
    ///         println!("Schema: {} - State: {:?}", schema.schema.name, schema.state);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_schema(schema_name: &str) -> Result<Option<crate::schema::SchemaWithState>, IngestionError> {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());
        
        processor.get_schema(schema_name).await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get schema: {}", e)))
    }

    /// Block a schema from queries and mutations
    ///
    /// # Arguments
    ///
    /// * `schema_name` - Name of the schema to block
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     LambdaContext::block_schema("old_schema").await?;
    ///     println!("Schema blocked");
    ///     Ok(())
    /// }
    /// ```
    pub async fn block_schema(schema_name: &str) -> Result<(), IngestionError> {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());
        
        processor.block_schema(schema_name).await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to block schema: {}", e)))
    }

    /// Load schemas from the schema service
    ///
    /// Fetches available schemas from the configured schema service and loads them into the local database.
    ///
    /// # Returns
    ///
    /// Returns a tuple of (schemas_fetched, schemas_loaded, failed_schemas)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     let (fetched, loaded, failed) = LambdaContext::load_schemas().await?;
    ///     println!("Fetched {} schemas, loaded {} successfully", fetched, loaded);
    ///     Ok(())
    /// }
    /// ```
    pub async fn load_schemas() -> Result<(usize, usize, Vec<String>), IngestionError> {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());
        
        processor.load_schemas().await
             .map_err(|e| IngestionError::InvalidInput(format!("Failed to load schemas: {}", e)))
    }

    /// Approve a schema
    ///
    /// Approves a schema if it's not already approved (idempotent).
    ///
    /// # Arguments
    ///
    /// * `schema_name` - Name of the schema to approve
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     LambdaContext::approve_schema("users").await?;
    ///     println!("Schema approved");
    ///     Ok(())
    /// }
    /// ```
    pub async fn approve_schema(schema_name: &str) -> Result<Option<String>, IngestionError> {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());
        
        processor.approve_schema(schema_name).await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to approve schema: {}", e)))
    }

    /// Get the state of a schema
    ///
    /// # Arguments
    ///
    /// * `schema_name` - Name of the schema
    ///
    /// # Returns
    ///
    /// Returns `Some(SchemaState)` if the schema exists, or `None` if not found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     if let Some(state) = LambdaContext::get_schema_state("users").await? {
    ///         println!("Schema state: {:?}", state);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_schema_state(schema_name: &str) -> Result<Option<crate::schema::SchemaState>, IngestionError> {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());
        
        if let Some(schema_with_state) = processor.get_schema(schema_name).await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get schema state: {}", e)))? {
             Ok(Some(schema_with_state.state))
        } else {
             Ok(None)
        }
    }

    /// Get backfill status by hash
    ///
    /// # Arguments
    ///
    /// * `backfill_hash` - The hash of the backfill to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Some(BackfillInfo)` if found, or `None` if not found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use datafold::lambda::LambdaContext;
    ///
    /// async fn handler() -> Result<(), Box<dyn std::error::Error>> {
    ///     if let Some(info) = LambdaContext::get_backfill_status("abc123hash").await? {
    ///         println!("Backfill status: {:?}", info.status);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_backfill_status(
        backfill_hash: &str,
    ) -> Result<Option<crate::fold_db_core::infrastructure::backfill_tracker::BackfillInfo>, IngestionError> {
        let node_mutex = Self::node().await?;
        let node_guard = node_mutex.lock().await;
        let processor = OperationProcessor::new(node_guard.clone());
        
        processor.get_backfill(backfill_hash).await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get backfill status: {}", e)))
    }
}
