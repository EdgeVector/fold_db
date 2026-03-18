use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use crate::db_operations::native_index::{Embedder, FastEmbedModel};
use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::Schema;
#[cfg(feature = "aws-backend")]
use crate::storage::DynamoDbSchemaStore;

#[cfg(feature = "aws-backend")]
pub use crate::storage::CloudConfig;

use super::state_matching::collect_field_names;
pub use super::state_matching::jaccard_index;
use super::types::{
    AddViewRequest, SchemaAddOutcome, SchemaLookupEntry, SchemaReuseMatch, SimilarSchemaEntry,
    SimilarSchemasResponse, StoredView, ViewAddOutcome,
};


/// Storage backend for the schema service
#[derive(Clone)]
pub enum SchemaStorage {
    /// Local sled database (default)
    Sled {
        db: sled::Db,
        schemas_tree: sled::Tree,
    },
    /// Cloud storage (DynamoDB etc) (serverless, no locking needed!)
    #[cfg(feature = "aws-backend")]
    Cloud { store: Arc<DynamoDbSchemaStore> },
}

/// Shared state for the schema service
#[derive(Clone)]
pub struct SchemaServiceState {
    pub schemas: Arc<RwLock<HashMap<String, Schema>>>,
    /// Secondary index: descriptive_name -> schema_name (identity_hash)
    pub descriptive_name_index: Arc<RwLock<HashMap<String, String>>>,
    /// Cached embeddings for descriptive names: descriptive_name -> embedding vector
    pub descriptive_name_embeddings: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    /// Cached embeddings for context-enriched field names: "desc_name:field_name" -> embedding
    pub field_embeddings: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    /// Global canonical field registry: canonical_name -> CanonicalField (description + type).
    /// New schema proposals have their field names matched against this list
    /// so that semantically equivalent fields use consistent names across all schemas.
    pub canonical_fields: Arc<RwLock<HashMap<String, super::types::CanonicalField>>>,
    /// Cached embeddings for canonical field names
    pub canonical_field_embeddings: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    /// Text embedding model for semantic descriptive name matching
    pub embedder: Arc<dyn Embedder>,
    pub storage: SchemaStorage,
    /// Registered views: view_name -> StoredView
    pub views: Arc<RwLock<HashMap<String, StoredView>>>,
}

impl SchemaServiceState {
    /// Create a new schema service state with local sled storage
    pub fn new(db_path: String) -> FoldDbResult<Self> {
        let db = sled::open(&db_path).map_err(|e| {
            FoldDbError::Config(format!(
                "Failed to open schema service database at '{}': {}",
                db_path, e
            ))
        })?;

        let schemas_tree = db
            .open_tree("schemas")
            .map_err(|e| FoldDbError::Config(format!("Failed to open schemas tree: {}", e)))?;

        let canonical_fields_tree = db
            .open_tree("canonical_fields")
            .map_err(|e| FoldDbError::Config(format!("Failed to open canonical_fields tree: {}", e)))?;

        let views_tree = db
            .open_tree("views")
            .map_err(|e| FoldDbError::Config(format!("Failed to open views tree: {}", e)))?;

        let state = Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
            descriptive_name_index: Arc::new(RwLock::new(HashMap::new())),
            descriptive_name_embeddings: Arc::new(RwLock::new(HashMap::new())),
            field_embeddings: Arc::new(RwLock::new(HashMap::new())),
            canonical_fields: Arc::new(RwLock::new(HashMap::new())),
            canonical_field_embeddings: Arc::new(RwLock::new(HashMap::new())),
            embedder: Arc::new(FastEmbedModel::new()),
            storage: SchemaStorage::Sled { db, schemas_tree },
            views: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load schemas synchronously for sled
        state.load_schemas_sync()?;
        state.rebuild_descriptive_name_index();
        state.load_canonical_fields_from_tree(&canonical_fields_tree)?;
        state.load_views_from_tree(&views_tree)?;

        Ok(state)
    }

