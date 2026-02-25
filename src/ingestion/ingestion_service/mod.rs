//! AI-powered ingestion service that works with FoldNode
//!
//! Handles JSON data ingestion with AI schema recommendation, mutation generation,
//! and execution. Refactored to take &FoldNode references for flexible locking.

mod decomposition;

use crate::fold_node::{FileIngestionRecord, FoldNode};
use crate::ingestion::config::AIProvider;
use crate::ingestion::decomposer;
use crate::ingestion::key_extraction::extract_key_values_from_data;
use crate::ingestion::mutation_generator;
use crate::ingestion::ollama_service::OllamaService;
use crate::ingestion::openrouter_service::OpenRouterService;
use crate::ingestion::progress::{
    IngestionResults, IngestionStep, ProgressService, SchemaWriteRecord,
};
use crate::ingestion::IngestionRequest;
use crate::ingestion::{
    AISchemaResponse, IngestionConfig, IngestionError, IngestionResponse, IngestionResult,
    IngestionStatus,
};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::{KeyValue, Mutation};
use serde_json::Value;
use std::collections::HashMap;

use decomposition::CachedSchema;

/// AI-powered ingestion service that works with FoldNode
pub struct IngestionService {
    config: IngestionConfig,
    openrouter_service: Option<OpenRouterService>,
    ollama_service: Option<OllamaService>,
}

impl IngestionService {
    /// Create an ingestion service from environment configuration
    pub fn from_env() -> IngestionResult<Self> {
        let config = IngestionConfig::from_env()?;
        Self::new(config)
    }

