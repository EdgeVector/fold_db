use crate::error::{FoldDbError, FoldDbResult};
use crate::ingestion::ingestion_service::IngestionService;
use crate::schema::types::{KeyValue, Mutation, Query};
#[cfg(test)]
use crate::schema::types::field::HashRangeFilter;
use crate::schema::types::topology::TopologyNode;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::response_types::QueryResultMap;
use super::FoldNode;
use crate::fold_node::config::DatabaseConfig;
use crate::fold_node::NodeConfig;
use crate::db_operations::IndexResult;
use crate::fold_db_core::infrastructure::backfill_tracker::{
    BackfillInfo, BackfillStatistics, BackfillStatus,
};
use crate::fold_db_core::orchestration::IndexingStatus;
use crate::schema::types::operations::{MutationType, Operation};
use crate::schema::types::Transform;
use crate::schema::{SchemaState, SchemaWithState};
use std::fs;
use std::io::Write;

/// Centralized operation processor that handles all operation types consistently.
///
/// This eliminates code duplication across HTTP routes, TCP server, CLI, and direct API usage.
/// All operation execution goes through this single processor to ensure consistent behavior.
pub struct OperationProcessor {
    node: FoldNode,
}

impl OperationProcessor {
    /// Creates a new operation processor with a FoldNode instance.
    pub fn new(node: FoldNode) -> Self {
        Self { node }
    }

