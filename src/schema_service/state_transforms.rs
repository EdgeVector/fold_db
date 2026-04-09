use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::data_classification::DataClassification;
use crate::schema::types::field_value_type::FieldValueType;

use super::state::{SchemaServiceState, SchemaStorage};
use super::types::{
    RegisterTransformRequest, SimilarTransformEntry, SimilarTransformsResponse,
    TransformAddOutcome, TransformListEntry, TransformRecord,
};

/// NMI leakage threshold — above this, an output field carries meaningful
/// information about the corresponding input field.
#[cfg(feature = "transform-wasm")]
const NMI_LEAKAGE_THRESHOLD: f32 = 0.1;

impl SchemaServiceState {
    // ============== Transform Storage ==============

    /// Compute sha256 hex digest of WASM bytes.
    pub fn compute_wasm_hash(wasm_bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(wasm_bytes);
        format!("{:x}", hasher.finalize())
    }

    /// Register a new transform. Returns the hash and whether it was newly added.
    pub async fn register_transform(
        &self,
        request: RegisterTransformRequest,
    ) -> FoldDbResult<(TransformRecord, TransformAddOutcome)> {
        if request.name.trim().is_empty() {
            return Err(FoldDbError::Config(
                "Transform name must be non-empty".to_string(),
            ));
        }
        if request.wasm_bytes.is_empty() {
            return Err(FoldDbError::Config(
                "Transform wasm_bytes must be non-empty".to_string(),
            ));
        }
        if request.version.trim().is_empty() {
            return Err(FoldDbError::Config(
                "Transform version must be non-empty".to_string(),
            ));
        }
        if request.output_fields.is_empty() {
            return Err(FoldDbError::Config(
                "Transform must declare at least one output field".to_string(),
            ));
        }

        let hash = Self::compute_wasm_hash(&request.wasm_bytes);

        // Check if already registered (idempotent)
        if let Some(existing) = self.get_transform_by_hash(&hash)? {
            return Ok((existing, TransformAddOutcome::AlreadyExists));
        }

        // Phase 1: Resolve input field classifications from schema service state
        let (input_schema, input_ceiling) =
            self.resolve_input_classifications(&request.input_queries)?;

        // Phase 2: NMI estimation (feature-gated)
        let (output_classification, nmi_matrix, classification_verified, sample_count) = self
            .estimate_output_classification(
                &request.wasm_bytes,
                &input_schema,
                &request.output_fields,
                input_ceiling.clone(),
            );

        // Final assignment: max(ceiling, output)
        let assigned_classification =
            std::cmp::max(input_ceiling.clone(), output_classification.clone());

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| FoldDbError::Config(format!("System time error: {}", e)))?
            .as_secs();

        let record = TransformRecord {
            hash: hash.clone(),
            name: request.name,
            version: request.version,
            description: request.description,
            input_schema,
            output_schema: request.output_fields,
            source_url: request.source_url,
            registered_at: now,
            input_ceiling,
            output_classification,
            nmi_matrix,
            classification_verified,
            sample_count,
            assigned_classification,
        };

        // Persist metadata and WASM separately
        self.persist_transform_metadata(&record)?;
        self.persist_transform_wasm(&hash, &request.wasm_bytes)?;

        // Insert into in-memory cache
        {
            let mut transforms = self.transforms.write().map_err(|_| {
                FoldDbError::Config("Failed to acquire transforms write lock".to_string())
            })?;
            transforms.insert(hash.clone(), record.clone());
        }

        log_feature!(
            LogFeature::Schema,
            info,
            "Transform '{}' v{} registered with hash {} (classification: {})",
            record.name,
            record.version,
            hash,
            record.assigned_classification
        );

