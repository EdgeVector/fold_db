use std::collections::{HashMap, HashSet};

use crate::db_operations::native_index::cosine_similarity;
use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::data_classification::DataClassification;
use crate::schema::types::field_value_type::FieldValueType;
use crate::schema::types::Schema;

use super::state::SchemaServiceState;
use super::state_matching::FIELD_SIMILARITY_THRESHOLD;
use super::types::CanonicalField;

impl SchemaServiceState {
    /// Build embedding text from a field's description.
    /// Embeds the description only — the field name is excluded because different
    /// sources use different names for the same concept (e.g. "summary" vs "subject"),
    /// and including the name adds noise that pushes cosine similarity below threshold.
    /// The description captures the semantic meaning; field names are compared separately.
    pub(super) fn build_embedding_text(_field_name: &str, description: &str) -> String {
        description.to_string()
    }

    /// Build a description for a field from its schema context.
    /// Prefers AI-generated field_descriptions, falls back to field_classifications + descriptive_name.
    ///
    /// For AI-generated descriptions, returns the description as-is without appending
    /// the schema's descriptive_name. The "in {schema}" suffix is shared by ALL fields
    /// in a schema and inflates cross-field similarity, causing false positive matches
    /// (e.g. "subject" matching "calendar" because both end with "in Calendar Events").
    /// Only the fallback paths use the suffix since their descriptions are generic.
    pub(super) fn build_field_description(field_name: &str, schema: &Schema) -> String {
        // Prefer the AI-generated natural language description (already specific)
        if let Some(desc) = schema.field_descriptions.get(field_name) {
            return desc.clone();
        }

        // Fall back to classifications + descriptive_name for context
        let desc_name = schema.descriptive_name.as_deref().unwrap_or("unknown");
        let classifications = schema
            .field_classifications
            .get(field_name)
            .map(|c| c.join(", "))
            .unwrap_or_default();

        if classifications.is_empty() {
            format!("field in {}", desc_name)
        } else {
            format!("{} field in {}", classifications, desc_name)
        }
    }

    /// Infer the FieldValueType for a field from schema metadata.
    /// Uses ref_fields for schema references, field_types if declared,
    /// and falls back to Any.
    fn infer_field_type(field_name: &str, schema: &Schema) -> FieldValueType {
        // If the schema already has a declared type, use it
        if let Some(ft) = schema.field_types.get(field_name) {
            return ft.clone();
        }

        // If it's a ref_field, type is SchemaRef
        if let Some(ref_schema) = schema.ref_fields.get(field_name) {
            return FieldValueType::SchemaRef(ref_schema.clone());
        }

        // No type info available
        FieldValueType::Any
    }

