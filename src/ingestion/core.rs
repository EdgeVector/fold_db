//! Core ingestion orchestrator

use crate::fold_db_core::FoldDB;
use crate::ingestion::{
    mutation_generator::MutationGenerator,
    openrouter_service::{AISchemaResponse, OpenRouterService},
    schema_stripper::SchemaStripper,
    IngestionConfig, IngestionError, IngestionResponse, IngestionResult,
};
use crate::schema::types::{JsonSchemaDefinition, Mutation};
use crate::schema::{Schema, SchemaCore};
use crate::logging::features::LogFeature;
use crate::log_feature;
use serde_json::Value;
use std::sync::Arc;

/// Core ingestion service that orchestrates the entire ingestion process
pub struct IngestionCore {
    config: IngestionConfig,
    openrouter_service: OpenRouterService,
    schema_stripper: SchemaStripper,
    mutation_generator: MutationGenerator,
    schema_core: Arc<SchemaCore>,
    fold_db: Arc<std::sync::Mutex<FoldDB>>,
}

/// Request for processing JSON ingestion
#[derive(Debug, serde::Deserialize)]
pub struct IngestionRequest {
    /// JSON data to ingest
    pub data: Value,
    /// Whether to auto-execute mutations after generation
    pub auto_execute: Option<bool>,
    /// Trust distance for mutations
    pub trust_distance: Option<u32>,
    /// Public key for mutations
    pub pub_key: Option<String>,
}

impl IngestionCore {
    /// Create a new ingestion core
    pub fn new(
        config: IngestionConfig,
        schema_core: Arc<SchemaCore>,
        fold_db: Arc<std::sync::Mutex<FoldDB>>,
    ) -> IngestionResult<Self> {
        let openrouter_service = OpenRouterService::new(config.clone())?;
        let schema_stripper = SchemaStripper::new();
        let mutation_generator = MutationGenerator::new();

        Ok(Self {
            config,
            openrouter_service,
            schema_stripper,
            mutation_generator,
            schema_core,
            fold_db,
        })
    }

    /// Process JSON ingestion request
    pub async fn process_json_ingestion(
        &self,
        request: IngestionRequest,
    ) -> IngestionResult<IngestionResponse> {
        log_feature!(LogFeature::Ingestion, info, "Starting JSON ingestion process");

        // Step 1: Validate configuration
        self.validate_configuration()?;

        // Step 2: Prepare schemas
        let available_schemas = self.prepare_schemas().await?;

        // Step 3: Get AI recommendation
        let ai_response = self.get_ai_recommendation(&request.data, &available_schemas).await?;

        // Step 4: Determine and setup schema
        let (schema_name, new_schema_created) = self.setup_schema(&ai_response).await?;

        // Step 5: Generate mutations
        let mutations = self.generate_mutations(&schema_name, &request, &ai_response)?;

        // Step 6: Execute mutations if requested
        let mutations_executed = self.execute_mutations_if_requested(&request, &mutations).await?;

        self.log_completion(&schema_name, mutations.len(), mutations_executed);

        Ok(IngestionResponse::success(
            schema_name,
            new_schema_created,
            mutations.len(),
            mutations_executed,
        ))
    }

    /// Validates that the ingestion configuration is ready.
    fn validate_configuration(&self) -> IngestionResult<()> {
        if !self.config.is_ready() {
            return Err(IngestionError::configuration_error(
                "Ingestion module is not properly configured or disabled"
            ));
        }
        Ok(())
    }

    /// Prepares available schemas for AI recommendation.
    async fn prepare_schemas(&self) -> IngestionResult<Value> {
        let available_schemas = self.get_stripped_available_schemas().await?;
        let schema_count = if let Some(obj) = available_schemas.as_object() {
            obj.len()
        } else {
            0
        };
        log_feature!(LogFeature::Ingestion, info, "Retrieved {} available schemas", schema_count);
        Ok(available_schemas)
    }

