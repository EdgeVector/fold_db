use crate::db_operations::IndexResult;
use crate::fold_db_core::FoldDB;
use crate::schema::field::HashRangeFilter;
use crate::schema::types::Query;
use std::collections::HashMap;

/// Maximum number of results to hydrate (for performance)
const MAX_HYDRATE_RESULTS: usize = 50;

/// Hydrate index results by fetching actual field values from the database
///
/// This function takes index search results (which only contain references) and
/// fetches the actual field values from the database, populating the `value` field.
///
/// # Arguments
/// * `results` - Vector of IndexResult from native index search
/// * `fold_db` - Reference to FoldDb for querying records
///
/// # Returns
/// * Vector of IndexResult with populated `value` fields
pub async fn hydrate_index_results(
    mut results: Vec<IndexResult>,
    fold_db: &FoldDB,
) -> Vec<IndexResult> {
    if results.is_empty() {
        return results;
    }

    // Deduplicate stale entries from append-only index
    results = IndexResult::keep_highest_molecule_version(results);

    // Limit the number of results to hydrate for performance
    let hydrate_count = results.len().min(MAX_HYDRATE_RESULTS);

    log::debug!(
        "Hydrating {} of {} index results",
        hydrate_count,
        results.len()
    );

    // Group results by schema_name to batch queries
    let mut schema_groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, result) in results.iter().enumerate().take(hydrate_count) {
        schema_groups
            .entry(result.schema_name.clone())
            .or_default()
            .push(idx);
    }

    // For each schema, fetch all needed records in one query
    for (schema_name, indices) in schema_groups {
        // Collect unique keys for this schema
        let mut keys_to_fetch: Vec<(String, String)> = Vec::new();
        let mut key_to_indices: HashMap<String, Vec<usize>> = HashMap::new();

        for idx in &indices {
            let result = &results[*idx];
            let hash = result.key_value.hash.clone().unwrap_or_default();
            let range = result.key_value.range.clone().unwrap_or_default();

            // Create a key identifier for deduplication
            let key_id = format!("{}:{}", hash, range);

            if !key_to_indices.contains_key(&key_id) {
                keys_to_fetch.push((hash, range));
            }
            key_to_indices.entry(key_id).or_default().push(*idx);
        }

        if keys_to_fetch.is_empty() {
            continue;
        }

        // Build a query to fetch all records for this schema
        // Use HashRangeKeys filter if we have multiple keys
        let filter = if keys_to_fetch.len() == 1 {
            let (hash, range) = &keys_to_fetch[0];
            if !hash.is_empty() && !range.is_empty() {
                Some(HashRangeFilter::HashRangeKey {
                    hash: hash.clone(),
                    range: range.clone(),
                })
            } else if !hash.is_empty() {
                Some(HashRangeFilter::HashKey(hash.clone()))
            } else if !range.is_empty() {
                Some(HashRangeFilter::RangePrefix(range.clone()))
            } else {
                None
            }
        } else {
            // Use batch filter for multiple keys
            Some(HashRangeFilter::HashRangeKeys(keys_to_fetch.clone()))
        };

        // Get all field names we need to fetch
        let fields_needed: Vec<String> = indices
            .iter()
            .map(|idx| results[*idx].field.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let query = Query {
            schema_name: schema_name.clone(),
            fields: fields_needed,
            filter,
            as_of: None,
            rehydrate_depth: Some(1),
        };

        // Execute the query
        match fold_db.query_executor.query(query).await {
            Ok(field_results) => {
                // field_results is HashMap<field_name, HashMap<KeyValue, FieldValue>>
                // We need to map back to our results

                for (idx, result) in results.iter_mut().enumerate().take(hydrate_count) {
                    if result.schema_name != schema_name {
                        continue;
                    }

                    // Find the value for this result's field and key
                    if let Some(field_data) = field_results.get(&result.field) {
                        if let Some(field_value) = field_data.get(&result.key_value) {
                            // Extract the actual value from FieldValue
                            result.value = field_value.value.clone();
                            log::trace!(
                                "Hydrated result {}: schema={}, field={}, key={:?}",
                                idx,
                                result.schema_name,
                                result.field,
                                result.key_value
                            );
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to hydrate results for schema {}: {}",
                    schema_name,
                    e
                );
            }
        }
    }

    log::debug!("Hydration complete");
    results
}

/// Generate a backfill hash for a transform schema
pub async fn generate_backfill_hash_for_transform(
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