    /// Register new fields from a schema as canonical.
    /// Only adds fields that don't already exist in the registry.
    ///
    /// The schema service is the authority on classification. For each new field:
    /// 1. Use caller-provided classification if present
    /// 2. LLM call to analyze field description (requires ANTHROPIC_API_KEY)
    /// 3. No fallback — returns error if classification cannot be determined
    pub(super) async fn register_canonical_fields(&self, schema: &Schema) -> FoldDbResult<()> {
        let field_names = schema.fields.as_deref().unwrap_or(&[]);

        // Phase 1: Identify new fields (read lock only)
        let new_fields: Vec<String> = {
            let fields = self.canonical_fields.read().map_err(|_| {
                FoldDbError::Config("Failed to acquire canonical_fields read lock".to_string())
            })?;
            field_names
                .iter()
                .filter(|f| !fields.contains_key(*f))
                .cloned()
                .collect()
        };

        if new_fields.is_empty() {
            return Ok(());
        }

        // Phase 2: Batch classify all new fields in a single LLM call (no locks held).
        // Previously this was 2 serial LLM calls per field (sensitivity + interest category).
        // Batch reduces N fields from 2N calls to 1 call.
        // Collect field metadata before the batch LLM call
        let field_meta: Vec<(String, String, FieldValueType)> = new_fields
            .iter()
            .map(|f| {
                let desc = Self::build_field_description(f, schema);
                let ft = Self::infer_field_type(f, schema);
                (f.clone(), desc, ft)
            })
            .collect();

        let batch_input: Vec<(&str, &str, Option<&DataClassification>)> = field_meta
            .iter()
            .map(|(name, desc, _ft)| {
                let caller = schema.field_data_classifications.get(name.as_str());
                (name.as_str(), desc.as_str(), caller)
            })
            .collect();

        let batch_results = super::classify::classify_fields_batch(&batch_input)
            .await
            .map_err(FoldDbError::Config)?;

        // Build canonical entries from batch results
        let batch_map: std::collections::HashMap<
            String,
            super::classify::BatchFieldClassification,
        > = batch_results.into_iter().collect();

        let mut entries: Vec<(String, CanonicalField, Option<Vec<f32>>)> = Vec::new();

        for (field_name, desc, field_type) in &field_meta {
            let batch = batch_map.get(field_name);
            let classification =
                batch
                    .map(|b| b.classification.clone())
                    .unwrap_or_else(|| DataClassification {
                        sensitivity_level: 1,
                        data_domain: "general".to_string(),
                    });
            let interest_category = batch.and_then(|b| b.interest_category.clone());

            let embed_text = Self::build_embedding_text(field_name, desc);
            let embedding = self.embedder.embed_text(&embed_text).ok();

            entries.push((
                field_name.clone(),
                CanonicalField {
                    description: desc.clone(),
                    field_type: field_type.clone(),
                    classification: Some(classification),
                    interest_category,
                },
                embedding,
            ));
        }

        // Phase 3: Store under write locks
        let mut fields = self.canonical_fields.write().map_err(|_| {
            FoldDbError::Config("Failed to acquire canonical_fields write lock".to_string())
        })?;
        let mut embeddings = self.canonical_field_embeddings.write().map_err(|_| {
            FoldDbError::Config(
                "Failed to acquire canonical_field_embeddings write lock".to_string(),
            )
        })?;

        for (field_name, canonical, embedding) in entries {
            // Re-check in case another thread registered it between phase 1 and 3
            if fields.contains_key(&field_name) {
                continue;
            }
            if let Some(vec) = embedding {
                embeddings.insert(field_name.clone(), vec);
            }
            self.persist_canonical_field(&field_name, &canonical);
            fields.insert(field_name, canonical);
        }

        Ok(())
    }