    /// Create a new ingestion service.
    /// Provider services are initialised best-effort: if validation fails
    /// (e.g. missing API key) the service is still created so that
    /// `get_status()` can report the correct provider/model — actual
    /// ingestion calls will fail at runtime with a clear error.
    pub fn new(config: IngestionConfig) -> IngestionResult<Self> {
        let openrouter_service = if config.provider == AIProvider::OpenRouter {
            match OpenRouterService::new(
                config.openrouter.clone(),
                config.timeout_seconds,
                config.max_retries,
            ) {
                Ok(svc) => Some(svc),
                Err(e) => {
                    log::warn!("OpenRouter service init skipped: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let ollama_service = if config.provider == AIProvider::Ollama {
            match OllamaService::new(
                config.ollama.clone(),
                config.timeout_seconds,
                config.max_retries,
            ) {
                Ok(svc) => Some(svc),
                Err(e) => {
                    log::warn!("Ollama service init skipped: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            config,
            openrouter_service,
            ollama_service,
        })
    }

    /// Process JSON ingestion using a FoldNode with progress tracking
    /// Accepts a reference to FoldNode, making it compatible with both Mutex and RwLock guards
    pub async fn process_json_with_node_and_progress(
        &self,
        request: IngestionRequest,
        node: &FoldNode,
        progress_service: &ProgressService,
        progress_id: String,
    ) -> IngestionResult<IngestionResponse> {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Starting JSON ingestion process with FoldNode (progress_id: {})",
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
        let flattened_data = crate::ingestion::json_processor::flatten_root_layers(request.data.clone());

        // Extract common mutation parameters
        let trust_distance = request.trust_distance;
        let pub_key = request.pub_key.clone();

        // Step 2.5: Decompose nested structures and decide code path
        //
        // For top-level arrays, we decompose the first element to check for
        // nested arrays-of-objects. For single objects, we decompose directly.
        let representative_for_decompose = if let Some(arr) = flattened_data.as_array() {
            arr.first().cloned()
        } else {
            Some(flattened_data.clone())
        };

        let has_nested_children = representative_for_decompose
            .as_ref()
            .map(|rep| !decomposer::decompose(rep).children.is_empty())
            .unwrap_or(false);

        if has_nested_children {
            // ── Recursive decomposition path ──
            progress_service
                .update_progress(
                    &progress_id,
                    IngestionStep::GettingAIRecommendation,
                    "Decomposing nested data structures...".to_string(),
                )
                .await;

            let mut schema_cache: HashMap<String, CachedSchema> = HashMap::new();
            let mut total_mutations_generated: usize = 0;
            let mut total_mutations_executed: usize = 0;
            let mut schemas_written_map: HashMap<String, Vec<KeyValue>> = HashMap::new();

            // Collect items: either array elements or the single object
            let items: Vec<Value> = if let Some(arr) = flattened_data.as_array() {
                arr.clone()
            } else {
                vec![flattened_data.clone()]
            };

            // Resolve schemas for the representative's structure tree.
            // The top-level item's own structure hash is used to cache its
            // flat-parent schema (after array-of-object fields are removed).
            let rep = representative_for_decompose.as_ref()
                .expect("representative_for_decompose is Some when has_nested_children is true");
            let top_level_topology = crate::schema::types::topology::JsonTopology::infer_from_value(rep);
            let top_level_hash = top_level_topology.compute_hash();
            self.resolve_schema_for_structure(
                &top_level_hash,
                rep,
                &mut schema_cache,
                node,
                0,
            )
            .await?;

            // Build metadata from file_hash if present
            let decomposed_metadata = request.file_hash.as_ref().map(|hash| {
                let mut meta = HashMap::new();
                meta.insert("file_hash".to_string(), hash.clone());
                meta
            });

            // Process each item: recursively handle children, then generate parent mutation.
            // Pass the top-level structure hash so the cache lookup matches.
            for item in &items {
                let (gen, exec, _key_value) = self
                    .ingest_decomposed_item(
                        item,
                        &top_level_hash,
                        &mut schema_cache,
                        node,
                        trust_distance,
                        &pub_key,
                        request.source_file_name.clone(),
                        decomposed_metadata.clone(),
                        request.auto_execute,
                        0,
                        &mut schemas_written_map,
                    )
                    .await?;
                total_mutations_generated += gen;
                total_mutations_executed += exec;
            }

            // Determine the top-level schema name for the response
            let top_schema_name = schema_cache
                .get(&top_level_hash)
                .map(|c| c.schema_name.clone())
                .unwrap_or_else(|| {
                    log::warn!("Schema cache miss for top-level hash '{}' — returning empty schema name", top_level_hash);
                    String::new()
                });

            // Build schemas_written from accumulator
            let schemas_written: Vec<SchemaWriteRecord> = schemas_written_map
                .into_iter()
                .map(|(name, keys)| SchemaWriteRecord {
                    schema_name: name,
                    keys_written: keys,
                })
                .collect();

            // Record file as ingested for this user (per-user file-level dedup)
            if let Some(ref fh) = request.file_hash {
                let record = FileIngestionRecord {
                    ingested_at: chrono::Utc::now().to_rfc3339(),
                    source_folder: request.source_folder.clone(),
                    source_file_name: request.source_file_name.clone(),
                    progress_id: request.progress_id.clone(),
                };
                if let Err(e) = node.mark_file_ingested(&request.pub_key, fh, record).await {
                    log::warn!("Failed to record file dedup entry: {}", e);
                }
            }

            // Mark as completed
            let results = IngestionResults {
                schema_name: top_schema_name.clone(),
                new_schema_created: true,
                mutations_generated: total_mutations_generated,
                mutations_executed: total_mutations_executed,
                schemas_written: schemas_written.clone(),
            };
            progress_service
                .complete_progress(&progress_id, results)
                .await;

            Ok(IngestionResponse::success_with_progress(
                progress_id,
                top_schema_name,
                true,
                total_mutations_generated,
                total_mutations_executed,
                schemas_written,
            ))
        } else {
            // ── Original flat path (no nested arrays of objects) ──

            // Step 3: Get AI recommendation
            let is_image = request
                .source_file_name
                .as_ref()
                .map(|name| crate::ingestion::is_image_file(name))
                .unwrap_or(false);
            progress_service
                .update_progress(
                    &progress_id,
                    IngestionStep::GettingAIRecommendation,
                    "Analyzing data with AI to determine schema...".to_string(),
                )
                .await;
            let mut ai_response = self.get_ai_recommendation(&flattened_data).await?;

            // CRITICAL: Images MUST use HashRange(image_type, created_at).
            // Without this, the AI would pick a Single or Range schema, and
            // multiple images would overwrite each other in the same schema.
            // HashRange lets images coexist: partitioned by type, ordered by date.
            if is_image {
                if let Some(ref mut schema_def) = ai_response.new_schemas {
                    schema_def["schema_type"] = serde_json::json!("HashRange");
                    schema_def["key"] = serde_json::json!({
                        "hash_field": "image_type",
                        "range_field": "created_at"
                    });
                    // Use the vision model's descriptive_name for a better schema display name
                    if let Some(desc) = flattened_data.get("descriptive_name").and_then(|v| v.as_str()) {
                        schema_def["descriptive_name"] = serde_json::json!(desc);
                    }
                }
            }

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

            // Build metadata from file_hash if present
            let metadata = request.file_hash.as_ref().map(|hash| {
                let mut meta = HashMap::new();
                meta.insert("file_hash".to_string(), hash.clone());
                meta
            });

            // Collect items to process — normalize single object to a one-element slice
            let items: Vec<&serde_json::Map<String, Value>> = if let Some(array) = flattened_data.as_array() {
                array
                    .iter()
                    .filter_map(|item| item.as_object())
                    .collect()
            } else if let Some(obj) = flattened_data.as_object() {
                vec![obj]
            } else {
                vec![]
            };

            let total_items = items.len();
            let mut mutations = Vec::new();
            for (idx, obj) in items.into_iter().enumerate() {
                let fields_and_values: HashMap<String, Value> =
                    obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

                let keys_and_values = extract_key_values_from_data(
                    &fields_and_values,
                    &schema_name,
                    &schema_manager,
                )
                .await?;

                let item_mutations = mutation_generator::generate_mutations(
                    &schema_name,
                    &keys_and_values,
                    &fields_and_values,
                    &ai_response.mutation_mappers,
                    trust_distance,
                    pub_key.clone(),
                    request.source_file_name.clone(),
                    metadata.clone(),
                )?;

                mutations.extend(item_mutations);

                // Update progress every 10 items (only meaningful for arrays)
                if total_items > 1 && ((idx + 1) % 10 == 0 || idx + 1 == total_items) {
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

            log_feature!(
                LogFeature::Ingestion,
                info,
                "Generated {} mutations",
                mutations.len()
            );

            // Collect schemas_written from generated mutations
            let schemas_written = {
                let mut map: HashMap<String, Vec<KeyValue>> = HashMap::new();
                for m in &mutations {
                    map.entry(m.schema_name.clone())
                        .or_default()
                        .push(m.key_value.clone());
                }
                map.into_iter()
                    .map(|(name, keys)| SchemaWriteRecord {
                        schema_name: name,
                        keys_written: keys,
                    })
                    .collect::<Vec<_>>()
            };

            // Step 6: Execute mutations if requested
            progress_service
                .update_progress(
                    &progress_id,
                    IngestionStep::ExecutingMutations,
                    "Executing mutations to store data...".to_string(),
                )
                .await;

            let mutations_len = mutations.len();

            let mutations_executed = if request.auto_execute {
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

            // Record file as ingested for this user (per-user file-level dedup)
            if let Some(ref fh) = request.file_hash {
                let record = FileIngestionRecord {
                    ingested_at: chrono::Utc::now().to_rfc3339(),
                    source_folder: request.source_folder.clone(),
                    source_file_name: request.source_file_name.clone(),
                    progress_id: request.progress_id.clone(),
                };
                if let Err(e) = node.mark_file_ingested(&request.pub_key, fh, record).await {
                    log::warn!("Failed to record file dedup entry: {}", e);
                }
            }

            // Mark as completed
            let results = IngestionResults {
                schema_name: schema_name.clone(),
                new_schema_created,
                mutations_generated: mutations_len,
                mutations_executed,
                schemas_written: schemas_written.clone(),
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
                schemas_written,
            ))
        }
    }

    /// Call the underlying AI API with a raw prompt string.
    ///
    /// This is the low-level API used by smart_folder scanning and other
    /// components that need raw AI text completion without schema parsing.
    pub async fn call_ai_raw(&self, prompt: &str) -> IngestionResult<String> {
        match self.config.provider {
            AIProvider::OpenRouter => {
                self.openrouter_service
                    .as_ref()
                    .ok_or_else(|| {
                        IngestionError::configuration_error("OpenRouter service not initialized")
                    })?
                    .call_openrouter_api(prompt)
                    .await
            }
            AIProvider::Ollama => {
                self.ollama_service
                    .as_ref()
                    .ok_or_else(|| {
                        IngestionError::configuration_error("Ollama service not initialized")
                    })?
                    .call_ollama_api(prompt)
                    .await
            }
        }
    }

    /// Get AI schema recommendation with validation retries.
    ///
    /// Builds the prompt once, then retries the AI call if response parsing fails
    /// (e.g., malformed JSON, missing required fields). Network-level retries are
    /// handled separately inside `call_ai_raw`.
    pub(super) async fn get_ai_recommendation(
        &self,
        json_data: &Value,
    ) -> IngestionResult<AISchemaResponse> {
        use crate::ingestion::ai_helpers::{analyze_and_build_prompt, parse_ai_response};

        let prompt = analyze_and_build_prompt(json_data)?;
        let max_validation_attempts = self.config.max_retries.clamp(1, 3);
        let mut last_error = None;

        for attempt in 1..=max_validation_attempts {
            let raw_response = self.call_ai_raw(&prompt).await?;

            match parse_ai_response(&raw_response) {
                Ok(response) => return Ok(response),
                Err(e) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        warn,
                        "AI response validation failed on attempt {}/{}: {}",
                        attempt,
                        max_validation_attempts,
                        e
                    );
                    last_error = Some(e);

                    if attempt < max_validation_attempts {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            IngestionError::ai_response_validation_error(
                "All AI attempts returned invalid responses",
            )
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
    pub(super) async fn determine_schema_to_use(
        &self,
        ai_response: &AISchemaResponse,
        sample_data: &Value,
        node: &FoldNode,
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

    /// Create a new schema using the FoldNode
    async fn create_new_schema_with_node(
        &self,
        schema_def: &Value,
        sample_data: &Value,
        node: &FoldNode,
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
        node: &FoldNode,
        progress_service: &ProgressService,
        progress_id: &str,
    ) -> IngestionResult<usize> {
        if mutations.is_empty() {
            return Ok(0);
        }

        let total_mutations = mutations.len();

        progress_service
            .update_progress_with_percentage(
                progress_id,
                IngestionStep::ExecutingMutations,
                format!("Submitting {} mutations...", total_mutations),
                90,
            )
            .await;

        // Execute all mutations in a batch using FoldNode directly
        // mutate_batch runs the MutationPreprocessor (keyword extraction) then writes
        let result = node.mutate_batch(mutations)
            .await
            .map(|mutation_ids| mutation_ids.len())
            .map_err(|e| {
                IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                    e.to_string(),
                ))
            });

        if let Ok(count) = &result {
            progress_service
                .update_progress_with_percentage(
                    progress_id,
                    IngestionStep::ExecutingMutations,
                    format!("Completed {} mutations", count),
                    95,
                )
                .await;
        }

        result
    }
}

