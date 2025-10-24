//! Simplified ingestion service that works with DataFoldNode's existing interface

use crate::datafold_node::{DataFoldNode, OperationProcessor};
use crate::ingestion::config::AIProvider;
use crate::ingestion::core::IngestionRequest;
use crate::ingestion::mutation_generator::MutationGenerator;
use crate::ingestion::ollama_service::OllamaService;
use crate::ingestion::openrouter_service::OpenRouterService;
use crate::ingestion::{
    AISchemaResponse, IngestionConfig, IngestionError, IngestionResponse, IngestionResult,
    IngestionStatus, SimplifiedSchema, SimplifiedSchemaMap,
};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::{Mutation, Operation};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Schema cache entry
#[derive(Debug, Clone)]
struct SchemaCacheEntry {
    schemas: SimplifiedSchemaMap,
    timestamp: Instant,
}

/// Simplified ingestion service that works with DataFoldNode
pub struct SimpleIngestionService {
    config: IngestionConfig,
    openrouter_service: Option<OpenRouterService>,
    ollama_service: Option<OllamaService>,
    mutation_generator: MutationGenerator,
    schema_cache: Arc<Mutex<Option<SchemaCacheEntry>>>,
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

        let mutation_generator = MutationGenerator::new();

        Ok(Self {
            config,
            openrouter_service,
            ollama_service,
            mutation_generator,
            schema_cache: Arc::new(Mutex::new(None)),
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
            available_schemas.len()
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
            .determine_schema_to_use(&ai_response, &request.data, node.clone())
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
                    &HashMap::new(),
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
                HashMap::new()
            };

            self.mutation_generator.generate_mutations(
                &schema_name,
                &HashMap::new(),
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
        available_schemas: &SimplifiedSchemaMap,
    ) -> IngestionResult<AISchemaResponse> {
        let schemas_json = available_schemas.to_json_value();
        match self.config.provider {
            AIProvider::OpenRouter => {
                self.openrouter_service
                    .as_ref()
                    .ok_or_else(|| {
                        IngestionError::configuration_error("OpenRouter service not initialized")
                    })?
                    .get_schema_recommendation(json_data, &schemas_json)
                    .await
            }
            AIProvider::Ollama => {
                self.ollama_service
                    .as_ref()
                    .ok_or_else(|| {
                        IngestionError::configuration_error("Ollama service not initialized")
                    })?
                    .get_schema_recommendation(json_data, &schemas_json)
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

    /// Get available schemas from the schema service via node (with caching)
    async fn get_stripped_available_schemas_from_node(
        &self,
        node: Arc<Mutex<DataFoldNode>>,
    ) -> IngestionResult<SimplifiedSchemaMap> {
        const CACHE_TTL: Duration = Duration::from_secs(30); // 30 second cache
        
        // Check cache first
        {
            let cache_guard = self.schema_cache.lock().await;
            if let Some(cache_entry) = cache_guard.as_ref() {
                if cache_entry.timestamp.elapsed() < CACHE_TTL {
                    log_feature!(
                        LogFeature::Ingestion,
                        info,
                        "Using cached schemas ({} schemas, {}s old)",
                        cache_entry.schemas.len(),
                        cache_entry.timestamp.elapsed().as_secs()
                    );
                    return Ok(cache_entry.schemas.clone());
                }
            }
        }

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Cache miss or expired, fetching schemas from schema service"
        );

        // Fetch available schemas from the schema service via the node
        let schemas = {
            let node_guard = node.lock().await;
            node_guard
                .fetch_available_schemas()
                .await
                .map_err(|e| {
                    IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                        format!("Failed to fetch schemas from schema service: {}", e),
                    ))
                })?
        };

        // Create a simplified schema representation for AI analysis
        let mut schema_map = SimplifiedSchemaMap::new();

        for schema in schemas {
            let mut fields: HashMap<String, Value> = HashMap::new();
            
            for (field_name, topology) in &schema.field_topologies {
                if let Ok(topology_value) = serde_json::to_value(topology) {
                    fields.insert(field_name.clone(), topology_value);
                }
            }

            let simplified = SimplifiedSchema {
                name: schema.name.clone(),
                fields,
            };

            schema_map.insert(schema.name.clone(), simplified);
        }

        // Update cache
        {
            let mut cache_guard = self.schema_cache.lock().await;
            *cache_guard = Some(SchemaCacheEntry {
                schemas: schema_map.clone(),
                timestamp: Instant::now(),
            });
        }

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Cached {} schemas for future requests",
            schema_map.len()
        );

        Ok(schema_map)
    }

