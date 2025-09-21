//! Simplified ingestion service that works with DataFoldNode's existing interface

use crate::datafold_node::DataFoldNode;
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
        node: &mut DataFoldNode,
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
        let available_schemas = self.get_stripped_available_schemas_from_node(node)?;
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
        let schema_name = self.determine_schema_to_use(&ai_response, node)?;
        let new_schema_created = ai_response.new_schemas.is_some();

        // Step 5: Generate mutations
        let mutations = self.generate_mutations_for_data(
            &schema_name,
            &request.data,
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
            self.execute_mutations_with_node(&mutations, node)?
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
    fn get_stripped_available_schemas_from_node(
        &self,
        node: &DataFoldNode,
    ) -> IngestionResult<Value> {
        // Get all available schemas from the node
        let schema_states = node.list_schemas_with_state().map_err(|e| {
            IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                e.to_string(),
            ))
        })?;

        let mut schemas = Vec::new();
        for schema_name in schema_states.keys() {
            if let Ok(Some(schema)) = node.get_schema(schema_name) {
                schemas.push(schema);
            }
        }

        // Strip payment and permission data
        self.schema_stripper
            .create_ai_schema_representation(&schemas)
    }

    /// Determine which schema to use based on AI response
    fn determine_schema_to_use(
        &self,
        ai_response: &AISchemaResponse,
        node: &mut DataFoldNode,
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
            let schema_name = self.create_new_schema_with_node(new_schema_def, node)?;
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
    fn create_new_schema_with_node(
        &self,
        schema_def: &Value,
        node: &mut DataFoldNode,
    ) -> IngestionResult<String> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Creating new schema from AI definition"
        );

        // Parse the schema definition
        let schema = self.parse_schema_definition(schema_def)?;
        let schema_name = schema.name.clone();

        // Load the schema using the node (this adds it as available and approves it)
        node.load_schema(schema)
            .map_err(|e| IngestionError::SchemaCreationError(e.to_string()))?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "New schema '{}' created and approved",
            schema_name
        );
        Ok(schema_name)
    }

    /// Parse schema definition from AI response
    fn parse_schema_definition(
        &self,
        schema_def: &Value,
    ) -> IngestionResult<crate::schema::Schema> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Parsing schema definition: {}",
            serde_json::to_string_pretty(schema_def).unwrap_or_else(|_| "Invalid JSON".to_string())
        );

        // Try to parse as a complete Schema first
        if let Ok(schema) = serde_json::from_value::<crate::schema::Schema>(schema_def.clone()) {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Successfully parsed as complete Schema"
            );
            return Ok(schema);
        }

        // Check if the schema is wrapped in an object with schema name as key
        if let Some(obj) = schema_def.as_object() {
            if obj.len() == 1 {
                let (schema_name, schema_content) = obj.iter().next().unwrap();
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "Found wrapped schema with name: {}",
                    schema_name
                );

                // Try to parse the wrapped content
                if let Ok(schema) =
                    serde_json::from_value::<crate::schema::Schema>(schema_content.clone())
                {
                    log_feature!(
                        LogFeature::Ingestion,
                        info,
                        "Successfully parsed wrapped schema"
                    );
                    return Ok(schema);
                }

                // If that fails, try to create a basic schema from the wrapped content
                return self
                    .create_basic_schema_from_wrapped_definition(schema_name, schema_content);
            }
        }

        // Create a basic schema from the definition
        self.create_basic_schema_from_definition(schema_def)
    }

    /// Create a basic schema from a wrapped definition (AI format: {"SchemaName": {...}})
    fn create_basic_schema_from_wrapped_definition(
        &self,
        schema_name: &str,
        schema_content: &Value,
    ) -> IngestionResult<crate::schema::Schema> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Creating basic schema from wrapped definition for: {}",
            schema_name
        );

        let obj = schema_content.as_object().ok_or_else(|| {
            IngestionError::schema_parsing_error("Wrapped schema content must be an object")
        })?;

        let mut schema = crate::schema::Schema::new(schema_name.to_string());

        // Add fields if provided
        if let Some(Value::Object(fields)) = obj.get("fields") {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Processing {} fields for schema {}",
                fields.len(),
                schema_name
            );
            for (field_name, _field_def) in fields {
                // Create a basic field for each field in the definition
                let field = self.create_basic_field()?;
                schema.add_field(field_name.clone(), field);
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "Added field '{}' to schema '{}'",
                    field_name,
                    schema_name
                );
            }
        }

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Successfully created schema '{}' with {} fields",
            schema_name,
            schema.fields.len()
        );
        Ok(schema)
    }

    /// Create a basic schema from a simple definition
    fn create_basic_schema_from_definition(
        &self,
        schema_def: &Value,
    ) -> IngestionResult<crate::schema::Schema> {
        let obj = schema_def.as_object().ok_or_else(|| {
            IngestionError::schema_parsing_error("Schema definition must be an object")
        })?;

        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IngestionError::schema_parsing_error("Schema must have a name"))?;

        let mut schema = crate::schema::Schema::new(name.to_string());

        // Add fields if provided
        if let Some(Value::Object(fields)) = obj.get("fields") {
            for (field_name, _field_def) in fields {
                // Create a basic field for each field in the definition
                let field = self.create_basic_field()?;
                schema.add_field(field_name.clone(), field);
            }
        }

        Ok(schema)
    }

    /// Create a basic field
    fn create_basic_field(&self) -> IngestionResult<crate::schema::types::field::FieldVariant> {
        use crate::fees::types::FieldPaymentConfig;
        use crate::permissions::types::policy::{PermissionsPolicy, TrustDistance};
        use crate::schema::types::field::{FieldVariant, SingleField};
        use std::collections::HashMap;

        // Create a basic single field with open read permissions and restricted write permissions
        let permissions = PermissionsPolicy::new(
            TrustDistance::NoRequirement, // Allow anyone to read
            TrustDistance::Distance(0),   // Only trust distance 0 can write
        );

        let field = SingleField::new(permissions, FieldPaymentConfig::default(), HashMap::new());

        Ok(FieldVariant::Single(field))
    }

    /// Generate mutations for the data
    fn generate_mutations_for_data(
        &self,
        schema_name: &str,
        json_data: &Value,
        mutation_mappers: &std::collections::HashMap<String, String>,
        trust_distance: u32,
        pub_key: String,
    ) -> IngestionResult<Vec<Mutation>> {
        self.mutation_generator.generate_mutations(
            schema_name,
            json_data,
            mutation_mappers,
            trust_distance,
            pub_key,
        )
    }

    /// Execute mutations using the DataFoldNode
    fn execute_mutations_with_node(
        &self,
        mutations: &[Mutation],
        node: &mut DataFoldNode,
    ) -> IngestionResult<usize> {
        let mut executed_count = 0;

        for mutation in mutations {
            // Convert mutation to operation
            let operation = Operation::Mutation {
                schema: mutation.schema_name.clone(),
                data: serde_json::to_value(&mutation.fields_and_values)
                    .map_err(|e| IngestionError::MutationGenerationError(e.to_string()))?,
                mutation_type: mutation.mutation_type.clone(),
            };

            match node.execute_operation(operation) {
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
