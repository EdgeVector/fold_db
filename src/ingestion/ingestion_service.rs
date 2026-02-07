//! AI-powered ingestion service that works with DataFoldNode
//!
//! Handles JSON data ingestion with AI schema recommendation, mutation generation,
//! and execution. Refactored to take &DataFoldNode references for flexible locking.

use crate::datafold_node::DataFoldNode;
use crate::ingestion::config::AIProvider;
use crate::ingestion::IngestionRequest;
use crate::ingestion::mutation_generator::MutationGenerator;
use crate::ingestion::ollama_service::OllamaService;
use crate::ingestion::openrouter_service::OpenRouterService;
use crate::ingestion::progress::{IngestionResults, IngestionStep, ProgressService};
use crate::ingestion::{
    AISchemaResponse, IngestionConfig, IngestionError, IngestionResponse, IngestionResult,
    IngestionStatus,
};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::Mutation;
use crate::schema::SchemaCore;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Extract key values from JSON data based on schema key fields.
/// Looks up the schema in the node's schema manager to find key configuration,
/// then extracts the corresponding values from the data.
async fn extract_key_values_from_data(
    fields_and_values: &HashMap<String, Value>,
    schema_name: &str,
    schema_manager: &Arc<SchemaCore>,
) -> IngestionResult<HashMap<String, String>> {
    let mut keys_and_values = HashMap::new();

    if let Ok(Some(schema)) = schema_manager.get_schema(schema_name) {
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
                if let Some(range_value) =
                    extract_nested_field_value(fields_and_values, range_field)
                {
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

/// Extract nested field value from JSON data using dot notation.
fn extract_nested_field_value<'a>(
    fields_and_values: &'a HashMap<String, Value>,
    field_path: &str,
) -> Option<&'a Value> {
    // First try direct field access
    if let Some(value) = fields_and_values.get(field_path) {
        return Some(value);
    }

    // Then try nested field access (e.g., "like.tweetId")
    if field_path.contains('.') {
        let parts: Vec<&str> = field_path.split('.').collect();
        if parts.len() == 2 {
            if let Some(parent_value) = fields_and_values.get(parts[0]) {
                if let Some(parent_obj) = parent_value.as_object() {
                    if let Some(result) = parent_obj.get(parts[1]) {
                        return Some(result);
                    }
                }
            }
        }
    }

    // Try to find the field in nested objects
    for value in fields_and_values.values() {
        if let Some(obj) = value.as_object() {
            if let Some(nested_value) = obj.get(field_path) {
                return Some(nested_value);
            }
        }
    }

    None
}

/// AI-powered ingestion service that works with DataFoldNode
pub struct IngestionService {
    config: IngestionConfig,
    openrouter_service: Option<OpenRouterService>,
    ollama_service: Option<OllamaService>,
    mutation_generator: MutationGenerator,
}

impl IngestionService {
    /// Create a new ingestion service
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
        })
    }

    /// Process JSON ingestion using a DataFoldNode with progress tracking
    /// Accepts a reference to DataFoldNode, making it compatible with both Mutex and RwLock guards
    pub async fn process_json_with_node_and_progress(
        &self,
        request: IngestionRequest,
        node: &DataFoldNode,
        progress_service: &ProgressService,
        progress_id: String,
    ) -> IngestionResult<IngestionResponse> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Starting JSON ingestion process with DataFoldNode (progress_id: {})",
            progress_id
        );

        if !self.config.is_ready() {
            progress_service
                .fail_progress(
                    &progress_id,
                    "Ingestion module is not properly configured or disabled".to_string(),
                )
                .await;
            return Ok(IngestionResponse::failure(vec![
                "Ingestion module is not properly configured or disabled".to_string(),
            ]));
        }

        // Step 1: Validate input
        progress_service
            .update_progress(
                &progress_id,
                IngestionStep::ValidatingConfig,
                "Validating input data...".to_string(),
            )
            .await;
        self.validate_input(&request.data)?;

        // Step 2: Flatten data structure for AI analysis
        progress_service
            .update_progress(
                &progress_id,
                IngestionStep::FlatteningData,
                "Processing and flattening data structure...".to_string(),
            )
            .await;
        let flattened_data = self.flatten_twitter_data(&request.data);

        // Step 3: Get AI recommendation
        progress_service
            .update_progress(
                &progress_id,
                IngestionStep::GettingAIRecommendation,
                "Analyzing data with AI to determine schema...".to_string(),
            )
            .await;
        let ai_response = self
            .get_ai_recommendation(&flattened_data)
            .await?;

        // Step 4: Determine schema to use
        progress_service
            .update_progress(
                &progress_id,
                IngestionStep::SettingUpSchema,
                "Setting up schema and preparing for data storage...".to_string(),
            )
            .await;
        let schema_name = self
            .determine_schema_to_use(&ai_response, &request.data, node)
            .await?;
        let new_schema_created = ai_response.new_schemas.is_some();

        // Step 5: Generate mutations
        progress_service
            .update_progress(
                &progress_id,
                IngestionStep::GeneratingMutations,
                "Generating database mutations...".to_string(),
            )
            .await;

        // Get schema manager for key extraction
        let schema_manager = {
            let db_guard = node
                .get_fold_db()
                .await
                .map_err(|error| IngestionError::SchemaCreationError(error.to_string()))?;
            let manager = db_guard.schema_manager.clone();
            drop(db_guard);
            manager
        };

        // Handle both single objects and arrays of objects
        let mutations = if let Some(array) = flattened_data.as_array() {
            // Generate a mutation for each element in the array
            let total_items = array.len();
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

                let keys_and_values = extract_key_values_from_data(
                    &fields_and_values,
                    &schema_name,
                    &schema_manager,
                )
                .await?;

                let mutations = self.mutation_generator.generate_mutations(
                    &schema_name,
                    &keys_and_values,
                    &fields_and_values,
                    &ai_response.mutation_mappers,
                    request
                        .trust_distance
                        .unwrap_or(self.config.default_trust_distance),
                    request.pub_key.clone().ok_or_else(|| {
                        IngestionError::invalid_input("Missing pub_key for mutation generation")
                    })?,
                    request.source_file_name.clone(),
                )?;

                all_mutations.extend(mutations);

                // Update progress every 10 items
                if (idx + 1) % 10 == 0 || idx + 1 == total_items {
                    let percent_of_step = ((idx + 1) as f32 / total_items as f32 * 15.0) as u8;
                    let progress_percent = 75 + percent_of_step;
                    progress_service
                        .update_progress_with_percentage(
                            &progress_id,
                            IngestionStep::GeneratingMutations,
                            format!("Generating mutations... ({}/{})", idx + 1, total_items),
                            progress_percent,
                        )
                        .await;
                }
            }

            all_mutations
        } else {
            // Handle single object
            let fields_and_values = if let Some(obj) = flattened_data.as_object() {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                HashMap::new()
            };

            let keys_and_values = extract_key_values_from_data(
                &fields_and_values,
                &schema_name,
                &schema_manager,
            )
            .await?;

            self.mutation_generator.generate_mutations(
                &schema_name,
                &keys_and_values,
                &fields_and_values,
                &ai_response.mutation_mappers,
                request
                    .trust_distance
                    .unwrap_or(self.config.default_trust_distance),
                request.pub_key.clone().ok_or_else(|| {
                    IngestionError::invalid_input("Missing pub_key for mutation generation")
                })?,
                request.source_file_name.clone(),
            )?
        };

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Generated {} mutations",
            mutations.len()
        );

        // Step 6: Execute mutations if requested
        progress_service
            .update_progress(
                &progress_id,
                IngestionStep::ExecutingMutations,
                "Executing mutations to store data...".to_string(),
            )
            .await;

        let mutations_len = mutations.len();

        let mutations_executed = if request
            .auto_execute
            .unwrap_or(self.config.auto_execute_mutations)
        {
            self.execute_mutations_with_node_and_progress(
                mutations,
                node,
                progress_service,
                &progress_id,
            )
            .await?
        } else {
            0
        };

        // Mark as completed
        let results = IngestionResults {
            schema_name: schema_name.clone(),
            new_schema_created,
            mutations_generated: mutations_len,
            mutations_executed,
        };
        progress_service
            .complete_progress(&progress_id, results)
            .await;

        Ok(IngestionResponse::success_with_progress(
            progress_id,
            schema_name,
            new_schema_created,
            mutations_len,
            mutations_executed,
        ))
    }

    /// Process JSON ingestion using a DataFoldNode (original method for backward compatibility)
    pub async fn process_json_with_node(
        &self,
        request: IngestionRequest,
        node: &DataFoldNode,
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

        // Step 2: Flatten Twitter data
        let flattened_data = self.flatten_twitter_data(&request.data);

        // Step 3: Get AI recommendation
        let ai_response = self
            .get_ai_recommendation(&flattened_data)
            .await?;

        // Step 4: Determine schema to use
        let schema_name = self
            .determine_schema_to_use(&ai_response, &request.data, node)
            .await?;
        let new_schema_created = ai_response.new_schemas.is_some();

        // Step 5: Generate mutations
        // Get schema manager for key extraction
        let schema_manager = {
            let db_guard = node
                .get_fold_db()
                .await
                .map_err(|error| IngestionError::SchemaCreationError(error.to_string()))?;
            let manager = db_guard.schema_manager.clone();
            drop(db_guard);
            manager
        };

        // Handle both single objects and arrays of objects
        let mutations = if let Some(array) = flattened_data.as_array() {
            // Generate a mutation for each element in the array
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

                let keys_and_values = extract_key_values_from_data(
                    &fields_and_values,
                    &schema_name,
                    &schema_manager,
                )
                .await?;

                let mutations = self.mutation_generator.generate_mutations(
                    &schema_name,
                    &keys_and_values,
                    &fields_and_values,
                    &ai_response.mutation_mappers,
                    request
                        .trust_distance
                        .unwrap_or(self.config.default_trust_distance),
                    request.pub_key.clone().ok_or_else(|| {
                        IngestionError::invalid_input("Missing pub_key for mutation generation")
                    })?,
                    request.source_file_name.clone(),
                )?;

                all_mutations.extend(mutations);
            }

            all_mutations
        } else {
            // Handle single object
            let fields_and_values = if let Some(obj) = flattened_data.as_object() {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                HashMap::new()
            };

            let keys_and_values = extract_key_values_from_data(
                &fields_and_values,
                &schema_name,
                &schema_manager,
            )
            .await?;

            self.mutation_generator.generate_mutations(
                &schema_name,
                &keys_and_values,
                &fields_and_values,
                &ai_response.mutation_mappers,
                request
                    .trust_distance
                    .unwrap_or(self.config.default_trust_distance),
                request.pub_key.clone().ok_or_else(|| {
                    IngestionError::invalid_input("Missing pub_key for mutation generation")
                })?,
                request.source_file_name.clone(),
            )?
        };

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Generated {} mutations",
            mutations.len()
        );

        let mutations_len = mutations.len();

        // Step 7: Execute mutations if requested
        let mutations_executed = if request
            .auto_execute
            .unwrap_or(self.config.auto_execute_mutations)
        {
            self.execute_mutations_with_node(mutations, node).await?
        } else {
            0
        };

        Ok(IngestionResponse::success(
            schema_name,
            new_schema_created,
            mutations_len,
            mutations_executed,
        ))
    }

    /// Get AI schema recommendation
    async fn get_ai_recommendation(
        &self,
        json_data: &Value,
    ) -> IngestionResult<AISchemaResponse> {
        match self.config.provider {
            AIProvider::OpenRouter => {
                self.openrouter_service
                    .as_ref()
                    .ok_or_else(|| {
                        IngestionError::configuration_error("OpenRouter service not initialized")
                    })?
                    .get_schema_recommendation(json_data)
                    .await
            }
            AIProvider::Ollama => {
                self.ollama_service
                    .as_ref()
                    .ok_or_else(|| {
                        IngestionError::configuration_error("Ollama service not initialized")
                    })?
                    .get_schema_recommendation(json_data)
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
            AIProvider::OpenRouter => (
                "OpenRouter".to_string(),
                self.config.openrouter.model.clone(),
            ),
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

    /// Determine which schema to use based on AI response
    async fn determine_schema_to_use(
        &self,
        ai_response: &AISchemaResponse,
        sample_data: &Value,
        node: &DataFoldNode,
    ) -> IngestionResult<String> {
        // Always create a new schema from the AI definition
        if let Some(new_schema_def) = &ai_response.new_schemas {
            let schema_name = self
                .create_new_schema_with_node(new_schema_def, sample_data, node)
                .await?;
            return Ok(schema_name);
        }

        Err(IngestionError::ai_response_validation_error(
            "AI response did not provide a new schema definition",
        ))
    }

    /// Create a new schema using the DataFoldNode
    async fn create_new_schema_with_node(
        &self,
        schema_def: &Value,
        sample_data: &Value,
        node: &DataFoldNode,
    ) -> IngestionResult<String> {
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
                let sample_map: std::collections::HashMap<String, serde_json::Value> = sample_obj
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                schema.infer_topologies_from_data(&sample_map);
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "Inferred topologies for {} fields (no AI topologies)",
                    sample_map.len()
                );
            }
        }

        // Ensure default classifications for String fields (force indexing)
        // This must run AFTER inference to catch inferred fields
        for topology in schema.field_topologies.values_mut() {
            if let crate::schema::types::topology::TopologyNode::Primitive {
                value: crate::schema::types::topology::PrimitiveValueType::String,
                classifications,
            } = &mut topology.root
            {
                if classifications.is_none()
                    || classifications
                        .as_ref()
                        .map(|c| c.is_empty())
                        .unwrap_or(false)
                {
                    *classifications = Some(vec!["word".to_string()]);
                    crate::log_feature!(
                        crate::logging::features::LogFeature::Ingestion,
                        info,
                        "Added default 'word' classification to string field"
                    );
                }
            }
        }

        // Ensure schema has key configuration for mutations to work
        if schema.key.is_none() {
            // Use the first field as the hash key, or generate an ID field if no fields exist
            let hash_field = if let Some(fields) = &schema.fields {
                fields.first().cloned()
            } else if !schema.field_topologies.is_empty() {
                schema.field_topologies.keys().next().cloned()
            } else {
                None
            };

            if let Some(field_name) = hash_field {
                schema.key = Some(crate::schema::types::KeyConfig::new(
                    Some(field_name.clone()),
                    None,
                ));
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "Added default key configuration using field '{}' for schema",
                    field_name
                );
            } else {
                return Err(IngestionError::SchemaCreationError(
                    "Cannot create schema without at least one field for key configuration".to_string(),
                ));
            }
        }

        // Use topology_hash as schema name for structure-based deduplication
        schema.compute_schema_topology_hash();
        let topology_hash = schema
            .get_topology_hash()
            .ok_or_else(|| {
                IngestionError::SchemaCreationError(
                    "Schema must have topology_hash computed".to_string(),
                )
            })?
            .clone();

        schema.name = topology_hash.clone();

        // Add schema to the schema service via the node
        let schema_response = {
            node.add_schema_to_service(&schema).await.map_err(|error| {
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
            let db_guard = node
                .get_fold_db()
                .await
                .map_err(|error| IngestionError::SchemaCreationError(error.to_string()))?;
            let manager = db_guard.schema_manager.clone();
            drop(db_guard);
            manager
        };

        match schema_manager.load_schema_from_json(&json_str).await {
            Ok(_) => {}
            Err(error) => return Err(IngestionError::SchemaCreationError(error.to_string())),
        };

        // Auto-approve the new schema (idempotent - only approves if not already approved)
        schema_manager
            .approve(&schema_response.name)
            .await
            .map_err(|error| IngestionError::SchemaCreationError(error.to_string()))?;

        Ok(schema_response.name)
    }

    /// Execute mutations with progress tracking
    async fn execute_mutations_with_node_and_progress(
        &self,
        mutations: Vec<Mutation>,
        node: &DataFoldNode,
        progress_service: &ProgressService,
        progress_id: &str,
    ) -> IngestionResult<usize> {
        if mutations.is_empty() {
            return Ok(0);
        }

        let total_mutations = mutations.len();

        // Convert mutations to operation format for batch processing
        // Update progress for every 5 items to ensure visibility
        for (idx, _) in mutations.iter().enumerate() {
            // Update progress more frequently (every 5 items) to ensure frontend catches updates
            if (idx + 1) % 5 == 0 || idx + 1 == total_mutations {
                // Calculate progress: 90% base + up to 5% for this step (max 95%, not 100%)
                // 100% is reserved for the Completed step
                let percent_of_step = ((idx + 1) as f32 / total_mutations as f32 * 5.0) as u8;
                let progress_percent = 90 + percent_of_step;
                progress_service
                    .update_progress_with_percentage(
                        progress_id,
                        IngestionStep::ExecutingMutations,
                        format!("Executing mutations... ({}/{})", idx + 1, total_mutations),
                        progress_percent,
                    )
                    .await;
            }
        }

        // Execute all mutations in a batch using DataFoldNode directly
        // Use mutate_batch which publishes MutationExecuted events for the IndexOrchestrator
        node.mutate_batch(mutations)
            .await
            .map(|mutation_ids| mutation_ids.len())
            .map_err(|e| {
                IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                    e.to_string(),
                ))
            })
    }

    /// Execute mutations without progress tracking (for backward compatibility)
    async fn execute_mutations_with_node(
        &self,
        mutations: Vec<Mutation>,
        node: &DataFoldNode,
    ) -> IngestionResult<usize> {
        if mutations.is_empty() {
            return Ok(0);
        }

        // Execute all mutations in a batch using DataFoldNode directly
        // Use mutate_batch which publishes MutationExecuted events for the IndexOrchestrator
        node.mutate_batch(mutations)
            .await
            .map(|mutation_ids| mutation_ids.len())
            .map_err(|e| {
                IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                    e.to_string(),
                ))
            })
    }

    /// Flatten Twitter data structure for AI analysis
    /// Converts nested structures like [{"like": {"tweetId": "...", "fullText": "..."}}]
    /// to flattened structures like [{"tweetId": "...", "fullText": "..."}]
    fn flatten_twitter_data(&self, data: &Value) -> Value {
        if let Some(array) = data.as_array() {
            let flattened_items: Vec<Value> = array
                .iter()
                .map(|item| {
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
                })
                .collect();

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
}
