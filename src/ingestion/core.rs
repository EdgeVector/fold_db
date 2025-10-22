//! Core ingestion orchestrator

use crate::datafold_node::SchemaServiceClient;
use crate::fold_db_core::FoldDB;
use crate::ingestion::{
    config::AIProvider, mutation_generator::MutationGenerator, ollama_service::OllamaService,
    openrouter_service::OpenRouterService, AISchemaResponse,
    IngestionConfig, IngestionError, IngestionResponse, IngestionResult,
    IngestionStatus, SimplifiedSchema, SimplifiedSchemaMap,
};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::Mutation;
use crate::schema::SchemaCore;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use utoipa::ToSchema;

/// Core ingestion service that orchestrates the entire ingestion process
/// TODO: Ingestion needs to be able to create new schemas and persist them to the 'available_schemas' folder.
pub struct IngestionCore {
    config: IngestionConfig,
    openrouter_service: Option<OpenRouterService>,
    ollama_service: Option<OllamaService>,
    mutation_generator: MutationGenerator,
    schema_core: Arc<SchemaCore>,
    fold_db: Arc<Mutex<FoldDB>>,
    schema_service_client: SchemaServiceClient,
}

/// Request for processing JSON ingestion
#[derive(Debug, Deserialize, ToSchema)]
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
        fold_db: Arc<Mutex<FoldDB>>,
        schema_service_client: SchemaServiceClient,
    ) -> IngestionResult<Self> {
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

        let mutation_generator = MutationGenerator::new();

        Ok(Self {
            config,
            openrouter_service,
            ollama_service,
            mutation_generator,
            schema_core,
            fold_db,
            schema_service_client,
        })
    }

    /// Process JSON ingestion request
    pub async fn process_json_ingestion(
        &self,
        request: IngestionRequest,
    ) -> IngestionResult<IngestionResponse> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Starting JSON ingestion process"
        );

        // Step 1: Validate configuration
        self.validate_configuration()?;

        // Step 2: Prepare schemas
        let available_schemas = self.prepare_schemas().await?;

        // Step 3: Get AI recommendation
        let ai_response = self
            .get_ai_recommendation(&request.data, &available_schemas)
            .await?;

        // Step 4: Determine and setup schema
        let (schema_name, new_schema_created, mutation_mappers) = self.setup_schema(&ai_response, &request.data).await?;

        // Step 5: Generate mutations using the returned mutation_mappers (which may have been updated by schema service)
        let mutations = self.generate_mutations_for_data(
            &schema_name,
            &request.data,
            &mutation_mappers,
            request.trust_distance.unwrap_or(self.config.default_trust_distance),
            request.pub_key.clone().unwrap_or_else(|| "default".to_string()),
        )?;

        // Step 6: Execute mutations if requested
        let mutations_executed = self
            .execute_mutations_if_requested(&request, &mutations)
            .await?;

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
                "Ingestion module is not properly configured or disabled",
            ));
        }
        Ok(())
    }

    /// Prepares available schemas for AI recommendation.
    async fn prepare_schemas(&self) -> IngestionResult<SimplifiedSchemaMap> {
        let available_schemas = self.get_stripped_available_schemas().await?;
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Retrieved {} available schemas",
            available_schemas.len()
        );
        Ok(available_schemas)
    }

    /// Gets AI recommendation for schema selection/creation.
    async fn get_ai_recommendation(
        &self,
        data: &Value,
        available_schemas: &SimplifiedSchemaMap,
    ) -> IngestionResult<AISchemaResponse> {
        let schemas_json = available_schemas.to_json_value();
        let ai_response = self
            .get_ai_schema_recommendation(data, &schemas_json)
            .await?;
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
    async fn setup_schema(
        &self,
        ai_response: &AISchemaResponse,
        sample_data: &Value,
    ) -> IngestionResult<(String, bool, HashMap<String, String>)> {
        let (schema_name, mutation_mappers) = self.determine_schema_to_use(ai_response, sample_data).await?;
        let new_schema_created = ai_response.new_schemas.is_some();
        Ok((schema_name, new_schema_created, mutation_mappers))
    }

    /// Executes mutations if auto-execution is enabled.
    async fn execute_mutations_if_requested(
        &self,
        request: &IngestionRequest,
        mutations: &[Mutation],
    ) -> IngestionResult<usize> {
        if request
            .auto_execute
            .unwrap_or(self.config.auto_execute_mutations)
        {
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
            schema_name,
            mutations_count,
            mutations_executed
        );
    }

    /// Get available schemas from the schema service
    async fn get_stripped_available_schemas(&self) -> IngestionResult<SimplifiedSchemaMap> {
        // Fetch available schemas from the schema service
        let schemas = self
            .schema_service_client
            .get_available_schemas()
            .await
            .map_err(|e| {
                IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                    format!("Failed to fetch schemas from schema service: {}", e),
                ))
            })?;

        // Create a simplified schema representation for AI analysis
        let mut schema_map = SimplifiedSchemaMap::new();

        for schema in schemas {
            let fields = if let Ok(Value::Object(fields_obj)) = serde_json::to_value(&schema.fields) {
                fields_obj.into_iter().collect()
            } else {
                HashMap::new()
            };

            let simplified = SimplifiedSchema {
                name: schema.name.clone(),
                fields,
            };

            schema_map.insert(schema.name.clone(), simplified);
        }

        Ok(schema_map)
    }

    /// Get AI schema recommendation
    async fn get_ai_schema_recommendation(
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

    /// Determine which schema to use based on AI response
    async fn determine_schema_to_use(
        &self,
        ai_response: &AISchemaResponse,
        sample_data: &Value,
    ) -> IngestionResult<(String, HashMap<String, String>)> {
        // If existing schemas were recommended, use the first one
        if !ai_response.existing_schemas.is_empty() {
            let schema_name = &ai_response.existing_schemas[0];
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Using existing schema: {}",
                schema_name
            );
            
            // Ensure existing schema has topologies - add them if missing
            self.ensure_schema_has_topologies(schema_name, sample_data, &ai_response.mutation_mappers).await?;
            
            return Ok((schema_name.clone(), ai_response.mutation_mappers.clone()));
        }

        // If a new schema was provided, create it
        if let Some(new_schema_def) = &ai_response.new_schemas {
            let (schema_name, mutation_mappers) = self.create_new_schema(new_schema_def, sample_data, ai_response.mutation_mappers.clone()).await?;
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Created new schema: {} with {} mutation mappers",
                schema_name,
                mutation_mappers.len()
            );
            return Ok((schema_name, mutation_mappers));
        }

        Err(IngestionError::ai_response_validation_error(
            "AI response contains neither existing schemas nor new schema definition",
        ))
    }

    /// Create a new schema from AI response with mutation mappers
    async fn create_new_schema(&self, schema_def: &Value, sample_data: &Value, mutation_mappers: HashMap<String, String>) -> IngestionResult<(String, HashMap<String, String>)> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Creating new schema from AI definition with {} mutation mappers",
            mutation_mappers.len()
        );

        // Deserialize Value to Schema
        let mut schema: crate::schema::types::Schema = serde_json::from_value(schema_def.clone())
            .map_err(|error| {
                IngestionError::SchemaCreationError(format!(
                    "Failed to deserialize schema from AI response: {}",
                    error
                ))
            })?;

        // Infer topologies from sample data
        let sample_for_topology = if let Some(array) = sample_data.as_array() {
            // Use first element if array
            array.first().unwrap_or(sample_data)
        } else {
            sample_data
        };

        if let Some(sample_obj) = sample_for_topology.as_object() {
            let sample_map: std::collections::HashMap<String, serde_json::Value> = 
                sample_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            schema.infer_topologies_from_data(&sample_map);
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Inferred topologies for {} fields from sample data",
                sample_map.len()
            );
        }

        // Use topology_hash as schema name for structure-based deduplication
        let topology_hash = schema.get_topology_hash()
            .ok_or_else(|| IngestionError::SchemaCreationError(
                "Schema must have topology_hash computed".to_string()
            ))?
            .clone();
        
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Using topology_hash as schema name: {}",
            topology_hash
        );
        
        schema.name = topology_hash.clone();

        let add_schema_response = self
            .schema_service_client
            .add_schema(&schema, mutation_mappers)
            .await
            .map_err(|error| {
                IngestionError::SchemaCreationError(format!(
                    "Failed to create schema via schema service: {}",
                    error
                ))
            })?;

        let json_str = serde_json::to_string(&add_schema_response.schema).map_err(|error| {
            IngestionError::schema_parsing_error(format!(
                "Failed to serialize schema definition: {}",
                error
            ))
        })?;

        self.schema_core
            .load_schema_from_json(&json_str)
            .map_err(IngestionError::SchemaSystemError)?;

        let schema_name = add_schema_response.schema.name.clone();
        let returned_mutation_mappers = add_schema_response.mutation_mappers;

        // Auto-approve the new schema (idempotent - only approves if not already approved)
        self.schema_core
            .approve(&schema_name)
            .map_err(IngestionError::SchemaSystemError)?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "New schema '{}' created and approved with {} mutation mappers",
            schema_name,
            returned_mutation_mappers.len()
        );
        Ok((schema_name, returned_mutation_mappers))
    }

    /// Ensure existing schema has topologies for all fields, adding them if missing
    async fn ensure_schema_has_topologies(
        &self,
        schema_name: &str,
        sample_data: &Value,
        mutation_mappers: &HashMap<String, String>,
    ) -> IngestionResult<()> {
        // Get the schema from the schema service
        let mut schema = self
            .schema_service_client
            .get_schema(schema_name)
            .await
            .map_err(|e| {
                IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                    format!("Failed to fetch schema '{}' from schema service: {}", schema_name, e),
                ))
            })?;

        // Check if schema already has all required topologies
        let fields_to_check: Vec<String> = mutation_mappers
            .values()
            .filter_map(|mapper| {
                // Extract field name from mapper (format: SchemaName.field_name)
                mapper.split('.').nth(1).map(|s| s.to_string())
            })
            .collect();

        let mut needs_update = false;
        for field_name in &fields_to_check {
            if !schema.has_field_topology(field_name) {
                needs_update = true;
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "Schema '{}' is missing topology for field '{}'",
                    schema_name,
                    field_name
                );
                break;
            }
        }

        // If all fields have topologies, no update needed
        if !needs_update {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Schema '{}' already has topologies for all required fields",
                schema_name
            );
            return Ok(());
        }

        // Infer topologies from sample data
        let sample_for_topology = if let Some(array) = sample_data.as_array() {
            array.first().unwrap_or(sample_data)
        } else {
            sample_data
        };

        if let Some(sample_obj) = sample_for_topology.as_object() {
            let sample_map: std::collections::HashMap<String, serde_json::Value> = 
                sample_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            
            schema.infer_topologies_from_data(&sample_map);
            
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Inferred topologies for {} fields in schema '{}' from sample data",
                sample_map.len(),
                schema_name
            );

            // Update the schema in the schema service
            let empty_mappers = HashMap::new();
            self.schema_service_client
                .add_schema(&schema, empty_mappers)
                .await
                .map_err(|e| {
                    IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                        format!("Failed to update schema '{}' with topologies: {}", schema_name, e),
                    ))
                })?;

            // Reload the schema in the local schema core
            let json_str = serde_json::to_string(&schema).map_err(|error| {
                IngestionError::schema_parsing_error(format!(
                    "Failed to serialize updated schema: {}",
                    error
                ))
            })?;

            self.schema_core
                .load_schema_from_json(&json_str)
                .map_err(IngestionError::SchemaSystemError)?;

            log_feature!(
                LogFeature::Ingestion,
                info,
                "Updated schema '{}' with inferred topologies",
                schema_name
            );
        }

        Ok(())
    }

    /// Generate mutations for the data
    fn generate_mutations_for_data(
        &self,
        schema_name: &str,
        json_data: &Value,
        mutation_mappers: &HashMap<String, String>,
        trust_distance: u32,
        pub_key: String,
    ) -> IngestionResult<Vec<Mutation>> {
        // Handle both single objects and arrays of objects
        if let Some(array) = json_data.as_array() {
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
                    schema_name,
                    &HashMap::new(),
                    &fields_and_values,
                    mutation_mappers,
                    trust_distance,
                    pub_key.clone(),
                )?;

                all_mutations.extend(mutations);
            }

            Ok(all_mutations)
        } else {
            // Handle single object
            let fields_and_values = if let Some(obj) = json_data.as_object() {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                HashMap::new()
            };

            self.mutation_generator.generate_mutations(
                schema_name,
                &HashMap::new(),
                &fields_and_values,
                mutation_mappers,
                trust_distance,
                pub_key,
            )
        }
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

        db.mutation_manager
            .write_mutation(mutation.clone())
            .map_err(IngestionError::SchemaSystemError)?;

        Ok(())
    }

    /// Get ingestion status
    pub fn get_status(&self) -> IngestionResult<IngestionStatus> {
        let (provider_name, model) = match self.config.provider {
            AIProvider::OpenRouter => ("OpenRouter".to_string(), self.config.openrouter.model.clone()),
            AIProvider::Ollama => ("Ollama".to_string(), self.config.ollama.model.clone()),
        };

        Ok(IngestionStatus {
            enabled: self.config.enabled,
            configured: self.config.is_ready(),
            provider: provider_name,
            model,
            auto_execute_mutations: self.config.auto_execute_mutations,
            default_trust_distance: self.config.default_trust_distance,
        })
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
    use crate::ingestion::config::AIProvider;
    use crate::schema::SchemaCore;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    // REMOVED: create_test_ingestion_core - dead code marked with #[allow(dead_code)]
    // This duplicated test setup logic available in testing_utils module

    #[test]
    fn test_ingestion_core_new_with_ollama_provider() {
        let config = IngestionConfig {
            provider: AIProvider::Ollama,
            ..Default::default()
        };

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path();

        let schema_core = Arc::new(SchemaCore::new_for_testing().unwrap());
        let fold_db = Arc::new(Mutex::new(FoldDB::new(db_path.to_str().unwrap()).unwrap()));

        let schema_client = SchemaServiceClient::new("http://localhost:0");
        let ingestion_core =
            IngestionCore::new(config, schema_core, fold_db, schema_client).unwrap();

        assert!(ingestion_core.ollama_service.is_some());
        assert!(ingestion_core.openrouter_service.is_none());
    }

    #[test]
    fn test_validate_input() {
        // Create isolated test setup for this test
        let mut config = IngestionConfig::from_env_allow_empty();
        config.enabled = true;
        config.openrouter.api_key = "test-key".to_string();

        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir
            .path()
            .join("test_validate")
            .to_string_lossy()
            .to_string();

        // Try to create components with better error handling
        let schema_core = match SchemaCore::new_for_testing() {
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

        let schema_client = SchemaServiceClient::new("http://localhost:0");
        let core = match IngestionCore::new(config, schema_core, fold_db, schema_client) {
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
}