    /// Executes a query and returns raw structured results, not JSON.
    pub async fn execute_query_map(&self, query: Query) -> FoldDbResult<QueryResultMap> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        let results = db.query_executor.query(query).await;
        Ok(results?)
    }

    /// Executes a query and returns formatted JSON records.
    /// This provides a consistent JSON representation for API responses.
    /// When `rehydrate_depth` is set on the query, Reference fields are automatically
    /// resolved to their actual child records up to the specified depth.
    pub async fn execute_query_json(&self, query: Query) -> FoldDbResult<Vec<Value>> {
        self.execute_query_json_internal(query, HashSet::new()).await
    }

    /// Internal implementation that threads a visited-schema set to detect circular references.
    async fn execute_query_json_internal(
        &self,
        query: Query,
        visited: HashSet<String>,
    ) -> FoldDbResult<Vec<Value>> {
        let schema_name = query.schema_name.clone();
        let rehydrate_depth = query.rehydrate_depth;

        let result_map = self.execute_query_map(query).await?;
        let records_map = crate::fold_db_core::query::records_from_field_map(&result_map);

        let mut results: Vec<Value> = records_map
            .into_iter()
            .map(|(key, record)| {
                serde_json::json!({
                    "key": key,
                    "fields": record.fields,
                    "metadata": record.metadata
                })
            })
            .collect();

        if let Some(depth) = rehydrate_depth {
            if depth > 0 {
                self.rehydrate_references(&mut results, &schema_name, depth, visited).await?;
            }
        }

        Ok(results)
    }

    // --- Reference Rehydration ---

    /// Post-processes query results to resolve Reference fields into actual child records.
    /// Recurses up to `remaining_depth` levels deep.
    /// Uses `Box::pin` to handle async recursion through `execute_query_json_internal`.
    /// The `visited` set tracks ancestor schemas to prevent infinite loops on circular references.
    fn rehydrate_references<'a>(
        &'a self,
        results: &'a mut [Value],
        schema_name: &'a str,
        remaining_depth: u32,
        visited: HashSet<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = FoldDbResult<()>> + 'a>> {
        Box::pin(async move {
            // Circular reference guard: if we've already visited this schema in the
            // current ancestor chain, stop recursion to avoid infinite loops.
            if visited.contains(schema_name) {
                log::debug!(
                    "Circular reference detected for schema '{}', stopping rehydration",
                    schema_name
                );
                return Ok(());
            }

            let mut visited = visited;
            visited.insert(schema_name.to_string());

            // Collect all schema metadata we need upfront, then drop the db guard
            // before making recursive queries (which also need the guard).
            let (ref_fields, child_field_map, child_key_config_map) = {
                let db = self
                    .node
                    .get_fold_db()
                    .await
                    .map_err(|e| FoldDbError::Database(e.to_string()))?;

                let schema = match db
                    .schema_manager
                    .get_schema(schema_name)
                    .map_err(|e| FoldDbError::Database(e.to_string()))?
                {
                    Some(s) => s,
                    None => return Ok(()),
                };

                // Find fields with Reference topology
                let ref_fields: Vec<(String, String)> = schema
                    .field_topologies
                    .iter()
                    .filter_map(|(field_name, topo)| {
                        if let TopologyNode::Reference { schema_name } = &topo.root {
                            Some((field_name.clone(), schema_name.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();

                if ref_fields.is_empty() {
                    return Ok(());
                }

                // Pre-fetch queryable fields and key configs for each referenced child schema
                let mut child_field_map: HashMap<String, Vec<String>> = HashMap::new();
                let mut child_key_config_map: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
                for (_, child_schema_name) in &ref_fields {
                    if child_field_map.contains_key(child_schema_name) {
                        continue;
                    }
                    if let Ok(Some(child_schema)) = db.schema_manager.get_schema(child_schema_name) {
                        let fields = Self::get_queryable_fields(&child_schema);
                        if !fields.is_empty() {
                            child_field_map.insert(child_schema_name.clone(), fields);
                        }
                        // Store key config so we can extract KeyValue from field values
                        if let Some(key_cfg) = &child_schema.key {
                            child_key_config_map.insert(
                                child_schema_name.clone(),
                                (key_cfg.hash_field.clone(), key_cfg.range_field.clone()),
                            );
                        }
                    }
                }

                (ref_fields, child_field_map, child_key_config_map)
            }; // db guard dropped here

            // --- Batch rehydration: collect → batch query → distribute ---

            // 1. Collect: Walk all results and ref fields, recording each reference's position.
            struct RefLocation {
                result_idx: usize,
                field_name: String,
                ref_idx: usize,
                key_value: KeyValue,
            }

            let mut ref_locations: Vec<RefLocation> = Vec::new();
            // Track unique keys needed per child schema
            let mut keys_by_schema: HashMap<String, HashSet<KeyValue>> = HashMap::new();

            for (result_idx, result) in results.iter().enumerate() {
                let fields_obj = match result.get("fields").and_then(|v| v.as_object()) {
                    Some(obj) => obj,
                    None => continue,
                };

                for (field_name, child_schema_name) in &ref_fields {
                    if !child_field_map.contains_key(child_schema_name) {
                        continue;
                    }

                    let refs_array = match fields_obj
                        .get(field_name)
                        .and_then(|v| v.as_array())
                    {
                        Some(arr) => arr,
                        None => continue,
                    };

                    for (ref_idx, ref_obj) in refs_array.iter().enumerate() {
                        if let Some(kv) = Self::parse_ref_key(ref_obj) {
                            keys_by_schema
                                .entry(child_schema_name.clone())
                                .or_default()
                                .insert(kv.clone());
                            ref_locations.push(RefLocation {
                                result_idx,
                                field_name: field_name.clone(),
                                ref_idx,
                                key_value: kv,
                            });
                        }
                    }
                }
            }

            // 2. Batch query: For each child schema, execute ONE unfiltered query.
            //    Then recursively rehydrate child results if depth > 1.
            //    Build a HashMap<KeyValue, Value> index for fast lookup.
            let mut hydrated_index: HashMap<String, HashMap<KeyValue, Value>> = HashMap::new();

            for child_schema_name in keys_by_schema.keys() {
                let child_fields = match child_field_map.get(child_schema_name) {
                    Some(f) => f,
                    None => continue,
                };

                let child_query = Query::new(
                    child_schema_name.clone(),
                    child_fields.clone(),
                );

                let mut child_results = match self
                    .execute_query_json_internal(child_query, HashSet::new())
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        log::warn!(
                            "Rehydration: failed to query child schema '{}': {}",
                            child_schema_name, e
                        );
                        continue;
                    }
                };

                // Recursively rehydrate child results if depth > 1
                if remaining_depth > 1 {
                    if let Err(e) = self
                        .rehydrate_references(
                            &mut child_results,
                            child_schema_name,
                            remaining_depth - 1,
                            visited.clone(),
                        )
                        .await
                    {
                        log::warn!(
                            "Rehydration: recursive rehydration failed for child schema '{}': {}",
                            child_schema_name, e
                        );
                    }
                }

                // Build index: map KeyValue → hydrated record.
                // Extract key from field values using the child schema's key config,
                // because the JSON "key" object may have nulls even when the record
                // has the actual values in its fields.
                let key_config = child_key_config_map.get(child_schema_name);
                let mut index: HashMap<KeyValue, Value> = HashMap::new();
                for record in child_results {
                    let fields_obj = record.get("fields");
                    let hash = key_config
                        .and_then(|(h, _)| h.as_ref())
                        .and_then(|hash_field| {
                            fields_obj
                                .and_then(|f| f.get(hash_field))
                                .and_then(Self::value_to_key_string)
                        });
                    let range = key_config
                        .and_then(|(_, r)| r.as_ref())
                        .and_then(|range_field| {
                            fields_obj
                                .and_then(|f| f.get(range_field))
                                .and_then(Self::value_to_key_string)
                        });
                    let kv = KeyValue::new(hash, range);
                    index.insert(kv, record);
                }

                hydrated_index.insert(child_schema_name.clone(), index);
            }

            // 3. Distribute: Walk results again, replacing raw references with hydrated records.
            //    Build a map of (result_idx, field_name) → Vec<(ref_idx, hydrated_value)>
            //    so we can batch-replace per field.
            let mut replacements: HashMap<(usize, String), Vec<(usize, Value)>> = HashMap::new();

            for loc in &ref_locations {
                let child_schema_name = ref_fields
                    .iter()
                    .find(|(f, _)| f == &loc.field_name)
                    .map(|(_, s)| s);

                if let Some(child_schema_name) = child_schema_name {
                    if let Some(index) = hydrated_index.get(child_schema_name) {
                        if let Some(hydrated) = index.get(&loc.key_value) {
                            replacements
                                .entry((loc.result_idx, loc.field_name.clone()))
                                .or_default()
                                .push((loc.ref_idx, hydrated.clone()));
                        }
                    }
                }
            }

            // Apply replacements
            for ((result_idx, field_name), ref_replacements) in &replacements {
                if let Some(Value::Object(fields_obj)) = results[*result_idx].get_mut("fields") {
                    if let Some(Value::Array(arr)) = fields_obj.get_mut(field_name) {
                        for (ref_idx, hydrated_value) in ref_replacements {
                            if *ref_idx < arr.len() {
                                arr[*ref_idx] = hydrated_value.clone();
                            }
                        }
                    }
                }
            }

            Ok(())
        })
    }

    /// Build a HashRangeFilter from a KeyValue.
    #[cfg(test)]
    fn filter_from_key_value(kv: &KeyValue) -> Option<HashRangeFilter> {
        match (&kv.hash, &kv.range) {
            (Some(h), Some(r)) => Some(HashRangeFilter::HashRangeKey {
                hash: h.clone(),
                range: r.clone(),
            }),
            (Some(h), None) => Some(HashRangeFilter::HashKey(h.clone())),
            _ => None,
        }
    }

    /// Get the list of queryable field names from a schema.
    fn get_queryable_fields(schema: &crate::schema::types::schema::Schema) -> Vec<String> {
        schema.fields.clone().unwrap_or_default()
    }

    /// Convert a JSON value to a string suitable for use as a key component.
    /// Handles both string and numeric values.
    fn value_to_key_string(v: &Value) -> Option<String> {
        v.as_str()
            .map(|s| s.to_string())
            .or_else(|| v.as_f64().map(|n| n.to_string()))
    }

    /// Parse a reference JSON object into a KeyValue.
    /// Expected format: `{"schema": "...", "key": {"hash": "...", "range": "..."}}`
    fn parse_ref_key(ref_obj: &Value) -> Option<KeyValue> {
        let key_obj = ref_obj.get("key")?;
        let hash = key_obj.get("hash").and_then(Self::value_to_key_string);
        let range = key_obj.get("range").and_then(Self::value_to_key_string);
        if hash.is_none() && range.is_none() {
            return None;
        }
        Some(KeyValue::new(hash, range))
    }

    // --- Logging Operations ---

    /// List logs with optional filtering.
    pub async fn list_logs(
        &self,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Vec<crate::logging::core::LogEntry> {
        crate::logging::LoggingSystem::query_logs(limit, since)
            .await
            .unwrap_or_default()
    }

    /// Get current logging configuration.
    pub async fn get_log_config(&self) -> Option<crate::logging::config::LogConfig> {
        crate::logging::LoggingSystem::get_config().await
    }

    /// Reload logging configuration from file.
    pub async fn reload_log_config(&self, path: &str) -> FoldDbResult<()> {
        crate::logging::LoggingSystem::reload_config_from_file(path)
            .await
            .map_err(|e| FoldDbError::Config(format!("Failed to reload log config: {}", e)))
    }

    /// Get available log features and their levels.
    pub async fn get_log_features(&self) -> Option<HashMap<String, String>> {
        crate::logging::LoggingSystem::get_features().await
    }

    /// Update log level for a specific feature.
    pub async fn update_log_feature_level(&self, feature: &str, level: &str) -> FoldDbResult<()> {
        crate::logging::LoggingSystem::update_feature_level(feature, level)
            .await
            .map_err(|e| FoldDbError::Config(format!("Failed to update log level: {}", e)))
    }

    /// Executes a mutation operation from a Mutation struct.
    pub async fn execute_mutation_op(&self, mutation: Mutation) -> FoldDbResult<String> {
        let schema_name = mutation.schema_name.clone();
        log::info!("🔄 Starting mutation execution for schema: {}", schema_name);

        let mut db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        let mut ids = db
            .mutation_manager
            .write_mutations_batch_async(vec![mutation])
            .await
            .map_err(|e| {
                log::error!("❌ Mutation execution failed: {}", e);
                FoldDbError::Config(format!("Mutation execution failed: {}", e))
            })?;

        log::info!("📊 Mutation returned {} IDs", ids.len());
        match ids.pop() {
            Some(id) => {
                log::info!("✅ Mutation succeeded with ID: {}", id);
                Ok(id)
            }
            None => {
                log::error!("❌ Batch mutation returned no IDs");
                Err(FoldDbError::Config(
                    "Batch mutation returned no IDs".to_string(),
                ))
            }
        }
    }

    /// Executes a mutation operation (legacy wrapper).
    pub async fn execute_mutation(
        &self,
        schema: String,
        fields_and_values: HashMap<String, Value>,
        key_value: KeyValue,
        mutation_type: MutationType,
    ) -> FoldDbResult<String> {
        // Delete mutations are allowed to have empty fields_and_values
        if fields_and_values.is_empty() && mutation_type != MutationType::Delete {
            return Err(FoldDbError::Config("No fields to mutate".to_string()));
        }

        let mutation = Mutation::new(
            schema,
            fields_and_values,
            key_value,
            String::new(),
            0,
            mutation_type,
        );

        self.execute_mutation_op(mutation).await
    }

    /// Executes multiple mutations in a batch from Mutation structs.
    pub async fn execute_mutations_batch_ops(
        &self,
        mutations: Vec<Mutation>,
    ) -> FoldDbResult<Vec<String>> {
        let mut db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        let mutation_ids = db
            .mutation_manager
            .write_mutations_batch_async(mutations)
            .await
            .map_err(|e| FoldDbError::Config(format!("Mutation execution failed: {}", e)))?;

        Ok(mutation_ids)
    }

    /// Executes multiple mutations in a batch for improved performance (from JSON).
    pub async fn execute_mutations_batch(
        &self,
        mutations_data: Vec<Value>,
    ) -> FoldDbResult<Vec<String>> {
        if mutations_data.is_empty() {
            return Ok(Vec::new());
        }

        let mut mutations = Vec::new();

        // Parse each mutation from the input data
        for mutation_data in mutations_data {
            let (schema, fields_and_values, key_value, mutation_type, source_file_name) =
                match serde_json::from_value::<Operation>(mutation_data) {
                    Ok(Operation::Mutation {
                        schema,
                        fields_and_values,
                        key_value,
                        mutation_type,
                        source_file_name,
                    }) => (
                        schema,
                        fields_and_values,
                        key_value,
                        mutation_type,
                        source_file_name,
                    ),
                    Err(e) => {
                        return Err(FoldDbError::Config(format!(
                            "Failed to parse mutation: {}",
                            e
                        )));
                    }
                };

            // Delete mutations are allowed to have empty fields_and_values
            if fields_and_values.is_empty() && mutation_type != MutationType::Delete {
                return Err(FoldDbError::Config("No fields to mutate".to_string()));
            }

            let mut mutation = Mutation::new(
                schema,
                fields_and_values,
                key_value,
                String::new(),
                0,
                mutation_type,
            );

            // Add source_file_name if provided
            if let Some(filename) = source_file_name {
                mutation = mutation.with_source_file_name(filename);
            }

            mutations.push(mutation);
        }

        self.execute_mutations_batch_ops(mutations).await
    }

    // Removed execute_sync as part of eliminating the generic execute path

    /// Search the native word index for a term.
    pub async fn native_index_search(&self, term: &str) -> FoldDbResult<Vec<IndexResult>> {
        let term = term.trim();
        if term.is_empty() {
            return Err(FoldDbError::Config("Term cannot be empty".to_string()));
        }

        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        db.native_search_all_classifications(term)
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))
    }

    /// List all schemas with their states.
    pub async fn list_schemas(&self) -> FoldDbResult<Vec<SchemaWithState>> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        db.schema_manager
            .get_schemas_with_states()
            .map_err(|e| FoldDbError::Database(e.to_string()))
    }

    /// Get a specific schema by name with its state.
    pub async fn get_schema(&self, name: &str) -> FoldDbResult<Option<SchemaWithState>> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        let mgr = &db.schema_manager;
        match mgr
            .get_schema(name)
            .map_err(|e| FoldDbError::Database(e.to_string()))?
        {
            Some(schema) => {
                let states = mgr
                    .get_schema_states()
                    .map_err(|e| FoldDbError::Database(e.to_string()))?;
                let state = states.get(name).copied().unwrap_or_default();
                Ok(Some(SchemaWithState::new(schema, state)))
            }
            None => Ok(None),
        }
    }

    /// Approve a schema.
    pub async fn approve_schema(&self, schema_name: &str) -> FoldDbResult<Option<String>> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        let schema_mgr = &db.schema_manager;
        let transform_mgr = &db.transform_manager;

        // Check if schema is already approved
        let states = schema_mgr
            .get_schema_states()
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        let current_state = states.get(schema_name).copied().unwrap_or_default();

        if current_state == SchemaState::Approved {
            return Ok(None);
        }

        let is_transform = transform_mgr.transform_exists(schema_name).unwrap_or(false);

        // Logic to generate backfill hash needs to be moved here or reused.
        // For now, I will create a private helper or copy the logic since it was in the route handler.
        // But `generate_backfill_hash_for_transform` was in `src/server/routes/schema.rs`.
        // Integrating it here.
        let backfill_hash = if is_transform {
            Self::generate_backfill_hash_for_transform(transform_mgr, schema_name).await
        } else {
            None
        };

        schema_mgr
            .set_schema_state_with_backfill(
                schema_name,
                SchemaState::Approved,
                backfill_hash.clone(),
            )
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        Ok(backfill_hash)
    }

    /// Block a schema.
    pub async fn block_schema(&self, schema_name: &str) -> FoldDbResult<()> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        db.schema_manager
            .block_schema(schema_name)
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))
    }

    /// Load schemas from the schema service (Standard fetch & load logic).
    /// Returns (available_count, loaded_count, failed_schemas).
    pub async fn load_schemas(&self) -> FoldDbResult<(usize, usize, Vec<String>)> {
        // Need to drop lock between fetch and load to avoid holding it too long?
        // The original implementation dropped it.
        let schemas = {
            self.node
                .fetch_available_schemas()
                .await
                .map_err(|e| FoldDbError::Database(e.to_string()))?
        };

        let schema_count = schemas.len();
        let mut loaded_count = 0;
        let mut failed_schemas = Vec::new();

        for schema in schemas {
            let schema_name = schema.name.clone();
            let result = {
                let db = self
                    .node
                    .get_fold_db()
                    .await
                    .map_err(|e| FoldDbError::Database(e.to_string()))?;

                db.schema_manager
                    .load_schema_internal(schema)
                    .await
                    .map_err(|e| FoldDbError::Database(e.to_string()))
            };

            match result {
                Ok(_) => loaded_count += 1,
                Err(e) => {
                    log::error!("Failed to load schema {}: {}", schema_name, e);
                    failed_schemas.push(schema_name);
                }
            }
        }

        Ok((schema_count, loaded_count, failed_schemas))
    }

    /// List transforms.
    pub async fn list_transforms(&self) -> FoldDbResult<HashMap<String, Transform>> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        db.transform_manager
            .list_transforms()
            .map_err(|e| FoldDbError::Database(e.to_string()))
    }

    /// Add transform to queue.
    pub async fn add_to_transform_queue(
        &self,
        transform_id: &str,
        trigger: &str,
    ) -> FoldDbResult<()> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        if let Some(orchestrator) = db.transform_orchestrator() {
            orchestrator
                .add_transform(transform_id, trigger)
                .await
                .map_err(|e| FoldDbError::Config(e.to_string()))
        } else {
            Err(FoldDbError::Config(
                "Transform orchestrator not available".to_string(),
            ))
        }
    }

    /// Get transform queue info.
    /// Returns (length, queued_transforms).
    pub async fn get_transform_queue(&self) -> FoldDbResult<(usize, Vec<String>)> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        if let Some(orchestrator) = db.transform_orchestrator() {
            let queued = orchestrator
                .list_queued_transforms()
                .map_err(|e| FoldDbError::Config(e.to_string()))?;
            let len = orchestrator.len().unwrap_or(0);
            Ok((len, queued))
        } else {
            Err(FoldDbError::Config(
                "Transform orchestrator not available".to_string(),
            ))
        }
    }

    /// Get all backfills.
    pub async fn get_all_backfills(&self) -> FoldDbResult<Vec<BackfillInfo>> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        Ok(db.get_all_backfills())
    }

    /// Get active backfills.
    pub async fn get_active_backfills(&self) -> FoldDbResult<Vec<BackfillInfo>> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        Ok(db.get_active_backfills())
    }

    /// Get backfill by ID/Hash.
    pub async fn get_backfill(&self, id: &str) -> FoldDbResult<Option<BackfillInfo>> {
        // Access via get_fold_db is standardized
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        Ok(db.get_backfill(id))
    }

    /// Get backfill statistics.
    pub async fn get_backfill_statistics(&self) -> FoldDbResult<BackfillStatistics> {
        let backfills = self.get_all_backfills().await?;

        let active_count = backfills
            .iter()
            .filter(|b| b.status == BackfillStatus::InProgress)
            .count();
        let completed_count = backfills
            .iter()
            .filter(|b| b.status == BackfillStatus::Completed)
            .count();
        let failed_count = backfills
            .iter()
            .filter(|b| b.status == BackfillStatus::Failed)
            .count();

        Ok(BackfillStatistics {
            total_backfills: backfills.len(),
            active_backfills: active_count,
            completed_backfills: completed_count,
            failed_backfills: failed_count,
            total_mutations_expected: backfills.iter().map(|b| b.mutations_expected).sum(),
            total_mutations_completed: backfills.iter().map(|b| b.mutations_completed).sum(),
            total_mutations_failed: backfills.iter().map(|b| b.mutations_failed).sum(),
            total_records_produced: backfills.iter().map(|b| b.records_produced).sum(),
        })
    }

    /// Get event/transform statistics.
    pub async fn get_transform_statistics(
        &self,
    ) -> FoldDbResult<crate::fold_db_core::infrastructure::event_statistics::EventStatistics> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        Ok(db.get_event_statistics())
    }

    /// Get indexing status.
    pub async fn get_indexing_status(&self) -> FoldDbResult<IndexingStatus> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;
        Ok(db.get_indexing_status().await)
    }

    /// Get the node's private key
    pub fn get_node_private_key(&self) -> String {
        self.node.get_node_private_key().to_string()
    }

    /// Get the node's public key
    pub fn get_node_public_key(&self) -> String {
        self.node.get_node_public_key().to_string()
    }

    /// Get the system public key
    pub fn get_system_public_key(&self) -> FoldDbResult<Option<crate::security::PublicKeyInfo>> {
        let security_manager = self.node.get_security_manager();
        security_manager
            .get_system_public_key()
            .map_err(|e| FoldDbError::Other(e.to_string()))
    }

    /// Reset schema service
    pub async fn reset_schema_service(&self) -> FoldDbResult<()> {
        let schema_client = self.node.get_schema_client();
        schema_client
            .reset_schema_service()
            .await
            .map_err(|e| FoldDbError::Other(format!("Schema service reset failed: {}", e)))
    }

    /// Get database configuration
    pub fn get_database_config(&self) -> DatabaseConfig {
        self.node.config.database.clone()
    }

    // Helper for approve_schema
    async fn generate_backfill_hash_for_transform(
        transform_manager: &crate::transform::manager::TransformManager,
        schema_name: &str,
    ) -> Option<String> {
        let transforms = match transform_manager.list_transforms() {
            Ok(t) => t,
            Err(e) => {
                log::warn!("Failed to list transforms for {}: {}", schema_name, e);
                return None;
            }
        };

        let transform = match transforms.get(schema_name) {
            Some(t) => t,
            None => {
                log::debug!("Transform {} not found in transform list", schema_name);
                return None;
            }
        };

        // Look up the transform's schema from the database
        let declarative_schema = match transform_manager
            .db_ops
            .get_schema(transform.get_schema_name())
            .await
        {
            Ok(Some(s)) => s,
            Ok(None) => {
                log::warn!("Transform {} schema not found in database", schema_name);
                return None;
            }
            Err(e) => {
                log::warn!("Failed to get schema for transform {}: {}", schema_name, e);
                return None;
            }
        };

        let inputs = declarative_schema.get_inputs();
        let first_input = match inputs.first() {
            Some(i) => i,
            None => {
                log::warn!(
                    "Transform {} has no inputs in declarative schema",
                    schema_name
                );
                return None;
            }
        };

        let source_schema_name = match first_input.split('.').next() {
            Some(s) => s,
            None => {
                log::warn!("Failed to parse source schema from input: {}", first_input);
                return None;
            }
        };

        Some(
            crate::fold_db_core::infrastructure::backfill_tracker::BackfillTracker::generate_hash(
                schema_name,
                source_schema_name,
            ),
        )
    }

    /// Reset the database (destructive operation).
    /// Handles closing DB and clearing storage (Local or DynamoDB).
    /// Note: Schema service reset is NOT included - use reset_schema_service() separately if needed.
    pub async fn perform_database_reset(
        &self,
        #[allow(unused_variables)] user_id_override: Option<&str>,
    ) -> FoldDbResult<()> {
        // 1. Get config and path before closing
        let config = self.node.config.clone();
        let db_path = config.get_storage_path();

        // 3. Close the current database
        if let Ok(db) = self.node.get_fold_db().await {
            if let Err(e) = db.close() {
                log::warn!("Failed to close database during reset: {}", e);
            }
        }

        // 4. Handle storage reset
        match &config.database {
            #[cfg(feature = "aws-backend")]
            DatabaseConfig::Cloud(cloud_config) => {
                let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                    .region(aws_sdk_dynamodb::config::Region::new(
                        cloud_config.region.clone(),
                    ))
                    .load()
                    .await;
                let client = std::sync::Arc::new(aws_sdk_dynamodb::Client::new(&aws_config));

                // Priority: 1) explicit override, 2) current user context from HTTP request,
                // 3) config user_id, 4) node public key
                let uid = user_id_override
                    .map(|s| s.to_string())
                    .or_else(crate::logging::core::get_current_user_id)
                    .or_else(|| cloud_config.user_id.clone())
                    .unwrap_or_else(|| self.node.get_node_public_key().to_string());

                log::info!(
                    "Resetting database for user_id={} using scan-free DynamoDbResetManager",
                    uid
                );

                let manager = crate::storage::reset_manager::DynamoDbResetManager::new(
                    client.clone(),
                    cloud_config.tables.clone(),
                );

                if let Err(e) = manager.reset_user(&uid).await {
                    log::error!("Failed to reset user data: {}", e);
                    return Err(FoldDbError::Other(format!(
                        "Failed to reset user data: {}",
                        e
                    )));
                }
            }
            DatabaseConfig::Local { .. } => {
                if db_path.exists() {
                    if let Err(e) = std::fs::remove_dir_all(&db_path) {
                        log::error!("Failed to delete database folder: {}", e);
                        return Err(FoldDbError::Io(e));
                    }
                }
                // Recreate the empty data directory so subsequent operations can use it
                if let Err(e) = std::fs::create_dir_all(&db_path) {
                    log::error!("Failed to recreate database folder: {}", e);
                    return Err(FoldDbError::Io(e));
                }
            }
        }

        Ok(())
    }

    /// Scan a folder using LLM to classify files and return recommendations.
    pub async fn smart_folder_scan(
        &self,
        folder_path: &std::path::Path,
        max_depth: usize,
        max_files: usize,
    ) -> FoldDbResult<crate::ingestion::smart_folder::SmartFolderScanResponse> {
        crate::ingestion::smart_folder::perform_smart_folder_scan(
            folder_path,
            max_depth,
            max_files,
            None,
        )
        .await
        .map_err(|e| FoldDbError::Other(e.to_string()))
    }

    /// Ingest a single file through the AI ingestion pipeline.
    ///
    /// Tries the native parser first for known formats (json, js/Twitter, csv, txt, md),
    /// then falls back to file_to_json for everything else (images, PDFs, YAML, etc.).
    pub async fn ingest_single_file(
        &self,
        file_path: &std::path::Path,
        auto_execute: bool,
    ) -> FoldDbResult<crate::ingestion::IngestionResponse> {
        use crate::ingestion::IngestionRequest;
        use crate::ingestion::json_processor::convert_file_to_json;
        use crate::ingestion::progress::ProgressService;
        use crate::ingestion::smart_folder;

        // Try native parser first (handles json, js/Twitter, csv, txt, md without LLM),
        // fall back to file_to_json for unsupported types (images, PDFs, etc.)
        let data = match smart_folder::read_file_as_json(file_path) {
            Ok(json) => json,
            Err(_) => convert_file_to_json(&file_path.to_path_buf())
                .await
                .map_err(|e| FoldDbError::Other(e.to_string()))?,
        };

        let progress_id = uuid::Uuid::new_v4().to_string();
        let pub_key = self.get_node_public_key();

        let request = IngestionRequest {
            data,
            auto_execute,
            trust_distance: 0,
            pub_key,
            source_file_name: file_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string()),
            progress_id: Some(progress_id.clone()),
        };

        let service = IngestionService::from_env().map_err(|e| FoldDbError::Other(e.to_string()))?;

        let progress_tracker = crate::ingestion::create_progress_tracker(None).await;
        let progress_service = ProgressService::new(progress_tracker);
        progress_service
            .start_progress(progress_id.clone(), "cli".to_string())
            .await;

        let response = service
            .process_json_with_node_and_progress(
                request,
                &self.node,
                &progress_service,
                progress_id,
            )
            .await
            .map_err(|e| FoldDbError::Other(e.to_string()))?;

        Ok(response)
    }

    /// Run an LLM agent query against the database.
    ///
    /// Creates an LlmQueryService, loads all schemas, and runs the agent
    /// which can autonomously use tools (query, list_schemas, search) to answer.
    pub async fn llm_query(
        &self,
        user_query: &str,
        user_hash: &str,
        max_iterations: usize,
    ) -> FoldDbResult<(
        String,
        Vec<crate::fold_node::llm_query::types::ToolCallRecord>,
    )> {
        use crate::fold_node::llm_query::service::LlmQueryService;
        use crate::ingestion::config::IngestionConfig;

        let config = IngestionConfig::from_env_allow_empty();
        let service = LlmQueryService::new(config).map_err(FoldDbError::Other)?;

        let schemas = self.list_schemas().await?;

        service
            .run_agent_query(user_query, &schemas, &self.node, user_hash, max_iterations)
            .await
            .map_err(FoldDbError::Other)
    }

    /// Update database configuration and write to disk.
    /// Returns the new NodeConfig so the caller can recreate the node.
    pub async fn update_database_configuration(
        &self,
        new_db_config: DatabaseConfig,
    ) -> FoldDbResult<NodeConfig> {
        let mut config = self.node.config.clone();
        config.database = new_db_config;

        let config_path =
            std::env::var("NODE_CONFIG").unwrap_or_else(|_| "config/node_config.json".to_string());

        // Ensure config directory exists
        if let Some(parent) = std::path::Path::new(&config_path).parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(FoldDbError::Other(format!(
                    "Failed to create config directory: {}",
                    e
                )));
            }
        }

        // Serialize and write config
        let config_json = serde_json::to_string_pretty(&config)
            .map_err(|e| FoldDbError::Config(format!("Failed to serialize config: {}", e)))?;

        let mut file = fs::File::create(&config_path)
            .map_err(|e| FoldDbError::Other(format!("Failed to create config file: {}", e)))?;

        file.write_all(config_json.as_bytes())
            .map_err(|e| FoldDbError::Other(format!("Failed to write config file: {}", e)))?;

        // Close current DB (best effort)
        if let Ok(db) = self.node.get_fold_db().await {
            if let Err(e) = db.close() {
                log::warn!("Failed to close database during config update: {}", e);
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold_node::NodeConfig;
    use crate::schema::types::declarative_schemas::DeclarativeSchemaDefinition;
    use crate::schema::types::key_config::KeyConfig;
    use crate::schema::types::operations::MutationType;
    use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
    use crate::schema::types::topology::{JsonTopology, TopologyNode, PrimitiveValueType};
    use crate::schema::SchemaState;
    use crate::security::Ed25519KeyPair;
    use serde_json::json;
    use tempfile::tempdir;

    /// Helper: create a FoldNode + OperationProcessor backed by a temp directory.
    async fn setup_processor() -> (OperationProcessor, FoldNode) {
        let temp_dir = tempdir().unwrap();
        let keypair = Ed25519KeyPair::generate().unwrap();
        let config = NodeConfig::new(temp_dir.path().to_path_buf())
            .with_schema_service_url("test://mock")
            .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());
        let node = FoldNode::new(config).await.unwrap();
        let processor = OperationProcessor::new(node.clone());
        (processor, node)
    }

    /// Helper: create a schema, load it, and approve it so mutations work.
    async fn load_and_approve_schema(node: &FoldNode, mut schema: DeclarativeSchemaDefinition) {
        schema.populate_runtime_fields().unwrap();
        let db = node.get_fold_db().await.unwrap();
        db.schema_manager.load_schema_internal(schema).await.unwrap();
    }

    async fn approve_schema(node: &FoldNode, name: &str) {
        let db = node.get_fold_db().await.unwrap();
        db.schema_manager.set_schema_state(name, SchemaState::Approved).await.unwrap();
    }

    #[tokio::test]
    async fn test_operation_processor_creation() {
        // This test would require a mock FoldNode
        // For now, just test that the struct can be created
        // In a real test, you'd create a test FoldNode instance
    }

    #[tokio::test]
    async fn test_logging_methods_signature() {
        // This test ensures the logging methods are available on OperationProcessor
        // without needing to instantiate a full FoldNode (which is complex).
        // It relies on the fact that if this compiles, the methods exist.
        async fn check_methods(processor: &crate::fold_node::OperationProcessor) {
            let _ = processor.list_logs(None, None).await;
            let _ = processor.get_log_config().await;
            let _ = processor.get_log_features().await;
        }
        let _ = check_methods;
    }

    fn string_topology() -> JsonTopology {
        JsonTopology::new(TopologyNode::Primitive {
            value: PrimitiveValueType::String,
            classifications: None,
        })
    }

    /// Helper: create a child HashRange schema with hash+range keys and one data field.
    /// Uses `field` as hash key and `_rk` as range key, both included in `fields`.
    fn make_child_schema(name: &str, field: &str) -> DeclarativeSchemaDefinition {
        let mut schema = DeclarativeSchemaDefinition::new(
            name.to_string(),
            SchemaType::HashRange,
            Some(KeyConfig {
                hash_field: Some(field.to_string()),
                range_field: Some("_rk".to_string()),
            }),
            Some(vec![field.to_string(), "_rk".to_string()]),
            None,
            None,
        );
        schema.field_topologies.insert(field.to_string(), string_topology());
        schema.field_topologies.insert("_rk".to_string(), string_topology());
        schema
    }

    /// Helper: create a parent HashRange schema with a name field and a Reference field.
    fn make_parent_schema(name: &str, ref_field: &str, child_schema_name: &str) -> DeclarativeSchemaDefinition {
        let mut schema = DeclarativeSchemaDefinition::new(
            name.to_string(),
            SchemaType::HashRange,
            Some(KeyConfig {
                hash_field: Some("name".to_string()),
                range_field: Some("_rk".to_string()),
            }),
            Some(vec!["name".to_string(), "_rk".to_string(), ref_field.to_string()]),
            None,
            None,
        );
        schema.field_topologies.insert("name".to_string(), string_topology());
        schema.field_topologies.insert("_rk".to_string(), string_topology());
        schema.field_topologies.insert(
            ref_field.to_string(),
            JsonTopology::new(TopologyNode::Reference {
                schema_name: child_schema_name.to_string(),
            }),
        );
        schema
    }

    #[tokio::test]
    async fn test_query_without_rehydrate_depth_returns_raw_references() {
        let (processor, node) = setup_processor().await;
        let pub_key = processor.get_node_public_key();

        let child_schema = make_child_schema("PostSchema", "title");
        let parent_schema = make_parent_schema("UserSchema", "posts", "PostSchema");

        load_and_approve_schema(&node, child_schema).await;
        approve_schema(&node, "PostSchema").await;
        load_and_approve_schema(&node, parent_schema).await;
        approve_schema(&node, "UserSchema").await;

        // Create a child record with hash+range key
        let mut child_fields = HashMap::new();
        child_fields.insert("title".to_string(), json!("Hello World"));
        child_fields.insert("_rk".to_string(), json!("r1"));
        processor.execute_mutation_op(Mutation::new(
            "PostSchema".to_string(), child_fields,
            KeyValue::new(Some("Hello World".to_string()), Some("r1".to_string())),
            pub_key.clone(), 0, MutationType::Create,
        )).await.unwrap();

        // Create a parent record with reference to the child
        let mut parent_fields = HashMap::new();
        parent_fields.insert("name".to_string(), json!("Alice"));
        parent_fields.insert("_rk".to_string(), json!("r1"));
        parent_fields.insert("posts".to_string(), json!([
            {"schema": "PostSchema", "key": {"hash": "Hello World", "range": "r1"}}
        ]));
        processor.execute_mutation_op(Mutation::new(
            "UserSchema".to_string(), parent_fields,
            KeyValue::new(Some("Alice".to_string()), Some("r1".to_string())),
            pub_key.clone(), 0, MutationType::Create,
        )).await.unwrap();

        // Query WITHOUT rehydration - should return raw reference objects
        let query = Query::new(
            "UserSchema".to_string(),
            vec!["name".to_string(), "posts".to_string()],
        );
        let results = processor.execute_query_json(query).await.unwrap();

        assert_eq!(results.len(), 1);
        let record = &results[0];
        assert_eq!(record["fields"]["name"], json!("Alice"));

        // posts field should contain raw reference objects (no rehydration)
        let posts = record["fields"]["posts"].as_array().unwrap();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0]["schema"], json!("PostSchema"));
    }

    #[tokio::test]
    async fn test_query_with_rehydrate_depth_resolves_references() {
        let (processor, node) = setup_processor().await;
        let pub_key = processor.get_node_public_key();

        let child_schema = make_child_schema("PostSchema", "title");
        let parent_schema = make_parent_schema("UserSchema", "posts", "PostSchema");

        load_and_approve_schema(&node, child_schema).await;
        approve_schema(&node, "PostSchema").await;
        load_and_approve_schema(&node, parent_schema).await;
        approve_schema(&node, "UserSchema").await;

        // Create child record with hash+range key
        let mut child_fields = HashMap::new();
        child_fields.insert("title".to_string(), json!("Hello World"));
        child_fields.insert("_rk".to_string(), json!("r1"));
        processor.execute_mutation_op(Mutation::new(
            "PostSchema".to_string(), child_fields,
            KeyValue::new(Some("Hello World".to_string()), Some("r1".to_string())),
            pub_key.clone(), 0, MutationType::Create,
        )).await.unwrap();

        // Create parent record with reference to child
        let mut parent_fields = HashMap::new();
        parent_fields.insert("name".to_string(), json!("Alice"));
        parent_fields.insert("_rk".to_string(), json!("r1"));
        parent_fields.insert("posts".to_string(), json!([
            {"schema": "PostSchema", "key": {"hash": "Hello World", "range": "r1"}}
        ]));
        processor.execute_mutation_op(Mutation::new(
            "UserSchema".to_string(), parent_fields,
            KeyValue::new(Some("Alice".to_string()), Some("r1".to_string())),
            pub_key.clone(), 0, MutationType::Create,
        )).await.unwrap();

        // Query WITH rehydration depth 1 - should resolve references
        let mut query = Query::new(
            "UserSchema".to_string(),
            vec!["name".to_string(), "posts".to_string()],
        );
        query.rehydrate_depth = Some(1);
        let results = processor.execute_query_json(query).await.unwrap();

        assert_eq!(results.len(), 1);
        let record = &results[0];
        assert_eq!(record["fields"]["name"], json!("Alice"));

        // posts field should now contain hydrated child records
        let posts = record["fields"]["posts"].as_array().unwrap();
        assert_eq!(posts.len(), 1);

        // Hydrated record should have "fields" with the child's data
        let hydrated_post = &posts[0];
        assert!(hydrated_post.get("fields").is_some(), "Hydrated post should have 'fields': {}", hydrated_post);
        assert_eq!(hydrated_post["fields"]["title"], json!("Hello World"));
        // Should also have a "key"
        assert!(hydrated_post.get("key").is_some(), "Hydrated post should have 'key'");
    }

    #[tokio::test]
    async fn test_rehydrate_depth_zero_does_not_resolve() {
        let (processor, node) = setup_processor().await;
        let pub_key = processor.get_node_public_key();

        let child_schema = make_child_schema("ItemSchema", "label");
        let parent_schema = make_parent_schema("ContainerSchema", "items", "ItemSchema");

        load_and_approve_schema(&node, child_schema).await;
        approve_schema(&node, "ItemSchema").await;
        load_and_approve_schema(&node, parent_schema).await;
        approve_schema(&node, "ContainerSchema").await;

        // Create child
        let mut child_fields = HashMap::new();
        child_fields.insert("label".to_string(), json!("Widget"));
        child_fields.insert("_rk".to_string(), json!("r1"));
        processor.execute_mutation_op(Mutation::new(
            "ItemSchema".to_string(), child_fields,
            KeyValue::new(Some("Widget".to_string()), Some("r1".to_string())),
            pub_key.clone(), 0, MutationType::Create,
        )).await.unwrap();

        // Create parent with reference
        let mut parent_fields = HashMap::new();
        parent_fields.insert("name".to_string(), json!("c1"));
        parent_fields.insert("_rk".to_string(), json!("r1"));
        parent_fields.insert("items".to_string(), json!([
            {"schema": "ItemSchema", "key": {"hash": "Widget", "range": "r1"}}
        ]));
        processor.execute_mutation_op(Mutation::new(
            "ContainerSchema".to_string(), parent_fields,
            KeyValue::new(Some("c1".to_string()), Some("r1".to_string())),
            pub_key.clone(), 0, MutationType::Create,
        )).await.unwrap();

        // Query with depth 0 - should NOT resolve references
        let mut query = Query::new(
            "ContainerSchema".to_string(),
            vec!["name".to_string(), "items".to_string()],
        );
        query.rehydrate_depth = Some(0);
        let results = processor.execute_query_json(query).await.unwrap();

        assert_eq!(results.len(), 1);
        let items = results[0]["fields"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        // Should still be raw reference - has "schema" key, not "fields" key
        assert!(items[0].get("schema").is_some(), "depth=0 should leave raw references");
    }

    #[test]
    fn test_parse_ref_key_with_hash_only() {
        let ref_obj = json!({"schema": "SomeSchema", "key": {"hash": "abc"}});
        let kv = OperationProcessor::parse_ref_key(&ref_obj).unwrap();
        assert_eq!(kv.hash, Some("abc".to_string()));
        assert_eq!(kv.range, None);
    }

    #[test]
    fn test_parse_ref_key_with_hash_and_range() {
        let ref_obj = json!({"schema": "S", "key": {"hash": "h1", "range": "r1"}});
        let kv = OperationProcessor::parse_ref_key(&ref_obj).unwrap();
        assert_eq!(kv.hash, Some("h1".to_string()));
        assert_eq!(kv.range, Some("r1".to_string()));
    }

    #[test]
    fn test_parse_ref_key_missing_key_returns_none() {
        let ref_obj = json!({"schema": "S"});
        assert!(OperationProcessor::parse_ref_key(&ref_obj).is_none());
    }

    #[test]
    fn test_filter_from_key_value_hash_only() {
        let kv = KeyValue::new(Some("abc".to_string()), None);
        let filter = OperationProcessor::filter_from_key_value(&kv);
        assert!(matches!(filter, Some(HashRangeFilter::HashKey(ref h)) if h == "abc"));
    }

    #[test]
    fn test_filter_from_key_value_hash_and_range() {
        let kv = KeyValue::new(Some("h".to_string()), Some("r".to_string()));
        let filter = OperationProcessor::filter_from_key_value(&kv);
        assert!(matches!(filter, Some(HashRangeFilter::HashRangeKey { ref hash, ref range }) if hash == "h" && range == "r"));
    }

    #[test]
    fn test_filter_from_key_value_no_keys_returns_none() {
        let kv = KeyValue::new(None, None);
        assert!(OperationProcessor::filter_from_key_value(&kv).is_none());
    }
}