    /// Determine which schema to use based on AI response
    async fn determine_schema_to_use(
        &self,
        ai_response: &AISchemaResponse,
        sample_data: &Value,
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
            
            // Ensure existing schema has topologies - add them if missing
            self.ensure_schema_has_topologies_with_node(schema_name, sample_data, &ai_response.mutation_mappers, node.clone()).await?;
            
            // Auto-approve existing schema (idempotent - only approves if not already approved)
            let schema_manager = {
                let node_guard = node.lock().await;
                let db_guard = node_guard
                    .get_fold_db()
                    .map_err(|error| IngestionError::SchemaCreationError(error.to_string()))?;
                db_guard.schema_manager.clone()
            };

            schema_manager
                .approve(schema_name)
                .map_err(|error| IngestionError::SchemaCreationError(error.to_string()))?;
            
            return Ok(schema_name.clone());
        }

        // If a new schema was provided, create it
        if let Some(new_schema_def) = &ai_response.new_schemas {
            let schema_name = self
                .create_new_schema_with_node(new_schema_def, sample_data, node.clone())
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
        sample_data: &Value,
        node: Arc<Mutex<DataFoldNode>>,
    ) -> IngestionResult<String> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Creating new schema from AI definition"
        );

        // Deserialize Value to Schema
        let mut schema: crate::schema::types::Schema = serde_json::from_value(schema_def.clone())
            .map_err(|error| {
                IngestionError::SchemaCreationError(format!(
                    "Failed to deserialize schema from AI response: {}",
                    error
                ))
            })?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Deserialized schema with {} field topologies from AI",
            schema.field_topologies.len()
        );

        // Compute topology hash for the AI-generated topologies
        if !schema.field_topologies.is_empty() {
            schema.compute_schema_topology_hash();
            log_feature!(
                LogFeature::Ingestion,
                info,
                "Computed topology hash from {} AI-generated topologies",
                schema.field_topologies.len()
            );
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
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "Inferred topologies for {} fields (no AI topologies)",
                    sample_map.len()
                );
            }
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

        // Add schema to the schema service via the node
        let schema_response = {
            let node_guard = node.lock().await;
            node_guard
                .add_schema_to_service(&schema)
                .await
                .map_err(|error| {
                    IngestionError::SchemaCreationError(format!(
                        "Failed to create schema via schema service: {}",
                        error
                    ))
                })?
        };

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

        // Auto-approve the new schema (idempotent - only approves if not already approved)
        schema_manager
            .approve(&schema_response.name)
            .map_err(|error| IngestionError::SchemaCreationError(error.to_string()))?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "New schema '{}' created and approved",
            schema_response.name
        );
        Ok(schema_response.name)
    }

    /// Ensure existing schema has topologies for all fields, adding them if missing
    async fn ensure_schema_has_topologies_with_node(
        &self,
        schema_name: &str,
        sample_data: &Value,
        mutation_mappers: &HashMap<String, String>,
        node: Arc<Mutex<DataFoldNode>>,
    ) -> IngestionResult<()> {
        // Get the schema from the schema manager
        let mut schema = {
            let node_guard = node.lock().await;
            let db_guard = node_guard
                .get_fold_db()
                .map_err(|error| IngestionError::SchemaCreationError(error.to_string()))?;
            let schema = db_guard.schema_manager
                .get_schema(schema_name)
                .map_err(|e| {
                    IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                        format!("Failed to fetch schema '{}': {}", schema_name, e),
                    ))
                })?
                .ok_or_else(|| {
                    IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                        format!("Schema '{}' not found", schema_name),
                    ))
                })?
                .clone();
            drop(db_guard);
            schema
        };

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

            // Update the schema in the schema service via node
            {
                let node_guard = node.lock().await;
                node_guard
                    .add_schema_to_service(&schema)
                    .await
                    .map_err(|e| {
                        IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                            format!("Failed to update schema '{}' with topologies: {}", schema_name, e),
                        ))
                    })?;
            }

            // Reload the schema
            let json_str = serde_json::to_string(&schema).map_err(|error| {
                IngestionError::schema_parsing_error(format!(
                    "Failed to serialize updated schema: {}",
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
                "Updated schema '{}' with inferred topologies",
                schema_name
            );
        }

        Ok(())
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
            let exec_result: Result<(), IngestionError> = match operation {
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
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(executed_count)
    }
}