    /// Synchronous version of load_schemas for Sled storage
    fn load_schemas_sync(&self) -> FoldDbResult<()> {
        let mut schemas = self
            .schemas
            .write()
            .map_err(|_| FoldDbError::Config("Failed to acquire schemas write lock".to_string()))?;

        schemas.clear();

        match &self.storage {
            SchemaStorage::Sled { schemas_tree, .. } => {
                let mut count = 0;
                for result in schemas_tree.iter() {
                    let (key, value) = result.map_err(|e| {
                        FoldDbError::Config(format!("Failed to iterate over schemas tree: {}", e))
                    })?;

                    let name = String::from_utf8(key.to_vec()).map_err(|e| {
                        FoldDbError::Config(format!("Failed to decode schema name from key: {}", e))
                    })?;

                    let schema: Schema = serde_json::from_slice(&value).map_err(|e| {
                        FoldDbError::Config(format!(
                            "Failed to parse schema '{}' from database: {}",
                            name, e
                        ))
                    })?;

                    schemas.insert(name, schema);
                    count += 1;
                }

                log_feature!(
                    LogFeature::Schema,
                    info,
                    "Schema service loaded {} schemas from sled",
                    count
                );
            }
            #[cfg(feature = "aws-backend")]
            _ => {
                return Err(FoldDbError::Config(
                    "load_schemas_sync called on non-Sled storage".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Create a new schema service state with Cloud storage
    /// No locking needed - identity hashes ensure idempotent writes!
    #[cfg(feature = "aws-backend")]
    pub async fn new_with_cloud(config: CloudConfig) -> FoldDbResult<Self> {
        log_feature!(
            LogFeature::Schema,
            info,
            "Initializing schema service with DynamoDB in region: {}",
            config.region
        );

        let store = DynamoDbSchemaStore::new(config).await?;

        let state = Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
            descriptive_name_index: Arc::new(RwLock::new(HashMap::new())),
            descriptive_name_embeddings: Arc::new(RwLock::new(HashMap::new())),
            field_embeddings: Arc::new(RwLock::new(HashMap::new())),
            canonical_fields: Arc::new(RwLock::new(HashMap::new())),
            canonical_field_embeddings: Arc::new(RwLock::new(HashMap::new())),
            embedder: Arc::new(FastEmbedModel::new()),
            storage: SchemaStorage::Cloud {
                store: Arc::new(store),
            },
            views: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load schemas on initialization
        state.load_schemas().await?;
        state.rebuild_descriptive_name_index();
        state.rebuild_canonical_fields_from_schemas();
        state.load_views().await?;

        log_feature!(
            LogFeature::Schema,
            info,
            "Schema service initialized with DynamoDB, loaded {} schemas",
            state.schemas.read().map(|s| s.len()).unwrap_or(0)
        );

        Ok(state)
    }

    /// Load all schemas from storage (works for both Sled and DynamoDB)
    pub async fn load_schemas(&self) -> FoldDbResult<()> {
        match &self.storage {
            SchemaStorage::Sled { schemas_tree, .. } => {
                let mut schemas = self.schemas.write().map_err(|_| {
                    FoldDbError::Config("Failed to acquire schemas write lock".to_string())
                })?;

                schemas.clear();
                let mut count = 0;
                for result in schemas_tree.iter() {
                    let (key, value) = result.map_err(|e| {
                        FoldDbError::Config(format!("Failed to iterate over schemas tree: {}", e))
                    })?;

                    let name = String::from_utf8(key.to_vec()).map_err(|e| {
                        FoldDbError::Config(format!("Failed to decode schema name from key: {}", e))
                    })?;

                    let schema: Schema = serde_json::from_slice(&value).map_err(|e| {
                        FoldDbError::Config(format!(
                            "Failed to parse schema '{}' from database: {}",
                            name, e
                        ))
                    })?;

                    log_feature!(
                        LogFeature::Schema,
                        info,
                        "Loaded schema '{}' from sled database",
                        name
                    );

                    schemas.insert(name, schema);
                    count += 1;
                }

                log_feature!(
                    LogFeature::Schema,
                    info,
                    "Schema service loaded {} schemas from sled",
                    count
                );
            }
            #[cfg(feature = "aws-backend")]
            SchemaStorage::Cloud { store } => {
                let all_schemas = store.get_all_schemas().await?;
                let count = all_schemas.len();

                let mut schemas = self.schemas.write().map_err(|_| {
                    FoldDbError::Config("Failed to acquire schemas write lock".to_string())
                })?;

                schemas.clear();

                for schema in all_schemas {
                    log_feature!(
                        LogFeature::Schema,
                        info,
                        "Loaded schema '{}' from DynamoDB",
                        schema.name
                    );
                    schemas.insert(schema.name.clone(), schema);
                }

                log_feature!(
                    LogFeature::Schema,
                    info,
                    "Schema service loaded {} schemas from DynamoDB",
                    count
                );
            }
        }

        Ok(())
    }

    /// Rebuild the descriptive_name -> schema_name index and embeddings cache.
    fn rebuild_descriptive_name_index(&self) {
        let schemas = match self.schemas.read() {
            Ok(s) => s,
            Err(_) => return,
        };
        let mut index = match self.descriptive_name_index.write() {
            Ok(i) => i,
            Err(_) => return,
        };
        let mut embeddings = match self.descriptive_name_embeddings.write() {
            Ok(e) => e,
            Err(_) => return,
        };
        index.clear();
        embeddings.clear();
        for (name, schema) in schemas.iter() {
            if let Some(ref desc) = schema.descriptive_name {
                index.insert(desc.clone(), name.clone());
                match self.embedder.embed_text(desc) {
                    Ok(vec) => { embeddings.insert(desc.clone(), vec); }
                    Err(e) => {
                        log_feature!(
                            LogFeature::Schema,
                            warn,
                            "Failed to embed descriptive_name '{}': {}",
                            desc,
                            e
                        );
                    }
                }
            }
        }
    }

    /// Create a schema service state with a custom embedder (for testing).
    #[cfg(any(test, feature = "test-utils"))]
    pub fn new_with_embedder(db_path: String, embedder: Arc<dyn Embedder>) -> FoldDbResult<Self> {
        let db = sled::open(&db_path).map_err(|e| {
            FoldDbError::Config(format!(
                "Failed to open schema service database at '{}': {}",
                db_path, e
            ))
        })?;

        let schemas_tree = db
            .open_tree("schemas")
            .map_err(|e| FoldDbError::Config(format!("Failed to open schemas tree: {}", e)))?;
        let canonical_fields_tree = db
            .open_tree("canonical_fields")
            .map_err(|e| FoldDbError::Config(format!("Failed to open canonical_fields tree: {}", e)))?;
        let views_tree = db
            .open_tree("views")
            .map_err(|e| FoldDbError::Config(format!("Failed to open views tree: {}", e)))?;

        let state = Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
            descriptive_name_index: Arc::new(RwLock::new(HashMap::new())),
            descriptive_name_embeddings: Arc::new(RwLock::new(HashMap::new())),
            field_embeddings: Arc::new(RwLock::new(HashMap::new())),
            canonical_fields: Arc::new(RwLock::new(HashMap::new())),
            canonical_field_embeddings: Arc::new(RwLock::new(HashMap::new())),
            embedder,
            storage: SchemaStorage::Sled { db, schemas_tree },
            views: Arc::new(RwLock::new(HashMap::new())),
        };

        state.load_schemas_sync()?;
        state.rebuild_descriptive_name_index();
        state.load_canonical_fields_from_tree(&canonical_fields_tree)?;
        state.load_views_from_tree(&views_tree)?;

        Ok(state)
    }

    pub async fn add_schema(
        &self,
        mut schema: Schema,
        mut mutation_mappers: HashMap<String, String>,
    ) -> FoldDbResult<SchemaAddOutcome> {
        // descriptive_name is required — it's how schemas are identified, displayed,
        // and matched for expansion. A schema without one is a bug in the caller.
        if schema.descriptive_name.as_ref().is_none_or(|dn| dn.trim().is_empty()) {
            return Err(FoldDbError::Config(
                "Schema must have a non-empty descriptive_name".to_string(),
            ));
        }

        // field_descriptions is required — the schema service uses them for
        // semantic field matching (embedding "field_name: description").
        // Without descriptions, field matching degrades to bare name comparison.
        if let Some(ref fields) = schema.fields {
            let missing: Vec<&String> = fields
                .iter()
                .filter(|f| !schema.field_descriptions.contains_key(*f))
                .collect();
            if !missing.is_empty() {
                return Err(FoldDbError::Config(format!(
                    "Schema fields missing descriptions (required for semantic matching): {:?}",
                    missing
                )));
            }
        }

        // Canonicalize field names against the global canonical field registry
        // before any dedup or identity hash computation.
        if let Some(ref fields) = schema.fields {
            let rename_map = self.canonicalize_fields(fields, &schema, &mut mutation_mappers);
            if !rename_map.is_empty() {
                Self::apply_field_renames(&mut schema, &rename_map, &mut mutation_mappers);
                // Canonicalization changed field names, so any precomputed identity
                // hash is stale — force recomputation below.
                schema.identity_hash = None;
            }
        }

        // Deduplicate fields before computing identity hash
        schema.dedup_fields();

        // Compute (or recompute after canonicalization) the identity hash.
        schema.compute_identity_hash();

        // Get the original schema name before we modify it
        let original_schema_name = schema.name.clone();

        // Use identity_hash as the schema identifier
        let identity_hash = schema
            .get_identity_hash()
            .ok_or_else(|| {
                FoldDbError::Config("Schema must have identity_hash computed".to_string())
            })?
            .clone();

        log_feature!(
            LogFeature::Schema,
            info,
            "Schema '{}' identity_hash: {}",
            original_schema_name,
            identity_hash
        );

        // Schema name is ALWAYS the identity_hash (hash of semantic name + fields).
        // This guarantees:
        // - Same semantic name + same fields = same hash = dedup
        // - Same semantic name + different fields = different hash = separate schemas
        // - Different semantic name + same fields = different hash = separate schemas
        // The human-readable name lives in descriptive_name (for display/search).
        let schema_name = identity_hash.clone();

        // Check if this exact schema already exists (same name)
        {
            let schemas = self.schemas.read().map_err(|_| {
                FoldDbError::Config("Failed to acquire schemas read lock".to_string())
            })?;

            if let Some(existing_schema) = schemas.get(&schema_name) {
                // If this schema has been superseded by expansion, redirect to the
                // current active schema for the subset/expansion check.
                let (check_schema, check_name) = self
                    .resolve_active_schema(existing_schema, &schema_name, &schemas)
                    .unwrap_or_else(|| (existing_schema.clone(), schema_name.clone()));

                // Check if the incoming schema has new fields not in the target schema.
                // If so, fall through to expansion instead of returning AlreadyExists.
                let existing_fields: HashSet<String> = check_schema
                    .fields
                    .as_ref()
                    .map(|f| f.iter().cloned().collect())
                    .unwrap_or_default();
                let incoming_fields: HashSet<String> = schema
                    .fields
                    .as_ref()
                    .map(|f| f.iter().cloned().collect())
                    .unwrap_or_default();
                let has_new_fields = !incoming_fields.is_subset(&existing_fields);

                if has_new_fields {
                    log_feature!(
                        LogFeature::Schema,
                        info,
                        "Schema '{}' (active='{}') has new fields {:?} — expanding",
                        schema_name,
                        check_name,
                        incoming_fields.difference(&existing_fields).collect::<Vec<_>>()
                    );
                    // Fall through to expansion path below
                } else {
                    log_feature!(
                        LogFeature::Schema,
                        info,
                        "Schema '{}' already exists with same fields (active='{}') - returning existing",
                        schema_name,
                        check_name
                    );

                    return Ok(SchemaAddOutcome::AlreadyExists(check_schema, mutation_mappers.clone()));
                }
            }
        }

        // Check for schema expansion: if the new schema has a descriptive_name that
        // matches an existing schema (exact or semantic), merge fields (expand, never shrink).
        if let Some(incoming_desc_name) = schema.descriptive_name.clone() {
            let (matched_desc, existing_schema_name, is_exact_match) = self.find_matching_descriptive_name(&incoming_desc_name)?;

            // For semantic (non-exact) matches, use descriptive names as a second gate.
            // "holiday_illustrations" and "famous_paintings" have similar descriptive names
            // (both art-related) but are clearly different collections. Only merge when
            // the descriptive names are semantically close enough.
            // NOTE: schema names are now identity hashes, so we must compare the
            // human-readable descriptive_name strings, not the hash-based schema names.
            let should_merge = if let Some(ref _old_name) = existing_schema_name {
                if is_exact_match {
                    true
                } else if let Some(ref canonical_desc) = matched_desc {
                    // Compare the human-readable descriptive names
                    self.schema_names_are_similar(&incoming_desc_name, canonical_desc)
                } else {
                    false
                }
            } else {
                false
            };

            if should_merge {
                let old_name = existing_schema_name.unwrap();
                // If matched via semantic similarity, adopt the existing descriptive_name
                // so the index stays consistent.
                if let Some(ref canonical_desc) = matched_desc {
                    if *canonical_desc != incoming_desc_name {
                        log_feature!(
                            LogFeature::Schema,
                            info,
                            "Semantic match: incoming '{}' matched existing '{}'",
                            incoming_desc_name,
                            canonical_desc
                        );
                        schema.descriptive_name = Some(canonical_desc.clone());
                    }
                }
                // Use the (possibly canonical) descriptive_name for the rest of expansion
                let desc_name = schema.descriptive_name.clone().unwrap_or(incoming_desc_name);
                // We already checked exact-hash match above, so the old schema
                // has a different (smaller) field set. Merge fields as a superset.
                let old_schema = {
                    let schemas = self.schemas.read().map_err(|_| {
                        FoldDbError::Config("Failed to acquire schemas read lock".to_string())
                    })?;
                    schemas.get(&old_name).cloned()
                };

                if let Some(existing) = old_schema {
                    let existing_fields = existing.fields.clone().unwrap_or_default();

                    // Semantic field matching: detect synonyms like "creator" ≈ "artist"
                    // and rename incoming fields to canonical names before expansion.
                    let incoming_fields = schema.fields.clone().unwrap_or_default();
                    let rename_map = self.semantic_field_rename_map(
                        &incoming_fields,
                        &existing_fields,
                        &desc_name,
                        &schema.field_descriptions,
                        &existing.field_descriptions,
                    );
                    let mut mutation_mappers = mutation_mappers;
                    Self::apply_field_renames(&mut schema, &rename_map, &mut mutation_mappers);

                    // Deduplicate fields after renaming (renamed fields may now
                    // duplicate existing ones)
                    schema.dedup_fields();

                    return self
                        .expand_schema(&mut schema, &existing, &old_name, &desc_name, &mutation_mappers)
                        .await;
                }
            }
        }

        // No field-overlap fallback needed — descriptive_name is required (validated above),
        // so descriptive_name matching handles all expansion cases.

        schema.name = schema_name.clone();

        // Persist to storage backend
        self.persist_schema(&schema, &mutation_mappers).await?;

        // Insert into in-memory cache and update descriptive_name index
        {
            let mut schemas = self.schemas.write().map_err(|_| {
                FoldDbError::Config("Failed to acquire schemas write lock".to_string())
            })?;
            schemas.insert(schema_name.clone(), schema.clone());
        }

        if let Some(ref desc_name) = schema.descriptive_name {
            let mut index = self.descriptive_name_index.write().map_err(|_| {
                FoldDbError::Config("Failed to acquire descriptive_name_index write lock".to_string())
            })?;
            index.insert(desc_name.clone(), schema_name.clone());
            drop(index);

            // Cache embedding for new descriptive_name
            if let Ok(vec) = self.embedder.embed_text(desc_name) {
                if let Ok(mut embeddings) = self.descriptive_name_embeddings.write() {
                    embeddings.insert(desc_name.clone(), vec);
                }
            }
        }

        // Register new fields as canonical for future schema proposals.
        // Fails if classification cannot be determined (no ANTHROPIC_API_KEY for new fields).
        self.register_canonical_fields(&schema).await?;

        // Propagate canonical field types and classifications to the schema
        self.apply_canonical_types(&mut schema);
        self.apply_canonical_classifications(&mut schema);

        log_feature!(
            LogFeature::Schema,
            info,
            "Schema '{}' successfully added to registry",
            schema_name
        );

        Ok(SchemaAddOutcome::Added(schema, mutation_mappers))
    }

    /// Persist a schema to the storage backend.
    #[allow(unused_variables)]
    pub(super) async fn persist_schema(
        &self,
        schema: &Schema,
        mutation_mappers: &HashMap<String, String>,
    ) -> FoldDbResult<()> {
        match &self.storage {
            SchemaStorage::Sled { db, schemas_tree } => {
                let serialized = serde_json::to_vec(schema).map_err(|e| {
                    FoldDbError::Serialization(format!(
                        "Failed to serialize schema '{}': {}", schema.name, e
                    ))
                })?;
                schemas_tree
                    .insert(schema.name.as_bytes(), serialized)
                    .map_err(|e| {
                        FoldDbError::Config(format!(
                            "Failed to insert schema '{}' into sled: {}", schema.name, e
                        ))
                    })?;
                db.flush().map_err(|e| {
                    FoldDbError::Config(format!("Failed to flush sled: {}", e))
                })?;
                log_feature!(LogFeature::Schema, info, "Schema '{}' persisted to sled", schema.name);
            }
            #[cfg(feature = "aws-backend")]
            SchemaStorage::Cloud { store } => {
                store.put_schema(schema, mutation_mappers).await?;
                log_feature!(LogFeature::Schema, info, "Schema '{}' persisted to DynamoDB", schema.name);
            }
        }
        Ok(())
    }

    /// Get all schema names (public accessor for Lambda integration)
    pub fn get_schema_names(&self) -> FoldDbResult<Vec<String>> {
        let schemas = self
            .schemas
            .read()
            .map_err(|_| FoldDbError::Config("Failed to acquire schemas read lock".to_string()))?;
        Ok(schemas.keys().cloned().collect())
    }

    /// Get all schemas (public accessor for Lambda integration)
    pub fn get_all_schemas_cached(&self) -> FoldDbResult<Vec<Schema>> {
        let schemas = self
            .schemas
            .read()
            .map_err(|_| FoldDbError::Config("Failed to acquire schemas read lock".to_string()))?;
        Ok(schemas.values().cloned().collect())
    }

    /// Get a schema by name (public accessor for Lambda integration)
    pub fn get_schema_by_name(&self, name: &str) -> FoldDbResult<Option<Schema>> {
        let schemas = self
            .schemas
            .read()
            .map_err(|_| FoldDbError::Config("Failed to acquire schemas read lock".to_string()))?;
        Ok(schemas.get(name).cloned())
    }

    /// Get schema count (public accessor for Lambda integration)
    pub fn get_schema_count(&self) -> usize {
        self.schemas.read().map(|s| s.len()).unwrap_or(0)
    }

    /// Find schemas similar to the given schema using Jaccard index on field name sets
    pub fn find_similar_schemas(
        &self,
        name: &str,
        threshold: f64,
    ) -> FoldDbResult<SimilarSchemasResponse> {
        let schemas = self
            .schemas
            .read()
            .map_err(|_| FoldDbError::Config("Failed to acquire schemas read lock".to_string()))?;

        let target = schemas.get(name).ok_or_else(|| {
            FoldDbError::Config(format!("Schema '{}' not found", name))
        })?;

        let target_fields = collect_field_names(target);

        let mut similar: Vec<SimilarSchemaEntry> = schemas
            .iter()
            .filter(|(k, _)| k.as_str() != name)
            .filter_map(|(_, schema)| {
                let other_fields = collect_field_names(schema);
                let similarity = jaccard_index(&target_fields, &other_fields);
                if similarity >= threshold {
                    Some(SimilarSchemaEntry {
                        schema: schema.clone(),
                        similarity,
                    })
                } else {
                    None
                }
            })
            .collect();

        similar.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));

        Ok(SimilarSchemasResponse {
            query_schema: name.to_string(),
            threshold,
            similar_schemas: similar,
        })
    }

    /// Batch check whether proposed schemas can reuse existing ones.
    ///
    /// For each entry, finds a matching descriptive name (exact or semantic),
    /// resolves to the active (non-deprecated) schema, computes field rename
    /// maps, and determines if the existing schema is a superset.
    ///
    /// Read-only operation — only acquires read locks.
    pub fn batch_check_schema_reuse(
        &self,
        entries: &[SchemaLookupEntry],
    ) -> FoldDbResult<HashMap<String, SchemaReuseMatch>> {
        let mut results = HashMap::new();

        let schemas = self.schemas.read().map_err(|_| {
            FoldDbError::Config("Failed to acquire schemas_cache read lock".to_string())
        })?;

        for entry in entries {
            // 1. Find matching descriptive name (exact or semantic)
            let (matched_desc, matched_hash, is_exact) =
                match self.find_matching_descriptive_name(&entry.descriptive_name) {
                    Ok((Some(desc), Some(hash), exact)) => (desc, hash, exact),
                    Ok(_) => continue,
                    Err(e) => {
                        log_feature!(
                            LogFeature::Schema,
                            warn,
                            "batch_check_schema_reuse: error matching '{}': {}",
                            entry.descriptive_name,
                            e
                        );
                        continue;
                    }
                };

            // 2. Resolve to active (non-deprecated) schema
            let existing = match schemas.get(&matched_hash) {
                Some(s) => s,
                None => continue,
            };
            let (active_schema, _active_name) =
                match self.resolve_active_schema(existing, &matched_hash, &schemas) {
                    Some(pair) => pair,
                    None => (existing.clone(), matched_hash.clone()),
                };

            // 3. Get the active schema's fields
            let existing_fields: Vec<String> = active_schema
                .fields
                .as_ref()
                .cloned()
                .unwrap_or_default();

            // 4. Compute semantic field rename map
            let field_rename_map = self.semantic_field_rename_map(
                &entry.fields,
                &existing_fields,
                &entry.descriptive_name,
                &HashMap::new(),
                &active_schema.field_descriptions,
            );

            // 5. Determine superset status and unmapped fields
            let existing_set: HashSet<&String> = existing_fields.iter().collect();
            let mut unmapped = Vec::new();
            for f in &entry.fields {
                if !existing_set.contains(f) && !field_rename_map.contains_key(f) {
                    unmapped.push(f.clone());
                }
            }
            let is_superset = unmapped.is_empty();

            results.insert(
                entry.descriptive_name.clone(),
                SchemaReuseMatch {
                    schema: active_schema,
                    matched_descriptive_name: matched_desc,
                    is_exact_match: is_exact,
                    field_rename_map,
                    is_superset,
                    unmapped_fields: unmapped,
                },
            );
        }

        Ok(results)
    }

    // ============== View Methods ==============

    /// Register a view: build an output schema from the view's fields, run it through
    /// add_schema (getting similarity/canonicalization/dedup/expansion), then store
    /// the view definition separately.
    pub async fn add_view(&self, request: AddViewRequest) -> FoldDbResult<ViewAddOutcome> {
        // Validate view name
        if request.name.trim().is_empty() {
            return Err(FoldDbError::Config("View name must be non-empty".to_string()));
        }

        // Validate input queries have explicit field lists
        for (i, query) in request.input_queries.iter().enumerate() {
            if query.fields.is_empty() {
                return Err(FoldDbError::Config(format!(
                    "Input query {} must have explicit fields",
                    i
                )));
            }
        }

        // Validate output fields are non-empty
        if request.output_fields.is_empty() {
            return Err(FoldDbError::Config(
                "View must have at least one output field".to_string(),
            ));
        }

        // Validate all output fields have descriptions
        let missing: Vec<&String> = request
            .output_fields
            .iter()
            .filter(|f| !request.field_descriptions.contains_key(*f))
            .collect();
        if !missing.is_empty() {
            return Err(FoldDbError::Config(format!(
                "Output fields missing descriptions: {:?}",
                missing
            )));
        }

        // Validate no duplicate (schema, field) pairs across input queries
        {
            let mut seen = HashSet::new();
            for query in &request.input_queries {
                for field in &query.fields {
                    let key = format!("{}.{}", query.schema_name, field);
                    if !seen.insert(key.clone()) {
                        return Err(FoldDbError::Config(format!(
                            "Duplicate (schema, field) pair in input queries: {}",
                            key
                        )));
                    }
                }
            }
        }

        // Build an output schema from the view's fields and run it through add_schema
        let mut output_schema = Schema::new(
            request.name.clone(),
            crate::schema::types::schema::DeclarativeSchemaType::Single,
            None,
            Some(request.output_fields.clone()),
            None,
            None,
        );
        output_schema.descriptive_name = Some(request.descriptive_name.clone());
        output_schema.field_descriptions = request.field_descriptions.clone();
        output_schema.field_classifications = request.field_classifications.clone();
        output_schema.field_data_classifications = request.field_data_classifications.clone();
        output_schema.schema_type = request.schema_type.clone();

        // Run through the full schema pipeline (similarity, canonicalization, dedup, expansion)
        let schema_outcome = self
            .add_schema(output_schema, HashMap::new())
            .await?;

        let (output_schema, _replaced_schema) = match &schema_outcome {
            SchemaAddOutcome::Added(schema, _) => (schema.clone(), None),
            SchemaAddOutcome::AlreadyExists(schema, _) => (schema.clone(), None),
            SchemaAddOutcome::Expanded(old_name, schema, _) => {
                (schema.clone(), Some(old_name.clone()))
            }
        };

        // Build the StoredView
        let view = StoredView {
            name: request.name.clone(),
            input_queries: request.input_queries,
            wasm_bytes: request.wasm_bytes,
            output_schema_name: output_schema.name.clone(),
            schema_type: request.schema_type,
        };

        // Persist the view
        self.persist_view(&view).await?;

        // Insert into in-memory cache
        {
            let mut views = self.views.write().map_err(|_| {
                FoldDbError::Config("Failed to acquire views write lock".to_string())
            })?;
            views.insert(view.name.clone(), view.clone());
        }

        log_feature!(
            LogFeature::Schema,
            info,
            "View '{}' registered with output schema '{}'",
            view.name,
            view.output_schema_name
        );

        match schema_outcome {
            SchemaAddOutcome::Added(..) => Ok(ViewAddOutcome::Added(view, output_schema)),
            SchemaAddOutcome::AlreadyExists(..) => {
                Ok(ViewAddOutcome::AddedWithExistingSchema(view, output_schema))
            }
            SchemaAddOutcome::Expanded(old_name, ..) => {
                Ok(ViewAddOutcome::Expanded(view, output_schema, old_name))
            }
        }
    }

    /// Get all view names
    pub fn get_view_names(&self) -> FoldDbResult<Vec<String>> {
        let views = self
            .views
            .read()
            .map_err(|_| FoldDbError::Config("Failed to acquire views read lock".to_string()))?;
        Ok(views.keys().cloned().collect())
    }

    /// Get all views
    pub fn get_all_views(&self) -> FoldDbResult<Vec<StoredView>> {
        let views = self
            .views
            .read()
            .map_err(|_| FoldDbError::Config("Failed to acquire views read lock".to_string()))?;
        Ok(views.values().cloned().collect())
    }

    /// Get a view by name
    pub fn get_view_by_name(&self, name: &str) -> FoldDbResult<Option<StoredView>> {
        let views = self
            .views
            .read()
            .map_err(|_| FoldDbError::Config("Failed to acquire views read lock".to_string()))?;
        Ok(views.get(name).cloned())
    }

    /// Persist a view to the storage backend
    #[allow(unused_variables)]
    async fn persist_view(&self, view: &StoredView) -> FoldDbResult<()> {
        match &self.storage {
            SchemaStorage::Sled { db, .. } => {
                let views_tree = db
                    .open_tree("views")
                    .map_err(|e| FoldDbError::Config(format!("Failed to open views tree: {}", e)))?;
                let serialized = serde_json::to_vec(view).map_err(|e| {
                    FoldDbError::Serialization(format!(
                        "Failed to serialize view '{}': {}",
                        view.name, e
                    ))
                })?;
                views_tree
                    .insert(view.name.as_bytes(), serialized)
                    .map_err(|e| {
                        FoldDbError::Config(format!(
                            "Failed to insert view '{}' into sled: {}",
                            view.name, e
                        ))
                    })?;
                db.flush()
                    .map_err(|e| FoldDbError::Config(format!("Failed to flush sled: {}", e)))?;
                log_feature!(LogFeature::Schema, info, "View '{}' persisted to sled", view.name);
            }
            #[cfg(feature = "aws-backend")]
            SchemaStorage::Cloud { store } => {
                // Store views in the same table with VIEW# prefix on the sort key
                let view_key = format!("VIEW#{}", view.name);
                let view_json = serde_json::to_string(view).map_err(|e| {
                    FoldDbError::Serialization(format!(
                        "Failed to serialize view '{}': {}",
                        view.name, e
                    ))
                })?;
                // Reuse put_schema with view_key as schema name and view JSON as the schema
                // We store the view as a schema with a special key prefix
                let view_as_schema = Schema::new(
                    view_key.clone(),
                    crate::schema::types::schema::DeclarativeSchemaType::Single,
                    None,
                    None,
                    None,
                    None,
                );
                let mut mappers = HashMap::new();
                mappers.insert("__view_json__".to_string(), view_json);
                store.put_schema(&view_as_schema, &mappers).await?;
                log_feature!(LogFeature::Schema, info, "View '{}' persisted to DynamoDB", view.name);
            }
        }
        Ok(())
    }

    /// Load views from a sled tree
    fn load_views_from_tree(&self, views_tree: &sled::Tree) -> FoldDbResult<()> {
        let mut views = self
            .views
            .write()
            .map_err(|_| FoldDbError::Config("Failed to acquire views write lock".to_string()))?;
        views.clear();

        let mut count = 0;
        for result in views_tree.iter() {
            let (key, value) = result.map_err(|e| {
                FoldDbError::Config(format!("Failed to iterate over views tree: {}", e))
            })?;

            let name = String::from_utf8(key.to_vec()).map_err(|e| {
                FoldDbError::Config(format!("Failed to decode view name from key: {}", e))
            })?;

            let view: StoredView = serde_json::from_slice(&value).map_err(|e| {
                FoldDbError::Config(format!(
                    "Failed to parse view '{}' from database: {}",
                    name, e
                ))
            })?;

            views.insert(name, view);
            count += 1;
        }

        log_feature!(
            LogFeature::Schema,
            info,
            "Schema service loaded {} views from sled",
            count
        );

        Ok(())
    }

    /// Load views from storage (async, works for both backends)
    #[allow(unused_variables)]
    pub async fn load_views(&self) -> FoldDbResult<()> {
        match &self.storage {
            SchemaStorage::Sled { db, .. } => {
                let views_tree = db
                    .open_tree("views")
                    .map_err(|e| FoldDbError::Config(format!("Failed to open views tree: {}", e)))?;
                self.load_views_from_tree(&views_tree)?;
            }
            #[cfg(feature = "aws-backend")]
            SchemaStorage::Cloud { store } => {
                // Load views from DynamoDB: they're stored with VIEW# prefix
                let all_schemas = store.get_all_schemas().await?;
                let mut views = self.views.write().map_err(|_| {
                    FoldDbError::Config("Failed to acquire views write lock".to_string())
                })?;
                views.clear();

                for schema in all_schemas {
                    if schema.name.starts_with("VIEW#") {
                        // This is a view entry; extract the view JSON from mutation_mappers
                        // We need to re-fetch with mappers to get the view JSON
                        // For now, try to get the schema with mappers
                        if let Ok(Some(raw_schema)) = store.get_schema(&schema.name).await {
                            // The view JSON was stored in field_descriptions as a workaround
                            // Actually, we stored it in mutation_mappers with key __view_json__
                            // We need a different approach for cloud storage
                            log_feature!(
                                LogFeature::Schema,
                                warn,
                                "Cloud view loading: found VIEW# entry '{}', but direct view deserialization not yet supported",
                                raw_schema.name
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
