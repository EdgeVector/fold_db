use std::collections::{HashMap, HashSet};

use crate::db_operations::native_index::cosine_similarity;
use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::Schema;

use super::state::SchemaServiceState;
use super::state_matching::SEMANTIC_RENAME_THRESHOLD;
use super::types::SchemaAddOutcome;

impl SchemaServiceState {
    /// If a schema has been superseded by an expanded version, resolve to the
    /// active schema. Returns `None` if no redirection is needed.
    pub(super) fn resolve_active_schema(
        &self,
        existing_schema: &Schema,
        schema_name: &str,
        schemas: &HashMap<String, Schema>,
    ) -> Option<(Schema, String)> {
        let desc_name = existing_schema.descriptive_name.as_ref()?;
        let index = match self.descriptive_name_index.read() {
            Ok(idx) => idx,
            Err(e) => {
                log_feature!(
                    LogFeature::Schema,
                    warn,
                    "Failed to acquire descriptive_name_index read lock: {} — falling back to original schema",
                    e
                );
                return None;
            }
        };
        let current_hash = index.get(desc_name)?;
        if *current_hash == schema_name {
            return None;
        }
        let active_schema = schemas.get(current_hash)?;
        log_feature!(
            LogFeature::Schema,
            info,
            "Schema '{}' was superseded by '{}' — checking active schema",
            schema_name,
            current_hash
        );
        Some((active_schema.clone(), current_hash.clone()))
    }

    /// Find semantic field name matches between incoming and existing schemas.
    ///
    /// For fields in the incoming schema that don't have a literal match in the
    /// existing schema, uses context-enriched embeddings to detect synonyms
    /// (e.g., "creator" ≈ "artist" in an artwork context).
    ///
    /// Returns a map: incoming_field_name → existing_field_name (canonical).
    pub(super) fn semantic_field_rename_map(
        &self,
        incoming_fields: &[String],
        existing_fields: &[String],
        descriptive_name: &str,
        incoming_descriptions: &HashMap<String, String>,
        existing_descriptions: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let existing_set: HashSet<&String> = existing_fields.iter().collect();
        let mut rename_map: HashMap<String, String> = HashMap::new();
        // Track which existing fields have been claimed to avoid many-to-one mapping
        let mut claimed: HashSet<String> = HashSet::new();

        for incoming_field in incoming_fields {
            // Skip fields that already have a literal match
            if existing_set.contains(incoming_field) {
                continue;
            }

            let incoming_emb = match self.get_field_embedding(
                incoming_field,
                descriptive_name,
                incoming_descriptions
                    .get(incoming_field.as_str())
                    .map(|s| s.as_str()),
            ) {
                Some(v) => v,
                None => continue,
            };

            let mut best: Option<(&str, f32)> = None;
            for existing_field in existing_fields {
                if claimed.contains(existing_field) {
                    continue;
                }
                let existing_emb = match self.get_field_embedding(
                    existing_field,
                    descriptive_name,
                    existing_descriptions
                        .get(existing_field.as_str())
                        .map(|s| s.as_str()),
                ) {
                    Some(v) => v,
                    None => continue,
                };
                let sim = cosine_similarity(&incoming_emb, &existing_emb);
                if sim >= SEMANTIC_RENAME_THRESHOLD && best.as_ref().is_none_or(|(_, s)| sim > *s) {
                    best = Some((existing_field.as_str(), sim));
                }
            }

            if let Some((matched_field, similarity)) = best {
                // Bidirectional check: verify the existing field's best match among
                // all incoming fields is also this incoming field. This prevents false
                // positives like "medium"→"artist" when "creator"→"artist" is stronger.
                let existing_emb = self
                    .get_field_embedding(
                        matched_field,
                        descriptive_name,
                        existing_descriptions.get(matched_field).map(|s| s.as_str()),
                    )
                    .unwrap();
                let mut reverse_best: Option<(&str, f32)> = None;
                for candidate in incoming_fields {
                    if existing_set.contains(candidate) {
                        continue; // skip literal matches
                    }
                    let candidate_emb = match self.get_field_embedding(
                        candidate,
                        descriptive_name,
                        incoming_descriptions
                            .get(candidate.as_str())
                            .map(|s| s.as_str()),
                    ) {
                        Some(v) => v,
                        None => continue,
                    };
                    let sim = cosine_similarity(&existing_emb, &candidate_emb);
                    if reverse_best.as_ref().is_none_or(|(_, s)| sim > *s) {
                        reverse_best = Some((candidate.as_str(), sim));
                    }
                }

                let is_mutual =
                    reverse_best.is_some_and(|(best_incoming, _)| best_incoming == incoming_field);

                if is_mutual {
                    log_feature!(
                        LogFeature::Schema,
                        info,
                        "Semantic field rename: '{}' → '{}' (similarity: {:.3}, context: '{}')",
                        incoming_field,
                        matched_field,
                        similarity,
                        descriptive_name
                    );
                    rename_map.insert(incoming_field.clone(), matched_field.to_string());
                    claimed.insert(matched_field.to_string());
                } else {
                    log_feature!(
                        LogFeature::Schema,
                        info,
                        "Rejected non-mutual match: '{}' → '{}' (similarity: {:.3}, but existing field's best match is '{}')",
                        incoming_field,
                        matched_field,
                        similarity,
                        reverse_best.map(|(f, _)| f).unwrap_or("none"),
                    );
                }
            } else {
                log_feature!(
                    LogFeature::Schema,
                    info,
                    "No semantic field match for '{}' in context '{}' — treating as new field",
                    incoming_field,
                    descriptive_name
                );
            }
        }

        rename_map
    }