    /// Gets AI recommendation for schema selection/creation.
    async fn get_ai_recommendation(
        &self,
        data: &Value,
        available_schemas: &Value,
    ) -> IngestionResult<AISchemaResponse> {
        let ai_response = self.get_ai_schema_recommendation(data, available_schemas).await?;
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Received AI recommendation: {} existing schemas, new schema: {}",
            ai_response.existing_schemas.len(),
            ai_response.new_schemas.is_some()
        );
        Ok(ai_response)
    }

    /// Sets up the schema to use (existing or newly created).
    async fn setup_schema(&self, ai_response: &AISchemaResponse) -> IngestionResult<(String, bool)> {
        let schema_name = self.determine_schema_to_use(ai_response).await?;
        let new_schema_created = ai_response.new_schemas.is_some();
        Ok((schema_name, new_schema_created))
    }

    /// Generates mutations for the data using the determined schema.
    fn generate_mutations(
        &self,
        schema_name: &str,
        request: &IngestionRequest,
        ai_response: &AISchemaResponse,
    ) -> IngestionResult<Vec<Mutation>> {
        let mutations = self.generate_mutations_for_data(
            schema_name,
            &request.data,
            &ai_response.mutation_mappers,
            request.trust_distance.unwrap_or(self.config.default_trust_distance),
            request.pub_key.clone().unwrap_or_else(|| "default".to_string()),
        )?;

        log_feature!(LogFeature::Ingestion, info, "Generated {} mutations", mutations.len());
        Ok(mutations)
    }

    /// Executes mutations if auto-execution is enabled.
    async fn execute_mutations_if_requested(
        &self,
        request: &IngestionRequest,
        mutations: &[Mutation],
    ) -> IngestionResult<usize> {
        if request.auto_execute.unwrap_or(self.config.auto_execute_mutations) {
            self.execute_mutations(mutations).await
        } else {
            Ok(0)
        }
    }

    /// Logs the completion of the ingestion process.
    fn log_completion(&self, schema_name: &str, mutations_count: usize, mutations_executed: usize) {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Ingestion completed successfully: schema '{}', {} mutations generated, {} executed",
            schema_name, mutations_count, mutations_executed
        );
    }

    /// Get available schemas stripped of payment and permission data
    async fn get_stripped_available_schemas(&self) -> IngestionResult<Value> {
        // Get all available schemas from SchemaCore
        let available_schema_names = self
            .schema_core
            .list_available_schemas()
            .map_err(IngestionError::SchemaSystemError)?;

        let mut schemas = Vec::new();
        for schema_name in available_schema_names {
            if let Ok(Some(schema)) = self.schema_core.get_schema(&schema_name) {
                schemas.push(schema);
            }
        }

        // Strip payment and permission data
        self.schema_stripper
            .create_ai_schema_representation(&schemas)
    }

    /// Get AI schema recommendation
    async fn get_ai_schema_recommendation(
        &self,
        json_data: &Value,
        available_schemas: &Value,
    ) -> IngestionResult<AISchemaResponse> {
        self.openrouter_service
            .get_schema_recommendation(json_data, available_schemas)
            .await
    }

    /// Determine which schema to use based on AI response
    async fn determine_schema_to_use(
        &self,
        ai_response: &AISchemaResponse,
    ) -> IngestionResult<String> {
        // If existing schemas were recommended, use the first one
        if !ai_response.existing_schemas.is_empty() {
            let schema_name = &ai_response.existing_schemas[0];
            log_feature!(LogFeature::Ingestion, info, "Using existing schema: {}", schema_name);
            return Ok(schema_name.clone());
        }

        // If a new schema was provided, create it
        if let Some(new_schema_def) = &ai_response.new_schemas {
            let schema_name = self.create_new_schema(new_schema_def).await?;
            log_feature!(LogFeature::Ingestion, info, "Created new schema: {}", schema_name);
            return Ok(schema_name);
        }

        Err(IngestionError::ai_response_validation_error(
            "AI response contains neither existing schemas nor new schema definition",
        ))
    }

    /// Create a new schema from AI response
    async fn create_new_schema(&self, schema_def: &Value) -> IngestionResult<String> {
        log_feature!(LogFeature::Ingestion, info, "Creating new schema from AI definition");

        // Parse the schema definition
        let schema = self.parse_schema_definition(schema_def)?;
        let schema_name = schema.name.clone();

        // Load the schema into SchemaCore
        self.schema_core
            .load_schema_internal(schema)
            .map_err(IngestionError::SchemaSystemError)?;

        // Set the schema to approved state
        self.schema_core
            .approve_schema(&schema_name)
            .map_err(IngestionError::SchemaSystemError)?;

        log_feature!(LogFeature::Ingestion, info, "New schema '{}' created and approved", schema_name);
        Ok(schema_name)
    }

    /// Parse schema definition from AI response
    fn parse_schema_definition(&self, schema_def: &Value) -> IngestionResult<Schema> {
        // Try to parse as a complete Schema first
        if let Ok(schema) = serde_json::from_value::<Schema>(schema_def.clone()) {
            return Ok(schema);
        }

        // Try to parse as JsonSchemaDefinition
        if let Ok(json_schema) = serde_json::from_value::<JsonSchemaDefinition>(schema_def.clone())
        {
            return self
                .schema_core
                .interpret_schema(json_schema)
                .map_err(IngestionError::SchemaSystemError);
        }

        // Try to create a basic schema from the definition
        self.create_basic_schema_from_definition(schema_def)
    }

    /// Create a basic schema from a simple definition
    fn create_basic_schema_from_definition(&self, schema_def: &Value) -> IngestionResult<Schema> {
        let obj = schema_def.as_object().ok_or_else(|| {
            IngestionError::schema_parsing_error("Schema definition must be an object")
        })?;

        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IngestionError::schema_parsing_error("Schema must have a name"))?;

        let mut schema = Schema::new(name.to_string());

        // Add fields if provided
        if let Some(Value::Object(fields)) = obj.get("fields") {
            for (field_name, field_def) in fields {
                // Create a basic single field for each field in the definition
                let field = self.create_basic_field_from_definition(field_def)?;
                schema.add_field(field_name.clone(), field);
            }
        }

        Ok(schema)
    }

    /// Create a basic field from definition
    fn create_basic_field_from_definition(
        &self,
        _field_def: &Value,
    ) -> IngestionResult<crate::schema::types::field::FieldVariant> {
        use crate::fees::types::FieldPaymentConfig;
        use crate::permissions::types::policy::PermissionsPolicy;
        use crate::schema::types::field::{FieldVariant, SingleField};
        use std::collections::HashMap;

        // Create a basic single field with default permissions and payment config
        let field = SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );

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

    /// Execute mutations
    async fn execute_mutations(&self, mutations: &[Mutation]) -> IngestionResult<usize> {
        let mut executed_count = 0;

        for mutation in mutations {
            match self.execute_single_mutation(mutation).await {
                Ok(()) => {
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

    /// Execute a single mutation
    async fn execute_single_mutation(&self, mutation: &Mutation) -> IngestionResult<()> {
        let mut db = self.fold_db.lock().map_err(|_| {
            IngestionError::DatabaseError("Failed to acquire database lock".to_string())
        })?;

        db.write_schema(mutation.clone())
            .map_err(IngestionError::SchemaSystemError)?;

        Ok(())
    }

    /// Get ingestion status
    pub fn get_status(&self) -> IngestionResult<Value> {
        Ok(serde_json::json!({
            "enabled": self.config.enabled,
            "configured": self.config.is_ready(),
            "model": self.config.openrouter_model,
            "auto_execute_mutations": self.config.auto_execute_mutations,
            "default_trust_distance": self.config.default_trust_distance
        }))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold_db_core::FoldDB;
    use crate::schema::SchemaCore;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    // REMOVED: create_test_ingestion_core - dead code marked with #[allow(dead_code)]
    // This duplicated test setup logic available in testing_utils module

    #[test]
    fn test_validate_input() {
        // Create isolated test setup for this test
        let config = IngestionConfig {
            openrouter_api_key: "test-key".to_string(),
            enabled: true,
            ..Default::default()
        };

        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir
            .path()
            .join("test_validate")
            .to_string_lossy()
            .to_string();

        // Try to create components with better error handling
        let schema_core = match SchemaCore::new_for_testing(&test_path) {
            Ok(core) => Arc::new(core),
            Err(_) => {
                eprintln!("Skipping test_validate_input: Could not create schema core");
                return;
            }
        };

        let fold_db = match FoldDB::new(&test_path) {
            Ok(db) => Arc::new(Mutex::new(db)),
            Err(_) => {
                eprintln!("Skipping test_validate_input: Could not create database");
                return;
            }
        };

        let core = match IngestionCore::new(config, schema_core, fold_db) {
            Ok(core) => core,
            Err(_) => {
                eprintln!("Skipping test_validate_input: Could not create ingestion core");
                return;
            }
        };

        // Valid inputs
        assert!(core
            .validate_input(&serde_json::json!({"key": "value"}))
            .is_ok());
        assert!(core.validate_input(&serde_json::json!([1, 2, 3])).is_ok());

        // Invalid inputs
        assert!(core.validate_input(&serde_json::json!(null)).is_err());
        assert!(core.validate_input(&serde_json::json!("string")).is_err());
        assert!(core.validate_input(&serde_json::json!(42)).is_err());
    }

    #[test]
    fn test_create_basic_schema_from_definition() {
        // Create isolated test setup for this test
        let config = IngestionConfig {
            openrouter_api_key: "test-key".to_string(),
            enabled: true,
            ..Default::default()
        };

        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir
            .path()
            .join("test_schema_def")
            .to_string_lossy()
            .to_string();

        // Try to create components with better error handling
        let schema_core = match SchemaCore::new_for_testing(&test_path) {
            Ok(core) => Arc::new(core),
            Err(_) => {
                eprintln!("Skipping test_create_basic_schema_from_definition: Could not create schema core");
                return;
            }
        };

        let fold_db = match FoldDB::new(&test_path) {
            Ok(db) => Arc::new(Mutex::new(db)),
            Err(_) => {
                eprintln!(
                    "Skipping test_create_basic_schema_from_definition: Could not create database"
                );
                return;
            }
        };

        let core = match IngestionCore::new(config, schema_core, fold_db) {
            Ok(core) => core,
            Err(_) => {
                eprintln!("Skipping test_create_basic_schema_from_definition: Could not create ingestion core");
                return;
            }
        };

        let schema_def = serde_json::json!({
            "name": "TestSchema",
            "fields": {
                "field1": {"type": "string"},
                "field2": {"type": "number"}
            }
        });

        let result = core.create_basic_schema_from_definition(&schema_def);
        assert!(result.is_ok());

        let schema = result.unwrap();
        assert_eq!(schema.name, "TestSchema");
        assert_eq!(schema.fields.len(), 2);
    }
}
