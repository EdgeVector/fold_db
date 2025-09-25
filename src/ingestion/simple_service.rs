//! Simplified ingestion service that works with DataFoldNode's existing interface

use crate::datafold_node::{DataFoldNode, OperationProcessor};
use crate::fees::SchemaPaymentConfig;
use crate::ingestion::config::AIProvider;
use crate::ingestion::core::IngestionRequest;
use crate::ingestion::mutation_generator::MutationGenerator;
use crate::ingestion::ollama_service::OllamaService;
use crate::ingestion::openrouter_service::OpenRouterService;
use crate::ingestion::schema_stripper::SchemaStripper;
use crate::ingestion::{
    AISchemaResponse, IngestionConfig, IngestionError, IngestionResponse, IngestionResult,
};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::{Mutation, Operation};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Simplified ingestion service that works with DataFoldNode
pub struct SimpleIngestionService {
    config: IngestionConfig,
    openrouter_service: Option<OpenRouterService>,
    ollama_service: Option<OllamaService>,
    schema_stripper: SchemaStripper,
    mutation_generator: MutationGenerator,
}

impl SimpleIngestionService {
    /// Create a new simple ingestion service
    pub fn new(config: IngestionConfig) -> IngestionResult<Self> {
        let openrouter_service = if config.provider == AIProvider::OpenRouter {
            Some(OpenRouterService::new(
                config.openrouter.clone(),
                config.timeout_seconds,
                config.max_retries,
            )?)
        } else {
            None
        };

        let ollama_service = if config.provider == AIProvider::Ollama {
            Some(OllamaService::new(
                config.ollama.clone(),
                config.timeout_seconds,
                config.max_retries,
            )?)
        } else {
            None
        };

        let schema_stripper = SchemaStripper::new();
        let mutation_generator = MutationGenerator::new();

        Ok(Self {
            config,
            openrouter_service,
            ollama_service,
            schema_stripper,
            mutation_generator,
        })
    }

