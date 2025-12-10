//! Core ingestion orchestrator

use crate::datafold_node::SchemaServiceClient;
use crate::fold_db_core::FoldDB;
use crate::ingestion::{
    config::AIProvider, mutation_generator::MutationGenerator, ollama_service::OllamaService,
    openrouter_service::OpenRouterService, AISchemaResponse,
    IngestionConfig, IngestionError, IngestionResponse, IngestionResult,
    IngestionStatus, SimplifiedSchema, SimplifiedSchemaMap,
    progress::{ProgressService, IngestionStep, IngestionResults},
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
use crate::schema::types::topology::{TopologyNode, PrimitiveValueType};

/// Core ingestion service that orchestrates the entire ingestion process
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
    /// Original source filename (for file uploads)
    pub source_file_name: Option<String>,
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

    /// Process JSON ingestion request with progress tracking
    pub async fn process_json_ingestion_with_progress(
        &self,
        request: IngestionRequest,
        progress_service: &ProgressService,
        progress_id: String,
    ) -> IngestionResult<IngestionResponse> {

        // Step 1: Validate configuration
        progress_service.update_progress(
            &progress_id,
            IngestionStep::ValidatingConfig,
            "Validating ingestion configuration...".to_string(),
        ).await;
        self.validate_configuration()?;

        // Step 2: Prepare schemas
        progress_service.update_progress(
            &progress_id,
            IngestionStep::PreparingSchemas,
            "Preparing available schemas...".to_string(),
        ).await;
        let available_schemas = self.prepare_schemas().await?;

        // Step 2.5: Flatten Twitter data structure for AI analysis
        progress_service.update_progress(
            &progress_id,
            IngestionStep::FlatteningData,
            "Processing and flattening data structure...".to_string(),
        ).await;
        let flattened_data = self.flatten_twitter_data(&request.data);

        // Step 3: Get AI recommendation
        progress_service.update_progress(
            &progress_id,
            IngestionStep::GettingAIRecommendation,
            "Analyzing data with AI to determine schema...".to_string(),
        ).await;
        let ai_response = self
            .get_ai_recommendation(&flattened_data, &available_schemas)
            .await?;

        // Step 4: Determine and setup schema
        progress_service.update_progress(
            &progress_id,
            IngestionStep::SettingUpSchema,
            "Setting up schema and preparing for data storage...".to_string(),
        ).await;
        let (schema_name, new_schema_created, mutation_mappers) = self.setup_schema(&ai_response, &flattened_data).await?;

        // Step 5: Generate mutations using the returned mutation_mappers (which may have been updated by schema service)
        progress_service.update_progress(
            &progress_id,
            IngestionStep::GeneratingMutations,
            "Generating database mutations...".to_string(),
        ).await;
        let mutations = self.generate_mutations_for_data(
            &schema_name,
            &flattened_data,
            &mutation_mappers,
            request.trust_distance.unwrap_or(self.config.default_trust_distance),
            request.pub_key.clone().unwrap_or_else(|| "default".to_string()),
            request.source_file_name.clone(),
        )?;

        // Step 6: Execute mutations if requested
        progress_service.update_progress(
            &progress_id,
            IngestionStep::ExecutingMutations,
            "Executing mutations to store data...".to_string(),
        ).await;
        let mutations_executed = self
            .execute_mutations_if_requested(&request, &mutations)
            .await?;

        // Mark as completed
        let results = IngestionResults {
            schema_name: schema_name.clone(),
            new_schema_created,
            mutations_generated: mutations.len(),
            mutations_executed,
        };
        progress_service.complete_progress(&progress_id, results).await;

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

    /// Flatten Twitter data structure for AI analysis
    /// Converts nested structures like [{"like": {"tweetId": "...", "fullText": "..."}}] 
    /// to flattened structures like [{"tweetId": "...", "fullText": "..."}]
    fn flatten_twitter_data(&self, data: &Value) -> Value {
        if let Some(array) = data.as_array() {
            let flattened_items: Vec<Value> = array.iter().map(|item| {
                if let Some(obj) = item.as_object() {
                    // Handle nested Twitter data structure (e.g., {"like": {...}} or {"following": {...}})
                    if obj.len() == 1 {
                        // If there's only one key, assume it's a wrapper and extract the inner object
                        let (_wrapper_key, inner_value) = obj.iter().next().unwrap();
                        if let Some(inner_obj) = inner_value.as_object() {
                            Value::Object(inner_obj.clone())
                        } else {
                            // Fallback to original structure if inner value is not an object
                            Value::Object(obj.clone())
                        }
                    } else {
                        // Multiple keys, use as-is
                        Value::Object(obj.clone())
                    }
                } else {
                    item.clone()
                }
            }).collect();
            
            Value::Array(flattened_items)
        } else if let Some(obj) = data.as_object() {
            // Handle single object case
            if obj.len() == 1 {
                let (wrapper_key, inner_value) = obj.iter().next().unwrap();
                if let Some(inner_obj) = inner_value.as_object() {
                    log_feature!(
                        LogFeature::Ingestion,
                        info,
                        "Flattening Twitter data: extracting inner object from wrapper '{}'",
                        wrapper_key
                    );
                    Value::Object(inner_obj.clone())
                } else {
                    Value::Object(obj.clone())
                }
            } else {
                Value::Object(obj.clone())
            }
        } else {
            data.clone()
        }
    }

    /// Prepares available schemas for AI recommendation.
    async fn prepare_schemas(&self) -> IngestionResult<SimplifiedSchemaMap> {
        let available_schemas = self.get_stripped_available_schemas().await?;
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
    fn log_completion(&self, _schema_name: &str, _mutations_count: usize, _mutations_executed: usize) {
        // Completion logging removed to reduce verbosity
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
            return Ok((schema_name, mutation_mappers));
        }

        Err(IngestionError::ai_response_validation_error(
            "AI response contains neither existing schemas nor new schema definition",
        ))
    }

    /// Create a new schema from AI response with mutation mappers
    async fn create_new_schema(&self, schema_def: &Value, sample_data: &Value, mutation_mappers: HashMap<String, String>) -> IngestionResult<(String, HashMap<String, String>)> {

        // Deserialize Value to Schema
        let mut schema: crate::schema::types::Schema = serde_json::from_value(schema_def.clone())
            .map_err(|error| {
                IngestionError::SchemaCreationError(format!(
                    "Failed to deserialize schema from AI response: {}",
                    error
                ))
            })?;


        // Ensure default classifications for all fields (e.g. "word" for strings)
        // This ensures that even if AI didn't provide classifications, we still index text
        Self::ensure_default_classifications(&mut schema);

        // Compute topology hash for the AI-generated topologies
        if !schema.field_topologies.is_empty() {
            schema.compute_schema_topology_hash();
        }

        // DON'T infer topologies - AI already provided them with classifications
        // Inferring would overwrite AI-generated classifications
        
        // Only infer if the schema is completely missing topologies
        if schema.field_topologies.is_empty() {
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "AI did not provide field_topologies, inferring from sample data"
            );
            
            let sample_for_topology = if let Some(array) = sample_data.as_array() {
                array.first().unwrap_or(sample_data)
            } else {
                sample_data
            };

            if let Some(sample_obj) = sample_for_topology.as_object() {
                let sample_map: std::collections::HashMap<String, serde_json::Value> = 
                    sample_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                schema.infer_topologies_from_data(&sample_map);
            }
        }

        // Use topology_hash as schema name for structure-based deduplication
        let topology_hash = schema.get_topology_hash()
            .ok_or_else(|| IngestionError::SchemaCreationError(
                "Schema must have topology_hash computed".to_string()
            ))?
            .clone();
        
        
        schema.name = topology_hash.clone();

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Submitting schema '{}' to schema service...",
            schema.name
        );

        let add_schema_response = self
            .schema_service_client
            .add_schema(&schema, mutation_mappers)
            .await
            .map_err(|error| {
                log_feature!(
                    LogFeature::Ingestion,
                    error,
                    "Failed to create schema via schema service: {}",
                    error
                );
                IngestionError::SchemaCreationError(format!(
                    "Failed to create schema via schema service: {}",
                    error
                ))
            })?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Schema service returned schema '{}'",
            add_schema_response.schema.name
        );

        let json_str = serde_json::to_string(&add_schema_response.schema).map_err(|error| {
            IngestionError::schema_parsing_error(format!(
                "Failed to serialize schema definition: {}",
                error
            ))
        })?;

            match self.schema_core.load_schema_from_json(&json_str).await {
                Ok(_) => {},
                Err(e) => return Err(IngestionError::SchemaSystemError(e)),
            };

        let schema_name = add_schema_response.schema.name.clone();
        let returned_mutation_mappers = add_schema_response.mutation_mappers;

        // Auto-approve the new schema (idempotent - only approves if not already approved)
        self.schema_core
            .approve(&schema_name)
            .await
            .map_err(IngestionError::SchemaSystemError)?;

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

            match self.schema_core.load_schema_from_json(&json_str).await {
                Ok(_) => {},
                Err(e) => return Err(IngestionError::SchemaSystemError(e)),
            };

            log_feature!(
                LogFeature::Ingestion,
                info,
                "Updated schema '{}' with inferred topologies",
                schema_name
            );
        }

        Ok(())
    }

    /// Extract key values from JSON data based on schema key fields
    fn extract_key_values_from_data(
        &self,
        fields_and_values: &HashMap<String, Value>,
        schema_name: &str,
    ) -> IngestionResult<HashMap<String, String>> {
        let mut keys_and_values = HashMap::new();
        
        // Get the schema to understand its key structure
        if let Ok(Some(schema)) = self.schema_core.get_schema(schema_name) {
            if let Some(key_def) = &schema.key {
                // Extract hash field value if present
                if let Some(hash_field) = &key_def.hash_field {
                    if let Some(hash_value) = fields_and_values.get(hash_field) {
                        if let Some(hash_str) = hash_value.as_str() {
                            keys_and_values.insert("hash_field".to_string(), hash_str.to_string());
                        } else if let Some(hash_num) = hash_value.as_f64() {
                            keys_and_values.insert("hash_field".to_string(), hash_num.to_string());
                        }
                    }
                }
                
                // Extract range field value if present
                if let Some(range_field) = &key_def.range_field {
                    if let Some(range_value) = self.extract_nested_field_value(fields_and_values, range_field) {
                        if let Some(range_str) = range_value.as_str() {
                            keys_and_values.insert("range_field".to_string(), range_str.to_string());
                        } else if let Some(range_num) = range_value.as_f64() {
                            keys_and_values.insert("range_field".to_string(), range_num.to_string());
                        }
                    }
                }
            }
        }
        
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Extracted key values for schema '{}': {:?}",
            schema_name,
            keys_and_values
        );
        
        Ok(keys_and_values)
    }

    /// Extract nested field value from JSON data using dot notation
    fn extract_nested_field_value<'a>(
        &self,
        fields_and_values: &'a HashMap<String, Value>,
        field_path: &str,
    ) -> Option<&'a Value> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "🔍 Searching for field '{}' in data with top-level keys: {:?}",
            field_path,
            fields_and_values.keys().collect::<Vec<_>>()
        );
        
        // First try direct field access
        if let Some(value) = fields_and_values.get(field_path) {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "✅ Found field '{}' at top level",
                field_path
            );
            return Some(value);
        }
        
        // Then try nested field access (e.g., "like.tweetId")
        if field_path.contains('.') {
            let parts: Vec<&str> = field_path.split('.').collect();
            if parts.len() == 2 {
                if let Some(parent_value) = fields_and_values.get(parts[0]) {
                    if let Some(parent_obj) = parent_value.as_object() {
                        if let Some(result) = parent_obj.get(parts[1]) {
                            log_feature!(
                                LogFeature::Ingestion,
                                info,
                                "✅ Found field '{}' using dot notation",
                                field_path
                            );
                            return Some(result);
                        }
                    }
                }
            }
        }
        
        // Try to find the field in nested objects
        for (parent_key, value) in fields_and_values {
            if let Some(obj) = value.as_object() {
                if let Some(nested_value) = obj.get(field_path) {
                    log_feature!(
                        LogFeature::Ingestion,
                        info,
                        "✅ Found field '{}' nested inside '{}'",
                        field_path,
                        parent_key
                    );
                    return Some(nested_value);
                }
            }
        }
        
        log_feature!(
            LogFeature::Ingestion,
            warn,
            "❌ Field '{}' not found in data",
            field_path
        );
        
        None
    }

    /// Generate mutations for the data
    fn generate_mutations_for_data(
        &self,
        schema_name: &str,
        json_data: &Value,
        mutation_mappers: &HashMap<String, String>,
        trust_distance: u32,
        pub_key: String,
        source_file_name: Option<String>,
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

                // Extract key values from the JSON data based on schema key fields
                let keys_and_values = self.extract_key_values_from_data(
                    &fields_and_values,
                    schema_name,
                )?;

                let mutations = self.mutation_generator.generate_mutations(
                    schema_name,
                    &keys_and_values,
                    &fields_and_values,
                    mutation_mappers,
                    trust_distance,
                    pub_key.clone(),
                    source_file_name.clone(),
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

            // Extract key values from the JSON data based on schema key fields
            let keys_and_values = self.extract_key_values_from_data(
                &fields_and_values,
                schema_name,
            )?;

            self.mutation_generator.generate_mutations(
                schema_name,
                &keys_and_values,
                &fields_and_values,
                mutation_mappers,
                trust_distance,
                pub_key,
                source_file_name,
            )
        }
    }

    /// Ensures that all String fields have at least the "word" classification
    /// This is crucial for the native index to work by default for text fields
    fn ensure_default_classifications(schema: &mut crate::schema::types::Schema) {
        for topology in schema.field_topologies.values_mut() {
            Self::ensure_default_classifications_recursive(&mut topology.root);
        }
    }

    fn ensure_default_classifications_recursive(node: &mut TopologyNode) {
        match node {
            TopologyNode::Primitive { value, classifications } => {
                if let PrimitiveValueType::String = value {
                    if classifications.is_none() {
                        *classifications = Some(vec!["word".to_string()]);
                    } else if let Some(classes) = classifications {
                        if classes.is_empty() {
                            classes.push("word".to_string());
                        }
                    }
                }
            }
            TopologyNode::Object { value } => {
                for node in value.values_mut() {
                    Self::ensure_default_classifications_recursive(node);
                }
            }
            TopologyNode::Array { value } => {
                Self::ensure_default_classifications_recursive(value);
            }
            _ => {}
        }
    }

    /// Execute mutations
    async fn execute_mutations(&self, mutations: &[Mutation]) -> IngestionResult<usize> {
        let mut executed_count = 0;

        for mutation in mutations {
            match self.execute_single_mutation(mutation).await {
                Ok(_mutation_id) => {
                    executed_count += 1;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(executed_count)
    }

    /// Execute a single mutation (now uses batch internally for efficiency) and return its ID
    async fn execute_single_mutation(&self, mutation: &Mutation) -> IngestionResult<String> {
        // Use block_in_place to acquire std::sync::Mutex without blocking the async runtime
        let mut db = tokio::task::block_in_place(|| {
            self.fold_db.lock()
        }).map_err(|_| {
            IngestionError::DatabaseError("Failed to acquire database lock".to_string())
        })?;

        // Use async batch API to avoid deadlocks with DynamoDB
        let mut ids = db
            .mutation_manager
            .write_mutations_batch_async(vec![mutation.clone()])
            .await
            .map_err(IngestionError::SchemaSystemError)?;

        ids.pop().ok_or_else(|| {
            IngestionError::DatabaseError("Batch mutation returned no IDs".to_string())
        })
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
    use log::warn;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    // REMOVED: create_test_ingestion_core - dead code marked with #[allow(dead_code)]
    // This duplicated test setup logic available in testing_utils module

    #[tokio::test]
    async fn test_ingestion_core_new_with_ollama_provider() {
        let config = IngestionConfig {
            provider: AIProvider::Ollama,
            ..Default::default()
        };

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path();

        let schema_core = Arc::new(SchemaCore::new_for_testing().await.unwrap());
        let fold_db = Arc::new(Mutex::new(FoldDB::new(db_path.to_str().unwrap()).await.unwrap()));

        let schema_client = SchemaServiceClient::new("http://localhost:0");
        let ingestion_core =
            IngestionCore::new(config, schema_core, fold_db, schema_client).unwrap();

        assert!(ingestion_core.ollama_service.is_some());
        assert!(ingestion_core.openrouter_service.is_none());
    }

    #[tokio::test]
    async fn test_validate_input() {
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
        let schema_core = match SchemaCore::new_for_testing().await {
            Ok(core) => Arc::new(core),
            Err(_) => {
                warn!("Skipping test_validate_input: Could not create schema core");
                return;
            }
        };

        let fold_db = match FoldDB::new(&test_path).await {
            Ok(db) => Arc::new(Mutex::new(db)),
            Err(_) => {
                warn!("Skipping test_validate_input: Could not create database");
                return;
            }
        };

        let schema_client = SchemaServiceClient::new("http://localhost:0");
        let core = match IngestionCore::new(config, schema_core, fold_db, schema_client) {
            Ok(core) => core,
            Err(_) => {
                warn!("Skipping test_validate_input: Could not create ingestion core");
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
