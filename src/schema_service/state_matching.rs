use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::db_operations::native_index::cosine_similarity;
use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::Schema;

use super::state::SchemaServiceState;

/// Minimum cosine similarity between descriptive names to consider them a semantic match.
pub(super) const DESCRIPTIVE_NAME_SIMILARITY_THRESHOLD: f32 = 0.8;

/// Minimum cosine similarity between context-enriched field names to consider them synonyms.
/// Threshold for canonicalize_fields: compares description-only embeddings against
/// the canonical field registry. Set to 0.88 based on empirical testing with
/// all-MiniLM-L6-v2: true synonyms score ≥0.88, false positives score ≤0.85.
pub(super) const FIELD_SIMILARITY_THRESHOLD: f32 = 0.88;

/// Threshold for semantic_field_rename_map: compares hybrid embeddings
/// ("the {name} of the {context}: {description}") during schema expansion.
/// Lower than FIELD_SIMILARITY_THRESHOLD because the hybrid format includes
/// field names and context that reduce absolute similarity, but the bidirectional
/// best-match check provides strong false-positive protection.
/// Empirical: start_date↔start_time=0.86, venue↔location=0.86, tags↔content=0.83 (reject).
pub(super) const SEMANTIC_RENAME_THRESHOLD: f32 = 0.84;

/// Collect all field names from a schema (union of fields and transform_fields keys)
pub(super) fn collect_field_names(schema: &Schema) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Some(ref fields) = schema.fields {
        for f in fields {
            names.insert(f.clone());
        }
    }
    if let Some(ref tf) = schema.transform_fields {
        for key in tf.keys() {
            names.insert(key.clone());
        }
    }
    names
}

/// Compute Jaccard index: |A ∩ B| / |A ∪ B|
pub fn jaccard_index(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    intersection as f64 / union as f64
}

impl SchemaServiceState {
    /// Find an existing descriptive_name that matches the given name.
    /// First tries exact match, then falls back to semantic (embedding) similarity.
    /// Returns (matched_descriptive_name, schema_identity_hash, is_exact_match).
    pub(super) fn find_matching_descriptive_name(
        &self,
        desc_name: &str,
    ) -> FoldDbResult<(Option<String>, Option<String>, bool)> {
        // 1. Exact match
        let index = self.descriptive_name_index.read().map_err(|_| {
            FoldDbError::Config("Failed to acquire descriptive_name_index read lock".to_string())
        })?;
        if let Some(hash) = index.get(desc_name) {
            return Ok((Some(desc_name.to_string()), Some(hash.clone()), true));
        }
        drop(index);

        // 2. Semantic similarity via embeddings
        let query_embedding = match self.embedder.embed_text(desc_name) {
            Ok(vec) => vec,
            Err(e) => {
                log_feature!(
                    LogFeature::Schema,
                    warn,
                    "Failed to embed descriptive_name '{}' for similarity search: {}",
                    desc_name,
                    e
                );
                return Ok((None, None, false));
            }
        };

        let embeddings = self.descriptive_name_embeddings.read().map_err(|_| {
            FoldDbError::Config(
                "Failed to acquire descriptive_name_embeddings read lock".to_string(),
            )
        })?;

        let mut best_match: Option<(&str, f32)> = None;
        for (existing_desc, existing_vec) in embeddings.iter() {
            let sim = cosine_similarity(&query_embedding, existing_vec);
            if sim >= DESCRIPTIVE_NAME_SIMILARITY_THRESHOLD
                && best_match.is_none_or(|(_, best_sim)| sim > best_sim)
            {
                best_match = Some((existing_desc.as_str(), sim));
            }
        }

        if let Some((matched_desc, similarity)) = best_match {
            log_feature!(
                LogFeature::Schema,
                info,
                "Semantic descriptive_name match: '{}' ≈ '{}' (similarity: {:.3})",
                desc_name,
                matched_desc,
                similarity
            );
            let index = self.descriptive_name_index.read().map_err(|_| {
                FoldDbError::Config(
                    "Failed to acquire descriptive_name_index read lock".to_string(),
                )
            })?;
            let hash = index.get(matched_desc).cloned();
            return Ok((Some(matched_desc.to_string()), hash, false));
        }

        Ok((None, None, false))
    }

    /// Check whether two schema names are semantically similar enough to be
    /// considered the same collection. Uses embedding similarity on the
    /// human-readable form of the names (underscores → spaces).
    ///
    /// This acts as a second gate for descriptive_name matching: even if
    /// "Holiday Illustration" ≈ "Famous Paintings" in embedding space, the
    /// schema names `artwork_collection` vs `famous_paintings` should NOT merge.
    pub(super) fn schema_names_are_similar(&self, incoming: &str, existing: &str) -> bool {
        // Exact match (case-insensitive)
        if incoming.eq_ignore_ascii_case(existing) {
            return true;
        }

        // Convert snake_case to readable form for embedding comparison
        let readable_incoming = incoming.replace('_', " ");
        let readable_existing = existing.replace('_', " ");

        let incoming_emb = match self.embedder.embed_text(&readable_incoming) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let existing_emb = match self.embedder.embed_text(&readable_existing) {
            Ok(v) => v,
            Err(_) => return false,
        };

        let sim = cosine_similarity(&incoming_emb, &existing_emb);
        log_feature!(
            LogFeature::Schema,
            info,
            "Schema name similarity: '{}' vs '{}' = {:.3}",
            incoming,
            existing,
            sim
        );
        // Use a high threshold — schema names are short and precise, so only
        // near-synonyms should match (e.g., "blog_posts" ≈ "blog_articles").
        sim >= 0.85
    }

    /// Get or compute the embedding for a field, combining name-in-context with description.
    ///
    /// Embeds "the {field_name} of the {descriptive_name}: {description}" when a description
    /// is available. The name-in-context prefix provides structural signal (fields with the
    /// same role in the same domain cluster together), while the description adds semantic
    /// specificity that prevents false positives (e.g. "subject" won't match "calendar"
    /// because their descriptions are unrelated).
    pub(super) fn get_field_embedding(
        &self,
        field_name: &str,
        descriptive_name: &str,
        field_description: Option<&str>,
    ) -> Option<Vec<f32>> {
        let context_text = match field_description {
            Some(desc) => format!("the {} of the {}: {}", field_name, descriptive_name, desc),
            None => format!("the {} of the {}", field_name, descriptive_name),
        };
        // Cache key includes a hash of the description to correctly distinguish
        // entries with different descriptions for the same field name.
        let desc_hash = match field_description {
            Some(desc) => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                desc.hash(&mut hasher);
                hasher.finish()
            }
            None => 0,
        };
        let cache_key = format!("{}:{}:{}", descriptive_name, field_name, desc_hash);

        // Check cache first
        if let Ok(cache) = self.field_embeddings.read() {
            if let Some(vec) = cache.get(&cache_key) {
                return Some(vec.clone());
            }
        }

        match self.embedder.embed_text(&context_text) {
            Ok(vec) => {
                if let Ok(mut cache) = self.field_embeddings.write() {
                    cache.insert(cache_key, vec.clone());
                }
                Some(vec)
            }
            Err(e) => {
                log_feature!(
                    LogFeature::Schema,
                    warn,
                    "Failed to embed field '{}' with context '{}': {}",
                    field_name,
                    descriptive_name,
                    e
                );
                None
            }
        }
    }
}