    /// Process JSON ingestion using a DataFoldNode
    pub async fn process_json_with_node(
        &self,
        request: IngestionRequest,
        node: Arc<Mutex<DataFoldNode>>,
    ) -> IngestionResult<IngestionResponse> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Starting JSON ingestion process with DataFoldNode"
        );

        if !self.config.is_ready() {
            return Ok(IngestionResponse::failure(vec![
                "Ingestion module is not properly configured or disabled".to_string(),
            ]));
        }

        // Step 1: Validate input
        self.validate_input(&request.data)?;

        // Step 2: Get available schemas and strip them
        let available_schemas = self.get_stripped_available_schemas_from_node(node.clone()).await?;
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Retrieved {} available schemas",
            available_schemas.as_object().map(|o| o.len()).unwrap_or(0)
        );

        // Step 3: Get AI recommendation
        let ai_response = self
            .get_ai_recommendation(&request.data, &available_schemas)
            .await?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Received AI recommendation: {} existing schemas, new schema: {}",
            ai_response.existing_schemas.len(),
            ai_response.new_schemas.is_some()
        );

        // Step 4: Determine schema to use
        let schema_name = self.determine_schema_to_use(&ai_response, node.clone()).await?;
        let new_schema_created = ai_response.new_schemas.is_some();

        // Step 5: Generate mutations
        // Convert JSON data to fields and values
        let fields_and_values = if let Some(obj) = request.data.as_object() {
            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else {
            std::collections::HashMap::new()
        };
        
        let mutations = self.mutation_generator.generate_mutations(
            &schema_name,
            &std::collections::HashMap::new(), // Empty keys for now
            &fields_and_values,
            &ai_response.mutation_mappers,
            request
                .trust_distance
                .unwrap_or(self.config.default_trust_distance),
            request.pub_key.unwrap_or_else(|| "default".to_string()),
        )?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Generated {} mutations",
            mutations.len()
        );

        // Step 6: Execute mutations if requested
        let mutations_executed = if request
            .auto_execute
            .unwrap_or(self.config.auto_execute_mutations)
        {
            self.execute_mutations_with_node(&mutations, node.clone())?
        } else {
            0
        };

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Ingestion completed successfully: schema '{}', {} mutations generated, {} executed",
            schema_name,
            mutations.len(),
            mutations_executed
        );

        Ok(IngestionResponse::success(
            schema_name,
            new_schema_created,
            mutations.len(),
            mutations_executed,
        ))
    }

    /// Get AI schema recommendation
    async fn get_ai_recommendation(
        &self,
        json_data: &Value,
        available_schemas: &Value,
    ) -> IngestionResult<AISchemaResponse> {
        match self.config.provider {
            AIProvider::OpenRouter => {
                self.openrouter_service
                    .as_ref()
                    .ok_or_else(|| {
                        IngestionError::configuration_error("OpenRouter service not initialized")
                    })?
                    .get_schema_recommendation(json_data, available_schemas)
                    .await
            }
            AIProvider::Ollama => {
                self.ollama_service
                    .as_ref()
                    .ok_or_else(|| {
                        IngestionError::configuration_error("Ollama service not initialized")
                    })?
                    .get_schema_recommendation(json_data, available_schemas)
                    .await
            }
        }
    }

    /// Validate JSON input
    pub fn validate_input(&self, data: &Value) -> IngestionResult<()> {
        if data.is_null() {
            return Err(IngestionError::invalid_input("Input data cannot be null"));
        }

        if !data.is_object() && !data.is_array() {
            return Err(IngestionError::invalid_input(
                "Input data must be a JSON object or array",
            ));
        }

        Ok(())
    }

    /// Get status information
    pub fn get_status(&self) -> IngestionResult<Value> {
        let (provider_name, model) = match self.config.provider {
            AIProvider::OpenRouter => ("OpenRouter", self.config.openrouter.model.clone()),
            AIProvider::Ollama => ("Ollama", self.config.ollama.model.clone()),
        };

        Ok(serde_json::json!({
            "enabled": self.config.enabled,
            "configured": self.config.is_ready(),
            "provider": provider_name,
            "model": model,
            "auto_execute_mutations": self.config.auto_execute_mutations,
            "default_trust_distance": self.config.default_trust_distance
        }))
    }

    /// Get available schemas stripped of payment and permission data
    async fn get_stripped_available_schemas_from_node(
        &self,
        node: Arc<Mutex<DataFoldNode>>,
    ) -> IngestionResult<Value> {
        // Get all available schemas from the node through the schema manager
        let node_guard = node.lock().await;
        let db_guard = node_guard.get_fold_db().map_err(|e| {
            IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                e.to_string(),
            ))
        })?;
        
        let schema_states = db_guard.schema_manager.get_schema_states().map_err(|e| {
            IngestionError::SchemaSystemError(e)
        })?;

        let mut schemas = Vec::new();
        for schema_name in schema_states.keys() {
            if let Ok(Some(schema)) = db_guard.schema_manager.get_schema(schema_name) {
                schemas.push(schema);
            }
        }

        // Strip payment and permission data
        self.schema_stripper
            .create_ai_schema_representation(&schemas)
    }

    /// Determine which schema to use based on AI response
    async fn determine_schema_to_use(
        &self,
        ai_response: &AISchemaResponse,
        node: Arc<Mutex<DataFoldNode>>,
    ) -> IngestionResult<String> {
        // If existing schemas were recommended, use the first one
        if !ai_response.existing_schemas.is_empty() {
            let schema_name = &ai_response.existing_schemas[0];
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Using existing schema: {}",
                schema_name
            );
            return Ok(schema_name.clone());
        }

        // If a new schema was provided, create it
        if let Some(new_schema_def) = &ai_response.new_schemas {
            let schema_name = self.create_new_schema_with_node(new_schema_def, node.clone()).await?;
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Created new schema: {}",
                schema_name
            );
            return Ok(schema_name);
        }

        Err(IngestionError::ai_response_validation_error(
            "AI response contains neither existing schemas nor new schema definition",
        ))
    }

    /// Create a new schema using the DataFoldNode
    async fn create_new_schema_with_node(
        &self,
        schema_def: &Value,
        node: Arc<Mutex<DataFoldNode>>,
    ) -> IngestionResult<String> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Creating new schema from AI definition"
        );

        // Convert JSON Value back to string for SchemaCore to parse
        let json_str = serde_json::to_string(schema_def)
            .map_err(|e| IngestionError::schema_parsing_error(format!("Failed to serialize schema definition: {}", e)))?;

        // Extract schema name from the definition
        let schema_name = schema_def.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IngestionError::schema_parsing_error("Schema definition must have a 'name' field"))?
            .to_string();

        // Load the schema using the node through the schema manager
        let node_guard = node.lock().await;
        let db_guard = node_guard.get_fold_db().map_err(|e| {
            IngestionError::SchemaCreationError(e.to_string())
        })?;
        
        db_guard.schema_manager.load_schema_from_json(&json_str)
            .map_err(|e| IngestionError::SchemaCreationError(e.to_string()))?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "New schema '{}' created and approved",
            schema_name
        );
        Ok(schema_name)
    }



    /// Execute mutations using the OperationProcessor
    fn execute_mutations_with_node(
        &self,
        mutations: &[Mutation],
        node: Arc<Mutex<DataFoldNode>>,
    ) -> IngestionResult<usize> {
        let mut executed_count = 0;
        let processor = OperationProcessor::new(node);

        for mutation in mutations {
            // Convert mutation to operation with correct structure
            let operation = Operation::Mutation {
                schema: mutation.schema_name.clone(),
                fields_and_values: mutation.fields_and_values.clone(),
                key_config: mutation.key_config.clone(),
                mutation_type: mutation.mutation_type.clone(),
            };

            match processor.execute_sync(operation) {
                Ok(_) => {
                    executed_count += 1;
                    log_feature!(
                        LogFeature::Ingestion,
                        info,
                        "Successfully executed mutation for schema '{}'",
                        mutation.schema_name
                    );
                }
                Err(e) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        error,
                        "Failed to execute mutation for schema '{}': {}",
                        mutation.schema_name,
                        e
                    );
                    // Continue with other mutations even if one fails
                }
            }
        }

        Ok(executed_count)
    }
}