    /// Apply field renames to a schema: rename fields, update classifications,
    /// mutation_mappers, and ref_fields to use canonical names.
    pub(super) fn apply_field_renames(
        schema: &mut Schema,
        rename_map: &HashMap<String, String>,
        mutation_mappers: &mut HashMap<String, String>,
    ) {
        if rename_map.is_empty() {
            return;
        }

        // Rename in fields list
        if let Some(ref mut fields) = schema.fields {
            for field in fields.iter_mut() {
                if let Some(canonical) = rename_map.get(field) {
                    *field = canonical.clone();
                }
            }
        }

        // Rename in field_classifications and field_data_classifications
        for (old_name, canonical) in rename_map {
            if let Some(classifications) = schema.field_classifications.remove(old_name) {
                schema
                    .field_classifications
                    .entry(canonical.clone())
                    .or_insert(classifications);
            }
            if let Some(data_classification) = schema.field_data_classifications.remove(old_name) {
                schema
                    .field_data_classifications
                    .entry(canonical.clone())
                    .or_insert(data_classification);
            }
            if let Some(interest_category) = schema.field_interest_categories.remove(old_name) {
                schema
                    .field_interest_categories
                    .entry(canonical.clone())
                    .or_insert(interest_category);
            }
            // Add mutation_mapper: old_name → canonical so AI mutations still work
            mutation_mappers
                .entry(old_name.clone())
                .or_insert_with(|| canonical.clone());
        }
    }