    /// Canonicalize incoming field names against the global canonical field registry.
    /// Returns a rename map: incoming_field -> canonical_field.
    /// Uses the same bidirectional best-match + threshold approach as semantic_field_rename_map.
    /// Embeds "field_name: description" for richer semantic matching.
    pub(super) fn canonicalize_fields(
        &self,
        incoming_fields: &[String],
        schema: &Schema,
        mutation_mappers: &mut HashMap<String, String>,
    ) -> HashMap<String, String> {
        let canonical = match self.canonical_fields.read() {
            Ok(c) => c,
            Err(_) => return HashMap::new(),
        };
        let embeddings = match self.canonical_field_embeddings.read() {
            Ok(e) => e,
            Err(_) => return HashMap::new(),
        };

        if canonical.is_empty() {
            return HashMap::new();
        }

        let mut rename_map: HashMap<String, String> = HashMap::new();
        let mut claimed: HashSet<String> = HashSet::new();

        for incoming_field in incoming_fields {
            // Don't rename if it already IS a canonical field
            if canonical.contains_key(incoming_field) {
                continue;
            }

            let incoming_desc = Self::build_field_description(incoming_field, schema);
            let incoming_embed_text = Self::build_embedding_text(incoming_field, &incoming_desc);
            let incoming_embedding = match self.embedder.embed_text(&incoming_embed_text) {
                Ok(vec) => vec,
                Err(_) => continue,
            };

            // Find best canonical match
            let mut best: Option<(&str, f32)> = None;
            for (canon_name, canon_vec) in embeddings.iter() {
                let sim = cosine_similarity(&incoming_embedding, canon_vec);
                if sim >= FIELD_SIMILARITY_THRESHOLD
                    && best.is_none_or(|(_, best_sim)| sim > best_sim)
                {
                    best = Some((canon_name.as_str(), sim));
                }
            }

            let Some((matched_canonical, _)) = best else {
                continue;
            };

            // Bidirectional check: is this incoming field the best match
            // for the canonical field too?
            let canon_vec = match embeddings.get(matched_canonical) {
                Some(v) => v,
                None => continue,
            };
            let mut reverse_best: Option<(&str, f32)> = None;
            for candidate in incoming_fields {
                let cand_desc = Self::build_field_description(candidate, schema);
                let cand_embed_text = Self::build_embedding_text(candidate, &cand_desc);
                if let Ok(cand_vec) = self.embedder.embed_text(&cand_embed_text) {
                    let sim = cosine_similarity(canon_vec, &cand_vec);
                    if reverse_best.is_none_or(|(_, best_sim)| sim > best_sim) {
                        reverse_best = Some((candidate.as_str(), sim));
                    }
                }
            }

            let is_mutual =
                reverse_best.is_some_and(|(best_incoming, _)| best_incoming == incoming_field);
            if is_mutual && !claimed.contains(matched_canonical) {
                log_feature!(
                    LogFeature::Schema,
                    info,
                    "Canonical field rename: '{}' -> '{}'",
                    incoming_field,
                    matched_canonical
                );
                rename_map.insert(incoming_field.clone(), matched_canonical.to_string());
                claimed.insert(matched_canonical.to_string());

                // Update mutation_mappers: incoming data key -> canonical field name
                if let Some(data_key) = mutation_mappers.remove(incoming_field) {
                    mutation_mappers.insert(data_key, matched_canonical.to_string());
                } else {
                    mutation_mappers.insert(incoming_field.clone(), matched_canonical.to_string());
                }
            }
        }

        rename_map
    }

    /// Load canonical fields from a sled tree.
    pub(super) fn load_canonical_fields_from_tree(&self, tree: &sled::Tree) -> FoldDbResult<()> {
        let mut fields = self.canonical_fields.write().map_err(|_| {
            FoldDbError::Config("Failed to acquire canonical_fields write lock".to_string())
        })?;
        let mut embeddings = self.canonical_field_embeddings.write().map_err(|_| {
            FoldDbError::Config(
                "Failed to acquire canonical_field_embeddings write lock".to_string(),
            )
        })?;
        fields.clear();
        embeddings.clear();

        for result in tree.iter() {
            let (key, value) = result.map_err(|e| {
                FoldDbError::Config(format!("Failed to iterate canonical_fields: {}", e))
            })?;
            let name = String::from_utf8(key.to_vec())
                .map_err(|e| FoldDbError::Config(format!("Invalid canonical field key: {}", e)))?;
            let value_bytes = value.to_vec();

            // Try to deserialize as CanonicalField (new format); fall back to plain description string (legacy)
            let canonical: CanonicalField =
                if let Ok(cf) = serde_json::from_slice::<CanonicalField>(&value_bytes) {
                    cf
                } else {
                    // Legacy format: value is just the description as UTF-8 string
                    let desc = String::from_utf8(value_bytes).map_err(|e| {
                        FoldDbError::Config(format!("Invalid canonical field description: {}", e))
                    })?;
                    CanonicalField {
                        description: desc,
                        field_type: FieldValueType::Any,
                        classification: None,
                        interest_category: None,
                    }
                };

            let embed_text = Self::build_embedding_text(&name, &canonical.description);
            if let Ok(vec) = self.embedder.embed_text(&embed_text) {
                embeddings.insert(name.clone(), vec);
            }
            fields.insert(name, canonical);
        }

        log_feature!(
            LogFeature::Schema,
            info,
            "Loaded {} canonical fields from sled",
            fields.len()
        );
        Ok(())
    }

