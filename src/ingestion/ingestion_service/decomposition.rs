//! Recursive decomposition ingestion for nested data structures.
//!
//! Handles ingestion of data with nested arrays-of-objects by recursively
//! decomposing, resolving schemas, and building parent-child references.

use crate::ingestion::decomposer;
use crate::ingestion::key_extraction::extract_key_values_from_data;
use crate::ingestion::mutation_generator;
use crate::ingestion::{IngestionError, IngestionResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::topology::{JsonTopology, TopologyNode};
use crate::schema::types::KeyValue;
use serde_json::Value;
use std::collections::HashMap;

use super::IngestionService;

/// Maximum recursion depth for decomposition to prevent unbounded nesting.
const MAX_DECOMPOSITION_DEPTH: usize = 10;

/// Cached result of AI schema determination for a given structure.
pub(super) struct CachedSchema {
    pub(super) schema_name: String,
    pub(super) mutation_mappers: HashMap<String, String>,
}

impl IngestionService {
    /// Resolve the schema for a given structure hash.
    ///
    /// If not cached, decomposes the representative item (recursively resolving
    /// its own children), then sends the flat representative to AI to determine
    /// the schema. Caches the result for reuse by all items sharing the same
    /// structure.
    pub(super) async fn resolve_schema_for_structure(
        &self,
        structure_hash: &str,
        representative: &Value,
        schema_cache: &mut HashMap<String, CachedSchema>,
        node: &crate::fold_node::FoldNode,
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

            match schema_manager.get_schema_metadata(&schema_name) {
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
    pub(super) async fn ingest_decomposed_item(
        &self,
        item: &Value,
        structure_hash: &str,
        schema_cache: &mut HashMap<String, CachedSchema>,
        node: &crate::fold_node::FoldNode,
        trust_distance: u32,
        pub_key: &str,
        source_file_name: Option<String>,
        metadata: Option<HashMap<String, String>>,
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
                    metadata.clone(),
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
                metadata,
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