    /// Expand an incoming schema to be a superset of an existing schema.
    ///
    /// Merges fields, sets field_mappers for shared fields (pointing to the old
    /// schema's molecules), merges classifications and ref_fields, recomputes
    /// identity_hash, persists, and updates caches.
    ///
    /// Returns `SchemaAddOutcome::Expanded` on success, or `AlreadyExists` if
    /// the incoming fields are a subset of the existing.
    pub(super) async fn expand_schema(
        &self,
        schema: &mut Schema,
        existing: &Schema,
        old_name: &str,
        desc_name: &str,
        mutation_mappers: &HashMap<String, String>,
    ) -> FoldDbResult<SchemaAddOutcome> {
        let existing_fields = existing.fields.clone().unwrap_or_default();
        let existing_set: HashSet<String> = existing_fields.iter().cloned().collect();
        let new_field_set: HashSet<String> = schema
            .fields
            .as_ref()
            .map(|nf| nf.iter().cloned().collect())
            .unwrap_or_default();

        // If the new schema's fields are a subset of the existing, reuse existing
        if new_field_set.is_subset(&existing_set) {
            log_feature!(
                LogFeature::Schema,
                info,
                "New schema is a subset of existing '{}' (descriptive_name='{}') — reusing existing",
                old_name,
                desc_name
            );
            return Ok(SchemaAddOutcome::AlreadyExists(
                existing.clone(),
                mutation_mappers.clone(),
            ));
        }

        log_feature!(
            LogFeature::Schema,
            info,
            "Expanding schema (descriptive_name='{}') — merging fields from old hash '{}'",
            desc_name,
            old_name
        );

        // Merge to superset: existing fields + new-only fields
        let new_fields_to_add: Vec<String> =
            new_field_set.difference(&existing_set).cloned().collect();
        let mut merged_fields = existing_fields.clone();
        merged_fields.extend(new_fields_to_add);
        schema.fields = Some(merged_fields);

        // Set field_mappers for shared fields (pointing to old schema's molecules)
        use crate::schema::types::declarative_schemas::FieldMapper;
        let mut mappers: HashMap<String, FieldMapper> =
            schema.field_mappers().cloned().unwrap_or_default();
        for field in &existing_fields {
            mappers
                .entry(field.clone())
                .or_insert_with(|| FieldMapper::new(old_name.to_string(), field.clone()));
        }
        schema.field_mappers = Some(mappers);
        schema.field_molecule_uuids = None;

        // Merge field_classifications (keep existing, add new)
        for (field, classifications) in &existing.field_classifications {
            schema
                .field_classifications
                .entry(field.clone())
                .or_insert_with(|| classifications.clone());
        }

        // Merge field_data_classifications (keep existing, add new)
        for (field, classification) in &existing.field_data_classifications {
            schema
                .field_data_classifications
                .entry(field.clone())
                .or_insert_with(|| classification.clone());
        }

        // Merge field_interest_categories (keep existing, add new)
        for (field, category) in &existing.field_interest_categories {
            schema
                .field_interest_categories
                .entry(field.clone())
                .or_insert_with(|| category.clone());
        }

        // Merge ref_fields (keep existing references)
        for (field, target) in &existing.ref_fields {
            schema
                .ref_fields
                .entry(field.clone())
                .or_insert_with(|| target.clone());
        }

        // Recompute identity hash with merged fields.
        // The expanded schema is a NEW schema — its name is the identity hash
        // (derived from schema name + fields). The old schema keeps its name and
        // gets blocked/superseded. Field mappers point back to the old schema.
        schema.compute_identity_hash();
        let new_hash = schema
            .get_identity_hash()
            .ok_or_else(|| {
                FoldDbError::Config("Failed to compute merged identity_hash".to_string())
            })?
            .clone();
        schema.name = new_hash.clone();
        let expanded_name = schema.name.clone();

        // Persist expanded schema
        self.persist_schema(schema, mutation_mappers).await?;

        // Mark the old schema as superseded in memory and grab a clone for persistence
        let old_clone = {
            let mut schemas = self.schemas.write().map_err(|_| {
                FoldDbError::Config("Failed to acquire schemas write lock".to_string())
            })?;
            let clone = if let Some(old_schema) = schemas.get_mut(old_name) {
                old_schema.superseded_by = Some(expanded_name.clone());
                Some(old_schema.clone())
            } else {
                None
            };
            schemas.insert(expanded_name.clone(), schema.clone());
            clone
        };

        // Persist the superseded marker (outside the lock)
        if let Some(old_schema) = old_clone {
            self.persist_schema(&old_schema, &HashMap::new()).await?;
        }

        // Update descriptive_name index to point to expanded schema
        {
            let mut index = self.descriptive_name_index.write().map_err(|_| {
                FoldDbError::Config(
                    "Failed to acquire descriptive_name_index write lock".to_string(),
                )
            })?;
            index.insert(desc_name.to_string(), expanded_name);
        }

        // Register new fields as canonical for future schema proposals.
        // Fails if classification cannot be determined (no ANTHROPIC_API_KEY for new fields).
        self.register_canonical_fields(schema).await?;

        // Propagate canonical field types, classifications, and interest categories to the expanded schema
        self.apply_canonical_types(schema);
        self.apply_canonical_classifications(schema);
        self.apply_canonical_interest_categories(schema);

        log_feature!(
            LogFeature::Schema,
            info,
            "Schema expanded: old='{}' (blocked) -> new='{}' (descriptive_name='{}')",
            old_name,
            schema.name,
            desc_name
        );

        Ok(SchemaAddOutcome::Expanded(
            old_name.to_string(),
            schema.clone(),
            mutation_mappers.clone(),
        ))
    }
}