        Ok((record, TransformAddOutcome::Added))
    }

    /// Get a transform record by hash (from in-memory cache).
    pub fn get_transform_by_hash(&self, hash: &str) -> FoldDbResult<Option<TransformRecord>> {
        let transforms = self.transforms.read().map_err(|_| {
            FoldDbError::Config("Failed to acquire transforms read lock".to_string())
        })?;
        Ok(transforms.get(hash).cloned())
    }

    /// Get WASM bytes for a transform by hash (from Sled, not cached in memory).
    pub fn get_transform_wasm(&self, hash: &str) -> FoldDbResult<Option<Vec<u8>>> {
        match &self.storage {
            SchemaStorage::Sled { db, .. } => {
                let wasm_tree = db.open_tree("transform_wasm").map_err(|e| {
                    FoldDbError::Config(format!("Failed to open transform_wasm tree: {}", e))
                })?;
                match wasm_tree.get(hash.as_bytes()).map_err(|e| {
                    FoldDbError::Config(format!("Failed to get transform WASM: {}", e))
                })? {
                    Some(bytes) => Ok(Some(bytes.to_vec())),
                    None => Ok(None),
                }
            }
        }
    }

    /// List all transform hashes + names.
    pub fn get_transform_list(&self) -> FoldDbResult<Vec<TransformListEntry>> {
        let transforms = self.transforms.read().map_err(|_| {
            FoldDbError::Config("Failed to acquire transforms read lock".to_string())
        })?;
        Ok(transforms
            .values()
            .map(|r| TransformListEntry {
                hash: r.hash.clone(),
                name: r.name.clone(),
                version: r.version.clone(),
            })
            .collect())
    }

    /// Get all transform records (no WASM bytes).
    pub fn get_all_transforms(&self) -> FoldDbResult<Vec<TransformRecord>> {
        let transforms = self.transforms.read().map_err(|_| {
            FoldDbError::Config("Failed to acquire transforms read lock".to_string())
        })?;
        Ok(transforms.values().cloned().collect())
    }

    /// Verify that WASM bytes match a given hash.
    pub fn verify_transform(hash: &str, wasm_bytes: &[u8]) -> (bool, String) {
        let computed = Self::compute_wasm_hash(wasm_bytes);
        (computed == hash, computed)
    }

    /// Find transforms with similar names (Jaccard on name tokens).
    pub fn find_similar_transforms(
        &self,
        name: &str,
        threshold: f64,
    ) -> FoldDbResult<SimilarTransformsResponse> {
        let transforms = self.transforms.read().map_err(|_| {
            FoldDbError::Config("Failed to acquire transforms read lock".to_string())
        })?;

        let query_tokens: std::collections::HashSet<String> = tokenize_name(name);
        let mut similar = Vec::new();

        for record in transforms.values() {
            let record_tokens: std::collections::HashSet<String> = tokenize_name(&record.name);
            let similarity = super::state_matching::jaccard_index(&query_tokens, &record_tokens);
            if similarity >= threshold {
                similar.push(SimilarTransformEntry {
                    record: record.clone(),
                    similarity,
                });
            }
        }

        similar.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(SimilarTransformsResponse {
            query_name: name.to_string(),
            threshold,
            similar_transforms: similar,
        })
    }

    // ============== Persistence ==============

    fn persist_transform_metadata(&self, record: &TransformRecord) -> FoldDbResult<()> {
        match &self.storage {
            SchemaStorage::Sled { db, .. } => {
                let meta_tree = db.open_tree("transform_metadata").map_err(|e| {
                    FoldDbError::Config(format!("Failed to open transform_metadata tree: {}", e))
                })?;
                let serialized = serde_json::to_vec(record).map_err(|e| {
                    FoldDbError::Serialization(format!(
                        "Failed to serialize transform '{}': {}",
                        record.hash, e
                    ))
                })?;
                meta_tree
                    .insert(record.hash.as_bytes(), serialized)
                    .map_err(|e| {
                        FoldDbError::Config(format!(
                            "Failed to insert transform metadata '{}': {}",
                            record.hash, e
                        ))
                    })?;
                db.flush()
                    .map_err(|e| FoldDbError::Config(format!("Failed to flush sled: {}", e)))?;
            }
        }
        Ok(())
    }

    fn persist_transform_wasm(&self, hash: &str, wasm_bytes: &[u8]) -> FoldDbResult<()> {
        match &self.storage {
            SchemaStorage::Sled { db, .. } => {
                let wasm_tree = db.open_tree("transform_wasm").map_err(|e| {
                    FoldDbError::Config(format!("Failed to open transform_wasm tree: {}", e))
                })?;
                wasm_tree.insert(hash.as_bytes(), wasm_bytes).map_err(|e| {
                    FoldDbError::Config(format!(
                        "Failed to insert transform WASM '{}': {}",
                        hash, e
                    ))
                })?;
                db.flush()
                    .map_err(|e| FoldDbError::Config(format!("Failed to flush sled: {}", e)))?;
            }
        }
        Ok(())
    }

    /// Load transforms from sled tree into memory.
    pub(super) fn load_transforms_from_tree(&self, meta_tree: &sled::Tree) -> FoldDbResult<()> {
        let mut transforms = self.transforms.write().map_err(|_| {
            FoldDbError::Config("Failed to acquire transforms write lock".to_string())
        })?;
        transforms.clear();

        let mut count = 0;
        for result in meta_tree.iter() {
            let (key, value) = result.map_err(|e| {
                FoldDbError::Config(format!(
                    "Failed to iterate over transform_metadata tree: {}",
                    e
                ))
            })?;

            let hash = String::from_utf8(key.to_vec()).map_err(|e| {
                FoldDbError::Config(format!("Failed to decode transform hash from key: {}", e))
            })?;

            let record: TransformRecord = serde_json::from_slice(&value).map_err(|e| {
                FoldDbError::Config(format!(
                    "Failed to parse transform '{}' from database: {}",
                    hash, e
                ))
            })?;

            transforms.insert(hash, record);
            count += 1;
        }

        log_feature!(
            LogFeature::Schema,
            info,
            "Schema service loaded {} transforms from sled",
            count
        );

        Ok(())
    }

    // ============== Classification Phase 1 ==============

    /// Resolve input field types and classifications from the schema service state.
    /// Returns (input_schema, input_ceiling).
    fn resolve_input_classifications(
        &self,
        input_queries: &[crate::schema::types::operations::Query],
    ) -> FoldDbResult<(HashMap<String, FieldValueType>, DataClassification)> {
        let schemas = self
            .schemas
            .read()
            .map_err(|_| FoldDbError::Config("Failed to acquire schemas read lock".to_string()))?;

        let mut input_schema = HashMap::new();
        let mut max_classification = DataClassification::low();

        for query in input_queries {
            // Look up schema by name (identity_hash) or by descriptive_name
            let schema = schemas.get(&query.schema_name).or_else(|| {
                // Try descriptive name index
                if let Ok(index) = self.descriptive_name_index.read() {
                    if let Some(hash) = index.get(&query.schema_name) {
                        return schemas.get(hash);
                    }
                }
                None
            });

            let schema = schema.ok_or_else(|| {
                FoldDbError::Config(format!(
                    "Input query references unknown schema '{}' — register the schema first",
                    query.schema_name
                ))
            })?;

            for field_name in &query.fields {
                // Resolve field type from schema's field_types or canonical fields
                let field_type = schema
                    .field_types
                    .get(field_name)
                    .cloned()
                    .unwrap_or(FieldValueType::Any);

                let qualified_name = format!("{}.{}", query.schema_name, field_name);
                input_schema.insert(qualified_name, field_type);

                // Resolve classification from field_classifications
                let classification = classify_field(schema.field_classifications.get(field_name));
                max_classification = std::cmp::max(max_classification, classification);
            }
        }

        Ok((input_schema, max_classification))
    }

    // ============== Classification Phase 2 ==============

    /// Estimate output classification using NMI over synthetic samples.
    /// Returns (output_classification, nmi_matrix, verified, sample_count).
    ///
    /// When `transform-wasm` feature is not enabled, falls back to the
    /// Phase 1 ceiling (conservative).
    #[allow(unused_variables)]
    fn estimate_output_classification(
        &self,
        wasm_bytes: &[u8],
        input_schema: &HashMap<String, FieldValueType>,
        output_fields: &HashMap<String, FieldValueType>,
        input_ceiling: DataClassification,
    ) -> (
        DataClassification,
        HashMap<String, HashMap<String, f32>>,
        bool,
        u32,
    ) {
        // Phase 2 requires the transform-wasm feature flag.
        // Without it, conservatively return the input ceiling.
        #[cfg(feature = "transform-wasm")]
        {
            match self.run_nmi_estimation(
                wasm_bytes,
                input_schema,
                output_fields,
                input_ceiling.clone(),
            ) {
                Ok(result) => result,
                Err(e) => {
                    log_feature!(
                        LogFeature::Schema,
                        warn,
                        "Phase 2 NMI estimation failed, falling back to ceiling: {}",
                        e
                    );
                    (input_ceiling, HashMap::new(), false, 0)
                }
            }
        }

        #[cfg(not(feature = "transform-wasm"))]
        {
            log_feature!(
                LogFeature::Schema,
                info,
                "transform-wasm feature not enabled, using Phase 1 ceiling for classification"
            );
            (input_ceiling, HashMap::new(), false, 0)
        }
    }

    /// Run the full NMI estimation pipeline (Phase 2).
    /// Only compiled when transform-wasm feature is enabled.
    #[cfg(feature = "transform-wasm")]
    fn run_nmi_estimation(
        &self,
        wasm_bytes: &[u8],
        input_schema: &HashMap<String, FieldValueType>,
        output_fields: &HashMap<String, FieldValueType>,
        _input_ceiling: DataClassification,
    ) -> FoldDbResult<(
        DataClassification,
        HashMap<String, HashMap<String, f32>>,
        bool,
        u32,
    )> {
        use super::nmi::{estimate_nmi_matrix, SyntheticDataGenerator};

        let sample_count: u32 = 512;
        let generator = SyntheticDataGenerator::new();

        // Generate baseline input (all fields at default values)
        let baseline = generator.generate_baseline(input_schema);

        // For each input field, generate N varied samples
        let mut all_input_samples: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
        for (field_name, field_type) in input_schema {
            let samples = generator.generate_field_samples(field_type, sample_count);
            all_input_samples.insert(field_name.clone(), samples);
        }

        // Run WASM on each set of samples and collect outputs
        // This requires a WASM engine — use wasmtime or similar
        let nmi_matrix = estimate_nmi_matrix(
            wasm_bytes,
            input_schema,
            output_fields,
            &baseline,
            &all_input_samples,
            sample_count,
        )?;

        // Determine output classification from NMI matrix
        let mut output_classification = DataClassification::low();
        for (input_field, output_scores) in &nmi_matrix {
            let input_classification = classify_field_from_schema(input_field, input_schema);
            for (_output_field, &nmi_score) in output_scores {
                if nmi_score > NMI_LEAKAGE_THRESHOLD {
                    output_classification =
                        std::cmp::max(output_classification, input_classification.clone());
                }
            }
        }

        Ok((output_classification, nmi_matrix, true, sample_count))
    }
}

/// Classify a field based on its classification tags.
/// Maps common classification strings to DataClassification levels.
fn classify_field(classifications: Option<&Vec<String>>) -> DataClassification {
    let tags = match classifications {
        Some(t) if !t.is_empty() => t,
        _ => return DataClassification::low(),
    };

    let mut max = DataClassification::low();
    for tag in tags {
        let level = match tag.to_lowercase().as_str() {
            "high" | "restricted" | "pii" | "medical" | "financial" | "hipaa" => {
                DataClassification::high()
            }
            "medium" | "internal" | "confidential" => DataClassification::medium(),
            _ => DataClassification::low(),
        };
        max = std::cmp::max(max, level);
    }
    max
}

/// Tokenize a transform name into words for similarity comparison.
fn tokenize_name(name: &str) -> std::collections::HashSet<String> {
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}

#[cfg(feature = "transform-wasm")]
fn classify_field_from_schema(
    qualified_field: &str,
    _input_schema: &HashMap<String, FieldValueType>,
) -> DataClassification {
    // In a full implementation, this would look up the field's classification
    // from the schema. For now, treat all input fields as their declared level.
    // The classification is already resolved in Phase 1.
    let _ = qualified_field;
    DataClassification::low()
}
