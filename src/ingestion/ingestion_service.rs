//! AI-powered ingestion service that works with FoldNode
//!
//! Handles JSON data ingestion with AI schema recommendation, mutation generation,
//! and execution. Refactored to take &FoldNode references for flexible locking.

use crate::fold_node::FoldNode;
use crate::ingestion::config::AIProvider;
use crate::ingestion::decomposer;
use crate::ingestion::IngestionRequest;
use crate::ingestion::mutation_generator;
use crate::ingestion::ollama_service::OllamaService;
use crate::ingestion::openrouter_service::OpenRouterService;
use crate::ingestion::progress::{IngestionResults, IngestionStep, ProgressService};
use crate::ingestion::{
    AISchemaResponse, IngestionConfig, IngestionError, IngestionResponse, IngestionResult,
    IngestionStatus,
};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::topology::{JsonTopology, TopologyNode};
use crate::schema::types::{KeyValue, Mutation};
use crate::schema::SchemaCore;
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Maximum recursion depth for decomposition to prevent unbounded nesting.
const MAX_DECOMPOSITION_DEPTH: usize = 10;

/// Try to normalize a date string to "YYYY-MM-DD HH:MM:SS" format for
/// chronological sorting. Returns the original string if it cannot be
/// parsed as a date.
fn try_normalize_date(value: &str) -> String {
    let trimmed = value.trim();

    // Already normalized — skip parsing
    if NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S").is_ok() {
        return trimmed.to_string();
    }

    // RFC 3339 / ISO 8601 with timezone (e.g. "2024-01-05T15:30:00Z", "2024-01-05T15:30:00+00:00")
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return dt.format("%Y-%m-%d %H:%M:%S").to_string();
    }

    // RFC 2822 (e.g. "Mon, 05 Jan 2024 15:30:00 +0000")
    // Try built-in first, then strip day-of-week prefix for lenient parsing
    // (source data may have incorrect day names).
    if let Ok(dt) = DateTime::parse_from_rfc2822(trimmed) {
        return dt.format("%Y-%m-%d %H:%M:%S").to_string();
    }
    if let Some(rest) = trimmed.split_once(", ").map(|(_, r)| r) {
        if let Ok(dt) = DateTime::<FixedOffset>::parse_from_str(rest, "%d %b %Y %H:%M:%S %z") {
            return dt.format("%Y-%m-%d %H:%M:%S").to_string();
        }
    }

    // Twitter format: "Mon Jan 05 15:30:00 +0000 2024"
    // chrono can't parse %z followed by %Y, so strip the tz offset and parse
    // the rest as naive datetime with the year moved.
    if let Some(dt) = try_parse_twitter_date(trimmed) {
        return dt.format("%Y-%m-%d %H:%M:%S").to_string();
    }

    // Timezone-aware formats
    let tz_formats = [
        "%Y-%m-%dT%H:%M:%S%z",        // "2024-01-05T15:30:00+0000"
        "%Y-%m-%dT%H:%M:%S%.f%z",     // "2024-01-05T15:30:00.000+0000"
    ];
    for fmt in &tz_formats {
        if let Ok(dt) = DateTime::<FixedOffset>::parse_from_str(trimmed, fmt) {
            return dt.format("%Y-%m-%d %H:%M:%S").to_string();
        }
    }

    // Naive datetime formats (no timezone)
    let naive_dt_formats = [
        "%Y-%m-%dT%H:%M:%S",          // "2024-01-05T15:30:00"
        "%m/%d/%Y %H:%M:%S",          // "01/05/2024 15:30:00"
        "%Y-%m-%d %H:%M",             // "2024-01-05 15:30"
    ];
    for fmt in &naive_dt_formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            return dt.format("%Y-%m-%d %H:%M:%S").to_string();
        }
    }

    // Date-only formats — normalize to midnight
    let date_formats = [
        "%Y-%m-%d",                    // "2024-01-05"
        "%m/%d/%Y",                    // "01/05/2024"
        "%B %d, %Y",                  // "January 5, 2024"
        "%b %d, %Y",                  // "Jan 5, 2024"
        "%d %B %Y",                   // "5 January 2024"
        "%d %b %Y",                   // "5 Jan 2024"
    ];
    for fmt in &date_formats {
        if let Ok(d) = NaiveDate::parse_from_str(trimmed, fmt) {
            return d.format("%Y-%m-%d 00:00:00").to_string();
        }
    }

    // Not a recognized date format — return original
    value.to_string()
}

