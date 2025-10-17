//! Simplified ingestion service that works with DataFoldNode's existing interface

use crate::datafold_node::{DataFoldNode, OperationProcessor, SchemaServiceClient};
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
        let available_schemas = self
            .get_stripped_available_schemas_from_node(node.clone())
            .await?;
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
        let schema_name = self
            .determine_schema_to_use(&ai_response, node.clone())
            .await?;
        let new_schema_created = ai_response.new_schemas.is_some();

        // Step 5: Generate mutations
        // Handle both single objects and arrays of objects
        let mutations = if let Some(array) = request.data.as_array() {
            // Generate a mutation for each element in the array
            log_feature!(
                LogFeature::Ingestion,
                info,
                "JSON data is an array with {} items, generating mutation for each",
                array.len()
            );

            let mut all_mutations = Vec::new();
            for (idx, item) in array.iter().enumerate() {
                let fields_and_values = if let Some(obj) = item.as_object() {
                    obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                } else {
                    log_feature!(
                        LogFeature::Ingestion,
                        warn,
                        "Array item {} is not an object, skipping",
                        idx
                    );
                    continue;
                };

                let mutations = self.mutation_generator.generate_mutations(
                    &schema_name,
                    &std::collections::HashMap::new(),
                    &fields_and_values,
                    &ai_response.mutation_mappers,
                    request
                        .trust_distance
                        .unwrap_or(self.config.default_trust_distance),
                    request
                        .pub_key
                        .clone()
                        .unwrap_or_else(|| "default".to_string()),
                )?;

                all_mutations.extend(mutations);
            }

            all_mutations
        } else {
            // Handle single object
            let fields_and_values = if let Some(obj) = request.data.as_object() {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                std::collections::HashMap::new()
            };

            self.mutation_generator.generate_mutations(
                &schema_name,
                &std::collections::HashMap::new(),
                &fields_and_values,
                &ai_response.mutation_mappers,
                request
                    .trust_distance
                    .unwrap_or(self.config.default_trust_distance),
                request.pub_key.unwrap_or_else(|| "default".to_string()),
            )?
        };

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
            self.execute_mutations_with_node(&mutations, node.clone())
                .await?
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

        let schema_states = db_guard
            .schema_manager
            .get_schema_states()
            .map_err(IngestionError::SchemaSystemError)?;

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
            let schema_name = self
                .create_new_schema_with_node(new_schema_def, node.clone())
                .await?;
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

        let schema_service_url = {
            let node_guard = node.lock().await;
            node_guard.schema_service_url().ok_or_else(|| {
                IngestionError::SchemaCreationError(
                    "Schema service URL is not configured for the node".to_string(),
                )
            })?
        };

        if schema_service_url.starts_with("test://") || schema_service_url.starts_with("mock://") {
            return Err(IngestionError::SchemaCreationError(
                "Schema service URL must point to an accessible HTTP endpoint".to_string(),
            ));
        }

        // Deserialize Value to Schema
        let schema: crate::schema::types::Schema = serde_json::from_value(schema_def.clone())
            .map_err(|error| {
                IngestionError::SchemaCreationError(format!(
                    "Failed to deserialize schema from AI response: {}",
                    error
                ))
            })?;

        let schema_response = SchemaServiceClient::new(&schema_service_url)
            .add_schema(&schema)
            .await
            .map_err(|error| {
                IngestionError::SchemaCreationError(format!(
                    "Failed to create schema via schema service: {}",
                    error
                ))
            })?;

        let json_str = serde_json::to_string(&schema_response).map_err(|error| {
            IngestionError::schema_parsing_error(format!(
                "Failed to serialize schema definition: {}",
                error
            ))
        })?;

        let schema_manager = {
            let node_guard = node.lock().await;
            let db_guard = node_guard
                .get_fold_db()
                .map_err(|error| IngestionError::SchemaCreationError(error.to_string()))?;
            let manager = db_guard.schema_manager.clone();
            drop(db_guard);
            manager
        };

        schema_manager
            .load_schema_from_json(&json_str)
            .map_err(|error| IngestionError::SchemaCreationError(error.to_string()))?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "New schema '{}' created and approved",
            schema_response.name
        );
        Ok(schema_response.name)
    }

    /// Execute mutations using the OperationProcessor
    async fn execute_mutations_with_node(
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
                key_value: mutation.key_value.clone(),
                mutation_type: mutation.mutation_type.clone(),
            };

            // Execute mutation asynchronously
            let exec_result: Result<serde_json::Value, IngestionError> = match operation {
                Operation::Mutation {
                    schema,
                    fields_and_values,
                    key_value,
                    mutation_type,
                } => processor
                    .execute_mutation(schema, fields_and_values, key_value, mutation_type)
                    .await
                    .map_err(|e| {
                        IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                            e.to_string(),
                        ))
                    }),
            };

            match exec_result {
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
