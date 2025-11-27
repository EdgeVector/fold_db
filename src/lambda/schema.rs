//! Schema management operations for Lambda context

use crate::ingestion::IngestionError;

use super::context::LambdaContext;

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
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        let db_guard = node.get_fold_db()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
        
        db_guard.schema_manager.get_schemas_with_states()
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
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        let db_guard = node.get_fold_db()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
        
        let schema = db_guard.schema_manager.get_schema(schema_name)
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get schema: {}", e)))?;
        
        if let Some(schema) = schema {
            let states = db_guard.schema_manager.get_schema_states()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to get schema states: {}", e)))?;
            let state = states.get(schema_name).copied().unwrap_or_default();
            Ok(Some(crate::schema::SchemaWithState::new(schema, state)))
        } else {
            Ok(None)
        }
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
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        let db_guard = node.get_fold_db()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
        
        db_guard.schema_manager.block_schema(schema_name)
            .await
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
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        
        // Fetch schemas from schema service
        let schemas = node.fetch_available_schemas().await
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to fetch schemas: {}", e)))?;
        
        let schema_count = schemas.len();
        drop(node); // Release lock before processing
        
        // Load each schema into the local database
        let mut loaded_count = 0;
        let mut failed_schemas = Vec::new();
        
        for schema in schemas {
            let schema_name = schema.name.clone();
            let node = ctx.node.lock().await;
            let db_guard = node.get_fold_db()
                .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
            
            match db_guard.schema_manager.load_schema_internal(schema).await {
                Ok(_) => {
                    loaded_count += 1;
                    log::debug!("Loaded schema: {}", schema_name);
                }
                Err(e) => {
                    log::error!("Failed to load schema {}: {}", schema_name, e);
                    failed_schemas.push(schema_name);
                }
            }
            drop(db_guard);
            drop(node);
        }
        
        log::info!("Loaded {} of {} schemas from schema service", loaded_count, schema_count);
        Ok((schema_count, loaded_count, failed_schemas))
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
    pub async fn approve_schema(schema_name: &str) -> Result<(), IngestionError> {
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        let db_guard = node.get_fold_db()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
        
        db_guard.schema_manager.approve(schema_name)
            .await
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
        let ctx = Self::get()?;
        let node = ctx.node.lock().await;
        let db_guard = node.get_fold_db()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to access database: {}", e)))?;
        
        let states = db_guard.schema_manager.get_schema_states()
            .map_err(|e| IngestionError::InvalidInput(format!("Failed to get schema states: {}", e)))?;
        
        Ok(states.get(schema_name).copied())
    }
}
