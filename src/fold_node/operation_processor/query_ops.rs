use crate::db_operations::IndexResult;
use crate::error::{FoldDbError, FoldDbResult};
use crate::fold_node::response_types::QueryResultMap;
use crate::schema::types::field::Field;
use crate::schema::types::topology::TopologyNode;
use crate::schema::types::{KeyValue, Query};
#[cfg(test)]
use crate::schema::types::field::HashRangeFilter;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::OperationProcessor;

impl OperationProcessor {
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

            // --- Batch rehydration: collect -> batch query -> distribute ---

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

                // Build index: map KeyValue -> hydrated record.
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
            //    Build a map of (result_idx, field_name) -> Vec<(ref_idx, hydrated_value)>
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
    pub(super) fn filter_from_key_value(kv: &KeyValue) -> Option<HashRangeFilter> {
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
    pub(super) fn parse_ref_key(ref_obj: &Value) -> Option<KeyValue> {
        let key_obj = ref_obj.get("key")?;
        let hash = key_obj.get("hash").and_then(Self::value_to_key_string);
        let range = key_obj.get("range").and_then(Self::value_to_key_string);
        if hash.is_none() && range.is_none() {
            return None;
        }
        Some(KeyValue::new(hash, range))
    }

    /// List keys for a schema with pagination.
    /// Returns (paginated_keys, total_count).
    pub async fn list_schema_keys(
        &self,
        schema_name: &str,
        offset: usize,
        limit: usize,
    ) -> FoldDbResult<(Vec<KeyValue>, usize)> {
        let db = self
            .node
            .get_fold_db()
            .await
            .map_err(|e| FoldDbError::Database(e.to_string()))?;

        let mut schema = db
            .schema_manager
            .get_schema(schema_name)
            .map_err(|e| FoldDbError::Database(e.to_string()))?
            .ok_or_else(|| {
                FoldDbError::Database(format!("Schema '{}' not found", schema_name))
            })?;

        // Pick the first runtime field to extract keys from
        let field = schema
            .runtime_fields
            .values_mut()
            .next()
            .ok_or_else(|| {
                FoldDbError::Database(format!("Schema '{}' has no fields", schema_name))
            })?;

        field.refresh_from_db(&db.db_ops).await;
        let all_keys = field.get_all_keys();
        let total = all_keys.len();
        let page = all_keys.into_iter().skip(offset).take(limit).collect();
        Ok((page, total))
    }

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
}