    /// Backfill interest categories for existing canonical fields that don't have one.
    /// Called on startup after loading canonical fields from storage.
    /// Best-effort: failures are logged but don't block startup.
    pub(super) async fn backfill_interest_categories(&self) {
        let fields_to_backfill: Vec<(String, String)> = {
            let fields = match self.canonical_fields.read() {
                Ok(f) => f,
                Err(_) => return,
            };
            fields
                .iter()
                .filter(|(_, cf)| cf.interest_category.is_none())
                .map(|(name, cf)| (name.clone(), cf.description.clone()))
                .collect()
        };

        if fields_to_backfill.is_empty() {
            return;
        }

        log_feature!(
            LogFeature::Schema,
            info,
            "Backfilling interest categories for {} canonical fields",
            fields_to_backfill.len()
        );

        let mut backfilled = 0usize;
        for (field_name, description) in &fields_to_backfill {
            let category = super::classify::infer_interest_category(field_name, description).await;

            if let Some(ref cat) = category {
                let mut fields = match self.canonical_fields.write() {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                if let Some(canonical) = fields.get_mut(field_name) {
                    canonical.interest_category = Some(cat.clone());
                    self.persist_canonical_field(field_name, canonical);
                    backfilled += 1;
                }
            }
        }

        log_feature!(
            LogFeature::Schema,
            info,
            "Backfilled interest categories: {}/{} fields classified",
            backfilled,
            fields_to_backfill.len()
        );
    }

    /// Persist a canonical field to sled storage.
    pub(super) fn persist_canonical_field(&self, name: &str, canonical: &CanonicalField) {
        match &self.storage {
            super::state::SchemaStorage::Sled { db, .. } => {
                if let Ok(tree) = db.open_tree("canonical_fields") {
                    if let Ok(bytes) = serde_json::to_vec(canonical) {
                        let _ = tree.insert(name.as_bytes(), bytes);
                    }
                }
            }
        }
    }

    /// Populate a schema's `field_types` map from the canonical field registry.
    /// Called after canonicalization to propagate types from the registry to the schema.
    pub(super) fn apply_canonical_types(&self, schema: &mut Schema) {
        let fields = match self.canonical_fields.read() {
            Ok(f) => f,
            Err(_) => return,
        };

        for field_name in schema.fields.as_deref().unwrap_or(&[]) {
            // Skip if the schema already has a declared type for this field
            if schema.field_types.contains_key(field_name) {
                continue;
            }
            if let Some(canonical) = fields.get(field_name) {
                if canonical.field_type != FieldValueType::Any {
                    schema
                        .field_types
                        .insert(field_name.clone(), canonical.field_type.clone());
                }
            }
        }
    }

    /// Populate a schema's `field_data_classifications` map from the canonical field registry.
    /// Called after canonicalization to propagate classifications from the registry to the schema.
    /// Only fills in fields that don't already have a classification declared.
    pub(super) fn apply_canonical_classifications(&self, schema: &mut Schema) {
        let fields = match self.canonical_fields.read() {
            Ok(f) => f,
            Err(_) => return,
        };

        for field_name in schema.fields.as_deref().unwrap_or(&[]) {
            // Skip if the schema already has a classification for this field
            if schema.field_data_classifications.contains_key(field_name) {
                continue;
            }
            if let Some(canonical) = fields.get(field_name) {
                if let Some(ref classification) = canonical.classification {
                    schema
                        .field_data_classifications
                        .insert(field_name.clone(), classification.clone());
                }
            }
        }
    }

    /// Populate a schema's `field_interest_categories` map from the canonical field registry.
    /// Called after canonicalization to propagate interest categories from the registry to the schema.
    /// Only fills in fields that don't already have an interest category declared.
    pub(super) fn apply_canonical_interest_categories(&self, schema: &mut Schema) {
        let fields = match self.canonical_fields.read() {
            Ok(f) => f,
            Err(_) => return,
        };

        for field_name in schema.fields.as_deref().unwrap_or(&[]) {
            if schema.field_interest_categories.contains_key(field_name) {
                continue;
            }
            if let Some(canonical) = fields.get(field_name) {
                if let Some(ref category) = canonical.interest_category {
                    schema
                        .field_interest_categories
                        .insert(field_name.clone(), category.clone());
                }
            }
        }
    }
}