/// Parse Twitter-style dates: "Mon Jan 05 15:30:00 +0000 2024"
/// Skips the day-of-week name and timezone offset, parses the rest.
/// This avoids chrono's strict day-of-week validation (source data may
/// have incorrect day names).
fn try_parse_twitter_date(value: &str) -> Option<NaiveDateTime> {
    // Pattern: "DDD MMM DD HH:MM:SS +ZZZZ YYYY"
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() != 6 {
        return None;
    }
    // parts[4] should be a timezone offset like "+0000"
    let tz_part = parts[4];
    if !(tz_part.starts_with('+') || tz_part.starts_with('-')) || tz_part.len() != 5 {
        return None;
    }
    // Skip day-of-week (parts[0]) and timezone (parts[4]):
    // "Jan 05 15:30:00 2024"
    let without_dow_tz = format!("{} {} {} {}", parts[1], parts[2], parts[3], parts[5]);
    NaiveDateTime::parse_from_str(&without_dow_tz, "%b %d %H:%M:%S %Y").ok()
}

/// Cached result of AI schema determination for a given structure.
struct CachedSchema {
    schema_name: String,
    mutation_mappers: HashMap<String, String>,
}

/// Extract key values from JSON data based on schema key fields.
/// Looks up the schema in the node's schema manager to find key configuration,
/// then extracts the corresponding values from the data.
async fn extract_key_values_from_data(
    fields_and_values: &HashMap<String, Value>,
    schema_name: &str,
    schema_manager: &Arc<SchemaCore>,
) -> IngestionResult<HashMap<String, String>> {
    let mut keys_and_values = HashMap::new();

    match schema_manager.get_schema(schema_name) {
        Ok(Some(schema)) => {
            if let Some(key_def) = &schema.key {
                // Extract hash field value if present
                if let Some(hash_field) = &key_def.hash_field {
                    if let Some(hash_value) = fields_and_values.get(hash_field) {
                        if let Some(hash_str) = hash_value.as_str() {
                            keys_and_values.insert("hash_field".to_string(), hash_str.to_string());
                        } else if let Some(hash_num) = hash_value.as_f64() {
                            keys_and_values.insert("hash_field".to_string(), hash_num.to_string());
                        } else {
                            log_feature!(
                                LogFeature::Ingestion,
                                warn,
                                "Hash field '{}' in schema '{}' has unsupported type (not string or number): {:?}",
                                hash_field, schema_name, hash_value
                            );
                        }
                    } else {
                        log_feature!(
                            LogFeature::Ingestion,
                            warn,
                            "Hash field '{}' not found in data for schema '{}'",
                            hash_field, schema_name
                        );
                    }
                }

                // Extract range field value if present, normalizing dates to
                // YYYY-MM-DD HH:MM:SS so records sort chronologically.
                if let Some(range_field) = &key_def.range_field {
                    if let Some(range_value) =
                        extract_nested_field_value(fields_and_values, range_field)
                    {
                        if let Some(range_str) = range_value.as_str() {
                            keys_and_values.insert("range_field".to_string(), try_normalize_date(range_str));
                        } else if let Some(range_num) = range_value.as_f64() {
                            keys_and_values.insert("range_field".to_string(), range_num.to_string());
                        } else {
                            log_feature!(
                                LogFeature::Ingestion,
                                warn,
                                "Range field '{}' in schema '{}' has unsupported type (not string or number): {:?}",
                                range_field, schema_name, range_value
                            );
                        }
                    } else {
                        log_feature!(
                            LogFeature::Ingestion,
                            warn,
                            "Range field '{}' not found in data for schema '{}'",
                            range_field, schema_name
                        );
                    }
                }
            }
        }
        Ok(None) => {
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "Schema '{}' not found — cannot extract key values",
                schema_name
            );
        }
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Failed to get schema '{}' for key extraction: {}",
                schema_name, e
            );
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

            // Collect items: either array elements or the single object
            let items: Vec<Value> = if let Some(arr) = flattened_data.as_array() {
                arr.clone()
            } else {
                vec![flattened_data.clone()]
            };

            // Resolve schemas for the representative's structure tree.
            // The top-level item's own structure hash is used to cache its
            // flat-parent schema (after array-of-object fields are removed).
            let rep = representative_for_decompose.as_ref().unwrap();
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
                        request.auto_execute,
                        0,
                    )
                    .await?;
                total_mutations_generated += gen;
                total_mutations_executed += exec;
            }

            // Determine the top-level schema name for the response
            let top_schema_name = schema_cache
                .get(&top_level_hash)
                .map(|c| c.schema_name.clone())
                .unwrap_or_default();

            // Mark as completed
            let results = IngestionResults {
                schema_name: top_schema_name.clone(),
                new_schema_created: true,
                mutations_generated: total_mutations_generated,
                mutations_executed: total_mutations_executed,
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
            ))
        } else {
            // ── Original flat path (no nested arrays of objects) ──

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
    async fn get_ai_recommendation(
        &self,
        json_data: &Value,
    ) -> IngestionResult<AISchemaResponse> {
        use super::ai_helpers::{analyze_and_build_prompt, parse_ai_response};

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
    async fn determine_schema_to_use(
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

    /// Resolve the schema for a given structure hash.
    ///
    /// If not cached, decomposes the representative item (recursively resolving
    /// its own children), then sends the flat representative to AI to determine
    /// the schema. Caches the result for reuse by all items sharing the same
    /// structure.
    async fn resolve_schema_for_structure(
        &self,
        structure_hash: &str,
        representative: &Value,
        schema_cache: &mut HashMap<String, CachedSchema>,
        node: &FoldNode,
        depth: usize,
    ) -> IngestionResult<String> {
        // Return cached result if available
        if let Some(cached) = schema_cache.get(structure_hash) {
            return Ok(cached.schema_name.clone());
        }

        // Decompose the representative to handle its own nested children
        let rep_decomp = decomposer::decompose(representative);

        // Depth guard: if we've recursed too deep, skip children and treat as flat
        if depth >= MAX_DECOMPOSITION_DEPTH {
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "Decomposition depth limit ({}) reached for structure hash '{}' — treating as flat",
                MAX_DECOMPOSITION_DEPTH,
                structure_hash
            );
        } else {
            // Recursively resolve schemas for the representative's children (depth-first)
            for child_group in &rep_decomp.children {
                Box::pin(self.resolve_schema_for_structure(
                    &child_group.structure_hash,
                    &child_group.items[0],
                    schema_cache,
                    node,
                    depth + 1,
                ))
                .await?;
            }
        }

        // Get AI recommendation for the flat parent (no array-of-object fields)
        let ai_response = self.get_ai_recommendation(&rep_decomp.parent).await?;

        // Create the schema via the standard path
        let schema_name = self
            .determine_schema_to_use(&ai_response, &rep_decomp.parent, node)
            .await?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Cached schema '{}' for structure hash {}",
            schema_name,
            structure_hash
        );

        schema_cache.insert(
            structure_hash.to_string(),
            CachedSchema {
                schema_name: schema_name.clone(),
                mutation_mappers: ai_response.mutation_mappers,
            },
        );

        // Update the parent schema with Reference topologies for each decomposed child field.
        // Children are resolved depth-first above, so their schema names are already in the cache.
        // Only do this when we actually resolved children (not at depth limit).
        if !rep_decomp.children.is_empty() && depth < MAX_DECOMPOSITION_DEPTH {
            let schema_manager = {
                let db_guard = node
                    .get_fold_db()
                    .await
                    .map_err(|error| IngestionError::SchemaCreationError(error.to_string()))?;
                let manager = db_guard.schema_manager.clone();
                drop(db_guard);
                manager
            };

            match schema_manager.get_schema(&schema_name) {
                Ok(Some(mut schema)) => {
                    for child_group in &rep_decomp.children {
                        let child_schema_name = schema_cache
                            .get(&child_group.structure_hash)
                            .map(|c| c.schema_name.clone())
                            .ok_or_else(|| {
                                IngestionError::SchemaCreationError(format!(
                                    "No cached schema for child structure hash '{}' (field '{}')",
                                    child_group.structure_hash, child_group.field_name
                                ))
                            })?;
                        schema.set_field_topology(
                            child_group.field_name.clone(),
                            JsonTopology::new(TopologyNode::Reference {
                                schema_name: child_schema_name,
                            }),
                        );

                        // Register the Reference field as a queryable schema field
                        if let Some(ref mut fields) = schema.fields {
                            if !fields.contains(&child_group.field_name) {
                                fields.push(child_group.field_name.clone());
                            }
                        } else {
                            schema.fields = Some(vec![child_group.field_name.clone()]);
                        }
                    }

                    if let Err(e) = schema.populate_runtime_fields() {
                        log_feature!(
                            LogFeature::Ingestion,
                            warn,
                            "Failed to populate runtime fields for schema '{}': {}",
                            schema_name,
                            e
                        );
                    }

                    schema_manager.update_schema(&schema).await.map_err(|e| {
                        IngestionError::SchemaCreationError(format!(
                            "Failed to update schema with Reference topologies: {}",
                            e
                        ))
                    })?;
                }
                Ok(None) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        warn,
                        "Schema '{}' not found when updating Reference topologies — child references will not be linked",
                        schema_name
                    );
                }
                Err(e) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        error,
                        "Failed to get schema '{}' for Reference topology update: {}",
                        schema_name,
                        e
                    );
                }
            }
        }

        Ok(schema_name)
    }

    /// Process a single item through decomposition: recursively handle its
    /// children, then generate and execute a mutation for the flat parent.
    ///
    /// `structure_hash` is the topology hash of the full item (before decomposition),
    /// matching the key used in `resolve_schema_for_structure`.
    ///
    /// Returns (mutations_generated, mutations_executed, own_key_value).
    /// The third element is the `key_value` from the mutation generated for this item
    /// (None if item was empty). This lets the caller (parent) build references to this child.
    #[allow(clippy::too_many_arguments)]
    async fn ingest_decomposed_item(
        &self,
        item: &Value,
        structure_hash: &str,
        schema_cache: &mut HashMap<String, CachedSchema>,
        node: &FoldNode,
        trust_distance: u32,
        pub_key: &str,
        source_file_name: Option<String>,
        auto_execute: bool,
        depth: usize,
    ) -> IngestionResult<(usize, usize, Option<KeyValue>)> {
        let item_decomp = decomposer::decompose(item);
        let mut total_gen: usize = 0;
        let mut total_exec: usize = 0;

        // Recursively process each child group's items and collect references.
        let mut child_references: HashMap<String, Vec<Value>> = HashMap::new();

        // Skip children if depth limit reached
        if depth >= MAX_DECOMPOSITION_DEPTH {
            log_feature!(
                LogFeature::Ingestion,
                warn,
                "Decomposition depth limit ({}) reached during ingestion for structure hash '{}' — skipping children",
                MAX_DECOMPOSITION_DEPTH,
                structure_hash
            );
        } else {
        for child_group in &item_decomp.children {
            let mut refs_for_field = Vec::new();

            for child_item in &child_group.items {
                let (gen, exec, child_key_value) = Box::pin(self.ingest_decomposed_item(
                    child_item,
                    &child_group.structure_hash,
                    schema_cache,
                    node,
                    trust_distance,
                    pub_key,
                    source_file_name.clone(),
                    auto_execute,
                    depth + 1,
                ))
                .await?;
                total_gen += gen;
                total_exec += exec;

                // Build reference matching the indexing system's (schema, key) pattern
                if let Some(kv) = child_key_value {
                    let child_schema_name = schema_cache
                        .get(&child_group.structure_hash)
                        .map(|c| c.schema_name.clone())
                        .ok_or_else(|| {
                            IngestionError::SchemaCreationError(format!(
                                "No cached schema for child structure hash '{}' (field '{}')",
                                child_group.structure_hash, child_group.field_name
                            ))
                        })?;
                    refs_for_field.push(serde_json::json!({
                        "schema": child_schema_name,
                        "key": kv,
                    }));
                } else {
                    log_feature!(
                        LogFeature::Ingestion,
                        warn,
                        "Child item in field '{}' (structure hash {}) produced no key_value — reference will be missing",
                        child_group.field_name,
                        child_group.structure_hash
                    );
                }
            }

            child_references.insert(child_group.field_name.clone(), refs_for_field);
        }
        } // end depth guard else

        // Generate and execute mutation for this item's flat parent.
        // Use the structure_hash passed in (hash of full item before decomposition)
        // to look up the cached schema — this matches the key from resolve_schema_for_structure.
        let mut parent = item_decomp.parent;

        // Inject child references into the parent data before mutation generation
        if let Some(parent_obj) = parent.as_object_mut() {
            for (field_name, refs) in &child_references {
                if !refs.is_empty() {
                    parent_obj.insert(field_name.clone(), Value::Array(refs.clone()));
                }
            }
        }

        let mut own_key_value: Option<KeyValue> = None;

        if let Some(parent_obj) = parent.as_object() {
            if parent_obj.is_empty() {
                return Ok((total_gen, total_exec, None));
            }

            // If this item's structure differs from the representative (e.g., empty
            // vs. non-empty nested arrays), the structure hash won't be cached yet.
            // Resolve it on the fly so the schema and mutation mappers are available.
            if !schema_cache.contains_key(structure_hash) {
                Box::pin(self.resolve_schema_for_structure(
                    structure_hash,
                    item,
                    schema_cache,
                    node,
                    depth,
                ))
                .await?;
            }

            let cached = schema_cache.get(structure_hash).ok_or_else(|| {
                IngestionError::SchemaCreationError(format!(
                    "No cached schema for structure hash {}",
                    structure_hash
                ))
            })?;

            let schema_name = cached.schema_name.clone();
            let mut mutation_mappers = cached.mutation_mappers.clone();

            // Add identity mappers for Reference fields so generate_mutations includes them
            for (field_name, refs) in &child_references {
                if !refs.is_empty() && !mutation_mappers.contains_key(field_name) {
                    mutation_mappers.insert(field_name.clone(), field_name.clone());
                }
            }

            let fields_and_values: HashMap<String, Value> = parent_obj
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

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

            let keys_and_values =
                extract_key_values_from_data(&fields_and_values, &schema_name, &schema_manager)
                    .await?;

            let mutations = mutation_generator::generate_mutations(
                &schema_name,
                &keys_and_values,
                &fields_and_values,
                &mutation_mappers,
                trust_distance,
                pub_key.to_string(),
                source_file_name,
            )?;

            // Extract the key_value from the first mutation before execution
            own_key_value = mutations.first().map(|m| m.key_value.clone());

            let gen_count = mutations.len();
            total_gen += gen_count;

            if auto_execute && !mutations.is_empty() {
                let exec_count = node
                    .mutate_batch(mutations)
                    .await
                    .map(|ids| ids.len())
                    .map_err(|e| {
                        IngestionError::SchemaSystemError(crate::schema::SchemaError::InvalidData(
                            e.to_string(),
                        ))
                    })?;
                total_exec += exec_count;
            }
        }

        Ok((total_gen, total_exec, own_key_value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_twitter_date() {
        // Correct day-of-week
        assert_eq!(
            try_normalize_date("Fri Jan 05 15:30:00 +0000 2024"),
            "2024-01-05 15:30:00"
        );
        assert_eq!(
            try_normalize_date("Fri Dec 20 08:45:12 +0000 2024"),
            "2024-12-20 08:45:12"
        );
        // Incorrect day-of-week (should still parse — real data may be wrong)
        assert_eq!(
            try_normalize_date("Mon Jan 05 15:30:00 +0000 2024"),
            "2024-01-05 15:30:00"
        );
    }

    #[test]
    fn test_normalize_iso8601() {
        assert_eq!(
            try_normalize_date("2024-01-05T15:30:00+0000"),
            "2024-01-05 15:30:00"
        );
        assert_eq!(
            try_normalize_date("2024-01-05T15:30:00"),
            "2024-01-05 15:30:00"
        );
    }

    #[test]
    fn test_normalize_already_normalized() {
        assert_eq!(
            try_normalize_date("2024-01-05 15:30:00"),
            "2024-01-05 15:30:00"
        );
    }

    #[test]
    fn test_normalize_date_only() {
        assert_eq!(
            try_normalize_date("2024-01-05"),
            "2024-01-05 00:00:00"
        );
        assert_eq!(
            try_normalize_date("January 5, 2024"),
            "2024-01-05 00:00:00"
        );
    }

    #[test]
    fn test_normalize_rfc2822() {
        // Correct day-of-week
        assert_eq!(
            try_normalize_date("Fri, 05 Jan 2024 15:30:00 +0000"),
            "2024-01-05 15:30:00"
        );
        // Incorrect day-of-week (lenient parsing)
        assert_eq!(
            try_normalize_date("Mon, 05 Jan 2024 15:30:00 +0000"),
            "2024-01-05 15:30:00"
        );
    }

    #[test]
    fn test_normalize_non_date() {
        assert_eq!(try_normalize_date("not-a-date"), "not-a-date");
        assert_eq!(try_normalize_date("12345"), "12345");
        assert_eq!(try_normalize_date("hello world"), "hello world");
    }

    #[test]
    fn test_normalize_chronological_ordering() {
        // These Twitter-format dates sort incorrectly by day name:
        // "Fri..." < "Mon..." < "Wed..." alphabetically
        let dates = vec![
            "Wed Jan 01 00:00:00 +0000 2025",
            "Fri Jan 03 00:00:00 +0000 2025",
            "Mon Jan 06 00:00:00 +0000 2025",
        ];
        let mut normalized: Vec<String> = dates.iter().map(|d| try_normalize_date(d)).collect();
        let sorted = normalized.clone();
        normalized.sort();
        assert_eq!(normalized, sorted, "Normalized dates should already be in chronological order");
    }
}

