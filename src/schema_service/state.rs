use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use crate::db_operations::native_index::{cosine_similarity, Embedder, FastEmbedModel};
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
    SimilarSchemasResponse, StoredView, TransformRecord, ViewAddOutcome,
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
    /// Registered transforms: sha256_hash -> TransformRecord (no wasm_bytes)
    pub transforms: Arc<RwLock<HashMap<String, TransformRecord>>>,
    /// Pre-computed embeddings for reference collection names (anchor set).
    /// Used to validate that incoming descriptive_names are proper collection names
    /// rather than AI-generated captions/descriptions.
    pub collection_name_anchors: Vec<Vec<f32>>,
}

/// Reference collection names used as anchor points in embedding space.
/// Real collection names cluster near these; AI captions are far from all of them.
const COLLECTION_NAME_ANCHORS: &[&str] = &[
    "Photo Collection",
    "Recipe Collection",
    "Medical Records",
    "Journal Entries",
    "Financial Transactions",
    "Product Catalog",
    "User Profiles",
    "Event Schedule",
    "Document Collection",
    "Contact Directory",
    "Task List",
    "Insurance Records",
    "Tax Documents",
    "Course Materials",
    "Meeting Notes",
    "Travel Itinerary",
    "Order History",
    "Sales Records",
    "Music Library",
    "Email Archive",
    "Inventory List",
    "Workout Log",
    "Customer Database",
    "Blog Posts",
];

/// Minimum cosine similarity to any anchor for a descriptive_name to be accepted.
const COLLECTION_NAME_SIMILARITY_THRESHOLD: f32 = 0.3;

impl SchemaServiceState {
    /// Compute embeddings for the reference collection name anchors.
    /// Returns an empty vec if the embedding model is unavailable.
    fn compute_anchor_embeddings(embedder: &dyn Embedder) -> Vec<Vec<f32>> {
        let mut anchors = Vec::with_capacity(COLLECTION_NAME_ANCHORS.len());
        for name in COLLECTION_NAME_ANCHORS {
            match embedder.embed_text(name) {
                Ok(vec) => anchors.push(vec),
                Err(e) => {
                    log_feature!(
                        LogFeature::Schema,
                        warn,
                        "Failed to embed anchor '{}': {} — falling back to heuristic validation",
                        name,
                        e
                    );
                    return Vec::new();
                }
            }
        }
        anchors
    }

    /// Check whether a descriptive_name looks like a proper collection name
    /// rather than an AI-generated caption or description.
    ///
    /// Applies fast heuristic pre-filters (word count, sentence patterns) first,
    /// then uses embedding similarity against reference collection names when the
    /// embedding model is available.
    pub fn is_valid_collection_name(&self, name: &str) -> bool {
        // Fast pre-filter: names with more than 8 words are almost certainly captions
        let word_count = name.split_whitespace().count();
        if word_count > 8 {
            return false;
        }

        // Fast pre-filter: reject names that start with common sentence/caption patterns.
        // These are dead giveaways of AI-generated descriptions regardless of embedding similarity.
        let dn_lower = name.to_lowercase();
        if dn_lower.starts_with("this is ")
            || dn_lower.starts_with("the image ")
            || dn_lower.starts_with("this image ")
            || dn_lower.starts_with("- **")
            || dn_lower.starts_with("- this")
            || dn_lower.starts_with("a close-up ")
            || dn_lower.starts_with("a photo ")
            || dn_lower.starts_with("an image ")
        {
            return false;
        }

        // If anchors are empty (embedding model unavailable), use heuristic fallback
        if self.collection_name_anchors.is_empty() {
            return Self::heuristic_collection_name_check(name);
        }

        // Compute embedding for the candidate name
        let candidate_embedding = match self.embedder.embed_text(name) {
            Ok(vec) => vec,
            Err(_) => return Self::heuristic_collection_name_check(name),
        };

        // Find max cosine similarity to any anchor
        let max_sim = self
            .collection_name_anchors
            .iter()
            .map(|anchor| cosine_similarity(&candidate_embedding, anchor))
            .fold(f32::NEG_INFINITY, f32::max);

        max_sim >= COLLECTION_NAME_SIMILARITY_THRESHOLD
    }

    /// Heuristic fallback when embedding model is unavailable.
    /// Rejects names that look like AI captions based on word count and common patterns.
    fn heuristic_collection_name_check(name: &str) -> bool {
        let dn_lower = name.to_lowercase();
        let is_caption = dn_lower.starts_with("this is ")
            || dn_lower.starts_with("the image ")
            || dn_lower.starts_with("this image ")
            || dn_lower.starts_with("- **")
            || dn_lower.starts_with("- this")
            || name.len() > 80;
        !is_caption
    }

    /// Detect AI-generated captions/descriptions masquerading as names.
    ///
    /// Returns `true` for sentence-like names (> 8 words, or starting with
    /// common caption patterns like "This is", "A photo of", etc.).
    fn is_caption_name(name: &str) -> bool {
        let word_count = name.split_whitespace().count();
        if word_count > 8 {
            return true;
        }
        let lower = name.to_lowercase();
        lower.starts_with("this is ")
            || lower.starts_with("the image ")
            || lower.starts_with("this image ")
            || lower.starts_with("- **")
            || lower.starts_with("- this")
            || lower.starts_with("a close-up ")
            || lower.starts_with("a photo ")
            || lower.starts_with("an image ")
    }

    /// Convert a snake_case name to Title Case (e.g. "technical_notes" → "Technical Notes").
    fn snake_to_title_case(name: &str) -> String {
        name.replace('_', " ")
            .split_whitespace()
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().to_string() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Generate a proper collection name from a schema's fields and field_descriptions.
    ///
    /// 1. Concatenates field names and descriptions into a single text
    /// 2. Embeds the text and compares against anchor collection names
    /// 3. If max similarity > 0.25, uses the best-matching anchor name
    /// 4. Otherwise, infers a name from field name patterns or falls back to the schema name
    pub fn generate_collection_name(&self, schema: &Schema) -> String {
        // Build text from fields + descriptions
        let field_text = Self::build_field_text(schema);

        // Try embedding-based matching against anchors
        if !self.collection_name_anchors.is_empty() && !field_text.is_empty() {
            if let Ok(embedding) = self.embedder.embed_text(&field_text) {
                let mut best_sim = f32::NEG_INFINITY;
                let mut best_idx = 0;
                for (i, anchor) in self.collection_name_anchors.iter().enumerate() {
                    let sim = cosine_similarity(&embedding, anchor);
                    if sim > best_sim {
                        best_sim = sim;
                        best_idx = i;
                    }
                }
                if best_sim > 0.25 {
                    return COLLECTION_NAME_ANCHORS[best_idx].to_string();
                }
            }
        }

        // Fallback: infer from field name patterns
        Self::infer_name_from_fields(schema)
    }

    /// Build a text string from schema fields and their descriptions for embedding.
    fn build_field_text(schema: &Schema) -> String {
        let fields = match schema.fields.as_ref() {
            Some(f) if !f.is_empty() => f,
            _ => return String::new(),
        };

        fields
            .iter()
            .map(|f| {
                if let Some(desc) = schema.field_descriptions.get(f) {
                    format!("{}: {}", f, desc)
                } else {
                    f.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Infer a collection name from field name patterns when embedding matching fails.
    fn infer_name_from_fields(schema: &Schema) -> String {
        if let Some(ref fields) = schema.fields {
            let all_lower: Vec<String> = fields.iter().map(|f| f.to_lowercase()).collect();
            let joined = all_lower.join(" ");

            if joined.contains("gps") || joined.contains("camera") || joined.contains("focal") {
                return "Photography".to_string();
            }
            if joined.contains("amount") || joined.contains("balance") || joined.contains("transaction") {
                return "Financial Records".to_string();
            }
            if joined.contains("title") && joined.contains("content") && joined.contains("author") {
                return "Written Works".to_string();
            }
        }

        // Use schema name if it looks like a word (not a hash)
        let name = &schema.name;
        if !name.is_empty() && name.len() < 40 && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ' ' || c == '-') && name.chars().any(|c| c.is_alphabetic()) {
            // Don't use it if it looks like a hex hash
            if !(name.len() > 16 && name.chars().all(|c| c.is_ascii_hexdigit() || c == '_')) {
                return name.clone();
            }
        }

        "Data Records".to_string()
    }

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

        let transform_metadata_tree = db
            .open_tree("transform_metadata")
            .map_err(|e| FoldDbError::Config(format!("Failed to open transform_metadata tree: {}", e)))?;

        let embedder: Arc<dyn Embedder> = Arc::new(FastEmbedModel::new());
        let collection_name_anchors = Self::compute_anchor_embeddings(embedder.as_ref());

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
            transforms: Arc::new(RwLock::new(HashMap::new())),
            collection_name_anchors,
        };

        // Load schemas synchronously for sled
        state.load_schemas_sync()?;
        state.rebuild_descriptive_name_index();
        state.load_canonical_fields_from_tree(&canonical_fields_tree)?;
        state.load_views_from_tree(&views_tree)?;
        state.load_transforms_from_tree(&transform_metadata_tree)?;

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

        let embedder: Arc<dyn Embedder> = Arc::new(FastEmbedModel::new());
        let collection_name_anchors = Self::compute_anchor_embeddings(embedder.as_ref());

        let state = Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
            descriptive_name_index: Arc::new(RwLock::new(HashMap::new())),
            descriptive_name_embeddings: Arc::new(RwLock::new(HashMap::new())),
            field_embeddings: Arc::new(RwLock::new(HashMap::new())),
            canonical_fields: Arc::new(RwLock::new(HashMap::new())),
            canonical_field_embeddings: Arc::new(RwLock::new(HashMap::new())),
            embedder,
            storage: SchemaStorage::Cloud {
                store: Arc::new(store),
            },
            views: Arc::new(RwLock::new(HashMap::new())),
            transforms: Arc::new(RwLock::new(HashMap::new())),
            collection_name_anchors,
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
            // Skip superseded schemas — only the active expanded version
            // should be in the index.
            if schema.superseded_by.is_some() {
                continue;
            }
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
        let transform_metadata_tree = db
            .open_tree("transform_metadata")
            .map_err(|e| FoldDbError::Config(format!("Failed to open transform_metadata tree: {}", e)))?;

        let collection_name_anchors = Self::compute_anchor_embeddings(embedder.as_ref());

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
            transforms: Arc::new(RwLock::new(HashMap::new())),
            collection_name_anchors,
        };

        state.load_schemas_sync()?;
        state.rebuild_descriptive_name_index();
        state.load_canonical_fields_from_tree(&canonical_fields_tree)?;
        state.load_views_from_tree(&views_tree)?;
        state.load_transforms_from_tree(&transform_metadata_tree)?;

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

        // Auto-correct bad descriptive names:
        // 1. AI captions ("A photo of a sunset") → use schema.name title-cased
        // 2. Generic structural names ("Document Collection") → use schema.name title-cased
        // The ingestion layer already rejects these with retries, so this is a
        // last-resort safety net that auto-corrects rather than failing.
        if let Some(ref dn) = schema.descriptive_name.clone() {
            if Self::is_caption_name(dn) || super::name_validator::is_generic_name(dn) {
                let title_cased = Self::snake_to_title_case(&schema.name);
                log_feature!(
                    LogFeature::Schema,
                    warn,
                    "Auto-corrected descriptive_name from '{}' to '{}' (caption or generic)",
                    &dn[..dn.len().min(60)],
                    title_cased
                );
                schema.descriptive_name = Some(title_cased);
            }
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

        // Field-overlap fallback: catch near-duplicate schemas whose descriptive names
        // differ but whose fields are largely the same (Jaccard >= 0.6 AND name similarity >= 0.8).
        let overlap_target = {
            let schemas = self.schemas.read().map_err(|_| {
                FoldDbError::Config("Failed to acquire schemas read lock".to_string())
            })?;
            let incoming_fields: std::collections::HashSet<String> = schema
                .fields
                .as_ref()
                .map(|f| f.iter().cloned().collect())
                .unwrap_or_default();

            let mut best: Option<(String, Schema, f64)> = None;
            for (existing_name, existing_schema) in schemas.iter() {
                if existing_schema.superseded_by.is_some() {
                    continue;
                }
                let existing_fields: std::collections::HashSet<String> = existing_schema
                    .fields
                    .as_ref()
                    .map(|f| f.iter().cloned().collect())
                    .unwrap_or_default();
                let jaccard = super::state_matching::jaccard_index(&incoming_fields, &existing_fields);
                if jaccard >= 0.6 {
                    if let (Some(ref inc_desc), Some(ref ext_desc)) =
                        (&schema.descriptive_name, &existing_schema.descriptive_name)
                    {
                        if let (Ok(inc_emb), Ok(ext_emb)) = (
                            self.embedder.embed_text(inc_desc),
                            self.embedder.embed_text(ext_desc),
                        ) {
                            let name_sim = cosine_similarity(&inc_emb, &ext_emb);
                            if name_sim >= 0.8
                                && best.as_ref().is_none_or(|(_, _, j)| jaccard > *j)
                            {
                                best = Some((existing_name.clone(), existing_schema.clone(), jaccard));
                            }
                        }
                    }
                }
            }
            best
        }; // read lock dropped here

        if let Some((target_name, existing, jaccard)) = overlap_target {
            let desc_name = existing
                .descriptive_name
                .clone()
                .unwrap_or_else(|| schema.descriptive_name.clone().unwrap_or_default());
            schema.descriptive_name = Some(desc_name.clone());

            log_feature!(
                LogFeature::Schema,
                info,
                "Field-overlap expansion: Jaccard={:.2}, merging into '{}'",
                jaccard,
                desc_name
            );

            let incoming_fields_vec = schema.fields.clone().unwrap_or_default();
            let existing_fields_vec = existing.fields.clone().unwrap_or_default();
            let rename_map = self.semantic_field_rename_map(
                &incoming_fields_vec,
                &existing_fields_vec,
                &desc_name,
                &schema.field_descriptions,
                &existing.field_descriptions,
            );
            Self::apply_field_renames(&mut schema, &rename_map, &mut mutation_mappers);
            schema.dedup_fields();

            return self
                .expand_schema(
                    &mut schema,
                    &existing,
                    &target_name,
                    &desc_name,
                    &mutation_mappers,
                )
                .await;
        }

        schema.name = schema_name.clone();

        // Final guard: re-check descriptive_name_index under write lock to prevent
        // race conditions where two concurrent add_schema calls with the same
        // descriptive_name both pass the read-only check and create duplicates.
        // Final guard: snapshot the descriptive_name_index to detect if a concurrent
        // add_schema call already registered this descriptive_name.
        let race_expansion_target = if let Some(ref desc_name_owned) = schema.descriptive_name {
            let index = self.descriptive_name_index.read().map_err(|_| {
                FoldDbError::Config("Failed to acquire descriptive_name_index read lock".to_string())
            })?;
            let target = index.get(desc_name_owned).cloned();
            drop(index); // release lock before any await
            target
        } else {
            None
        };

        if let Some(existing_hash) = race_expansion_target {
            let existing = {
                let schemas = self.schemas.read().map_err(|_| {
                    FoldDbError::Config("Failed to acquire schemas read lock".to_string())
                })?;
                schemas.get(&existing_hash).cloned()
            };
            if let Some(existing) = existing {
                if existing.superseded_by.is_none() {
                    let desc_name_owned = schema.descriptive_name.clone().unwrap_or_default();
                    log_feature!(
                        LogFeature::Schema,
                        warn,
                        "Race condition: descriptive_name '{}' already registered as '{}' — redirecting to expansion",
                        desc_name_owned,
                        existing_hash
                    );
                    return self
                        .expand_schema(
                            &mut schema,
                            &existing,
                            &existing_hash,
                            &desc_name_owned,
                            &mutation_mappers,
                        )
                        .await;
                }
            }
        }

        // Persist to storage backend
        self.persist_schema(&schema, &mutation_mappers).await?;

        // Insert into in-memory cache and update descriptive_name index atomically
        // to prevent a window where the schema exists but isn't indexed.
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

        // Build an output schema from the view's fields and run it through add_schema.
        // Use descriptive_name as the initial schema name — add_schema replaces it with
        // the identity hash, but having a meaningful name prevents infer_name_from_fields
        // from falling back to the view name (which is not a collection name).
        let mut output_schema = Schema::new(
            request.descriptive_name.clone(),
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
            transform_hash: None,
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
                // Load views from DynamoDB: they're stored with VIEW# prefix.
                // Collect all async work first, then acquire the write lock to avoid
                // holding a !Send RwLockWriteGuard across .await points.
                let all_schemas = store.get_all_schemas().await?;

                for schema in &all_schemas {
                    if schema.name.starts_with("VIEW#") {
                        if let Ok(Some(raw_schema)) = store.get_schema(&schema.name).await {
                            log_feature!(
                                LogFeature::Schema,
                                warn,
                                "Cloud view loading: found VIEW# entry '{}', but direct view deserialization not yet supported",
                                raw_schema.name
                            );
                        }
                    }
                }

                let mut views = self.views.write().map_err(|_| {
                    FoldDbError::Config("Failed to acquire views write lock".to_string())
                })?;
                views.clear();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Create a SchemaServiceState with the real FastEmbedModel for testing.
    /// Returns None if the embedding model is unavailable (e.g., in CI).
    fn create_test_state() -> Option<SchemaServiceState> {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let db_path = tmp.path().join("test_schema_db");
        let embedder: Arc<dyn Embedder> = Arc::new(FastEmbedModel::new());

        // Test if the model works by embedding a simple string
        if embedder.embed_text("test").is_err() {
            return None;
        }

        let state = SchemaServiceState::new_with_embedder(
            db_path.to_string_lossy().to_string(),
            embedder,
        )
        .expect("failed to create state");

        // Leak the TempDir so the sled DB stays alive for the duration of the test
        std::mem::forget(tmp);
        Some(state)
    }

    /// Create a SchemaServiceState with the MockEmbeddingModel (heuristic fallback).
    /// Clears anchor embeddings so `is_valid_collection_name` falls back to the
    /// heuristic check (mock embeddings lack semantic meaning).
    fn create_mock_state() -> SchemaServiceState {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let db_path = tmp.path().join("test_schema_db_mock");
        let embedder: Arc<dyn Embedder> = Arc::new(crate::db_operations::native_index::MockEmbeddingModel);
        let mut state = SchemaServiceState::new_with_embedder(
            db_path.to_string_lossy().to_string(),
            embedder,
        )
        .expect("failed to create state");
        state.collection_name_anchors.clear();
        std::mem::forget(tmp);
        state
    }

    // ---- Good names: should ALL pass validation ----

    #[test]
    fn test_valid_name_photography() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(state.is_valid_collection_name("Photography"));
    }

    #[test]
    fn test_valid_name_medical_records() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(state.is_valid_collection_name("Medical Records"));
    }

    #[test]
    fn test_valid_name_recipe_book() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(state.is_valid_collection_name("Recipe Book"));
    }

    #[test]
    fn test_valid_name_travel_photography() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(state.is_valid_collection_name("Travel Photography"));
    }

    #[test]
    fn test_valid_name_work_documents() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(state.is_valid_collection_name("Work Documents"));
    }

    #[test]
    fn test_valid_name_family_album() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(state.is_valid_collection_name("Family Album"));
    }

    #[test]
    fn test_valid_name_expense_reports() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(state.is_valid_collection_name("Expense Reports"));
    }

    #[test]
    fn test_valid_name_customer_orders() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(state.is_valid_collection_name("Customer Orders"));
    }

    #[test]
    fn test_valid_name_blog_posts() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(state.is_valid_collection_name("Blog Posts"));
    }

    #[test]
    fn test_valid_name_landscape_paintings() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(state.is_valid_collection_name("Landscape Paintings"));
    }

    // ---- Bad names: should ALL fail validation (still detected as invalid) ----

    #[test]
    fn test_invalid_name_vision_caption_1() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(!state.is_valid_collection_name(
            "This image depicts a scenic outdoor scene on a boat"
        ));
    }

    #[test]
    fn test_invalid_name_vision_caption_2() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(!state.is_valid_collection_name(
            "The image depicts a woman seated at a restaurant table with wine"
        ));
    }

    #[test]
    fn test_invalid_name_description_with_markdown() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(!state.is_valid_collection_name(
            "- **Description of the Image**: A pastoral landscape"
        ));
    }

    #[test]
    fn test_invalid_name_long_sentence() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(!state.is_valid_collection_name(
            "A close-up photograph of a red fox in a snowy forest setting"
        ));
    }

    #[test]
    fn test_invalid_name_this_is_pattern() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(!state.is_valid_collection_name(
            "This is not a document, receipt, or screenshot"
        ));
    }

    #[test]
    fn test_invalid_name_technical_description() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(!state.is_valid_collection_name(
            "The image shows a person wearing a form-fitting deep red long-sleeved top"
        ));
    }

    // ---- Auto-correction tests ----

    /// Helper to create a photo-like schema with camera/GPS fields
    fn make_photo_schema(descriptive_name: &str) -> Schema {
        use crate::schema::types::schema::DeclarativeSchemaType;
        let mut schema = Schema::new(
            "test_photo_schema".to_string(),
            DeclarativeSchemaType::Single,
            None,
            Some(vec![
                "focal_length".to_string(),
                "camera_model".to_string(),
                "gps_latitude".to_string(),
            ]),
            None,
            None,
        );
        schema.descriptive_name = Some(descriptive_name.to_string());
        schema.field_descriptions.insert("focal_length".to_string(), "lens focal length".to_string());
        schema.field_descriptions.insert("camera_model".to_string(), "camera model".to_string());
        schema.field_descriptions.insert("gps_latitude".to_string(), "GPS latitude".to_string());
        schema
    }

    #[test]
    fn test_autocorrect_vision_caption_1() {
        // AI caption should be auto-corrected to a proper collection name
        let state = create_test_state().unwrap_or_else(create_mock_state);
        let schema = make_photo_schema("This image depicts a scenic outdoor scene on a boat");
        let generated = state.generate_collection_name(&schema);
        // Should produce something like "Photo Collection" based on image/camera/gps fields
        assert!(state.is_valid_collection_name(&generated),
            "Auto-corrected name '{}' should be a valid collection name", generated);
    }

    #[test]
    fn test_autocorrect_vision_caption_2() {
        let state = create_test_state().unwrap_or_else(create_mock_state);
        let schema = make_photo_schema("The image depicts a woman seated at a restaurant table with wine");
        let generated = state.generate_collection_name(&schema);
        assert!(state.is_valid_collection_name(&generated),
            "Auto-corrected name '{}' should be a valid collection name", generated);
    }

    #[test]
    fn test_autocorrect_preserves_valid_name() {
        // Valid names should NOT be changed by is_valid_collection_name
        let state = create_test_state().unwrap_or_else(create_mock_state);
        assert!(state.is_valid_collection_name("Photography"));
        assert!(state.is_valid_collection_name("Medical Records"));
        assert!(state.is_valid_collection_name("Recipe Book"));
    }

    #[test]
    fn test_autocorrect_photo_fields_produce_photography_name() {
        // Schemas with camera/gps fields should produce "Photography"
        // when using the field-pattern fallback
        let state = create_mock_state(); // mock state forces heuristic/field-pattern path
        let schema = make_photo_schema("anything");
        let generated = state.generate_collection_name(&schema);
        assert_eq!(generated, "Photography");
    }

    #[test]
    fn test_autocorrect_financial_fields() {
        use crate::schema::types::schema::DeclarativeSchemaType;
        let state = create_mock_state();
        let mut schema = Schema::new(
            "test_fin".to_string(),
            DeclarativeSchemaType::Single,
            None,
            Some(vec!["amount".to_string(), "description".to_string()]),
            None,
            None,
        );
        schema.descriptive_name = Some("some caption".to_string());
        schema.field_descriptions.insert("amount".to_string(), "transaction amount".to_string());
        schema.field_descriptions.insert("description".to_string(), "transaction description".to_string());
        let generated = state.generate_collection_name(&schema);
        assert_eq!(generated, "Financial Records");
    }

    #[test]
    fn test_autocorrect_document_fields() {
        use crate::schema::types::schema::DeclarativeSchemaType;
        let state = create_mock_state();
        let mut schema = Schema::new(
            "test_doc".to_string(),
            DeclarativeSchemaType::Single,
            None,
            Some(vec!["title".to_string(), "content".to_string(), "author".to_string()]),
            None,
            None,
        );
        schema.descriptive_name = Some("some caption".to_string());
        schema.field_descriptions.insert("title".to_string(), "document title".to_string());
        schema.field_descriptions.insert("content".to_string(), "document content".to_string());
        schema.field_descriptions.insert("author".to_string(), "document author".to_string());
        let generated = state.generate_collection_name(&schema);
        assert_eq!(generated, "Written Works");
    }

    #[test]
    fn test_autocorrect_fallback_to_schema_name() {
        use crate::schema::types::schema::DeclarativeSchemaType;
        let state = create_mock_state();
        let mut schema = Schema::new(
            "my_recipes".to_string(),
            DeclarativeSchemaType::Single,
            None,
            Some(vec!["ingredient".to_string(), "step".to_string()]),
            None,
            None,
        );
        schema.descriptive_name = Some("some caption".to_string());
        schema.field_descriptions.insert("ingredient".to_string(), "recipe ingredient".to_string());
        schema.field_descriptions.insert("step".to_string(), "cooking step".to_string());
        let generated = state.generate_collection_name(&schema);
        // No field pattern matches, should fall back to schema name
        assert_eq!(generated, "my_recipes");
    }

    #[test]
    fn test_autocorrect_fallback_data_records_for_hash_name() {
        use crate::schema::types::schema::DeclarativeSchemaType;
        let state = create_mock_state();
        let mut schema = Schema::new(
            "abcdef1234567890abcdef".to_string(),
            DeclarativeSchemaType::Single,
            None,
            Some(vec!["foo".to_string(), "bar".to_string()]),
            None,
            None,
        );
        schema.descriptive_name = Some("some caption".to_string());
        schema.field_descriptions.insert("foo".to_string(), "a foo".to_string());
        schema.field_descriptions.insert("bar".to_string(), "a bar".to_string());
        let generated = state.generate_collection_name(&schema);
        assert_eq!(generated, "Data Records");
    }

    #[test]
    fn test_autocorrect_enables_schema_expansion() {
        // Two photo schemas with different AI captions should both generate
        // the same collection name, enabling schema expansion/merging
        let state = create_mock_state();
        let schema1 = make_photo_schema("This image depicts a scenic outdoor scene on a boat");
        let schema2 = make_photo_schema("The image depicts a woman seated at a restaurant table with wine");
        let name1 = state.generate_collection_name(&schema1);
        let name2 = state.generate_collection_name(&schema2);
        assert_eq!(name1, name2,
            "Both photo schemas should generate the same collection name for merging: '{}' vs '{}'",
            name1, name2);
        assert_eq!(name1, "Photography");
    }

    // ---- Heuristic fallback tests (using mock embedder) ----

    #[test]
    fn test_heuristic_rejects_this_is_prefix() {
        assert!(!SchemaServiceState::heuristic_collection_name_check(
            "This is a description of something"
        ));
    }

    #[test]
    fn test_heuristic_rejects_long_names() {
        let long_name = "A".repeat(81);
        assert!(!SchemaServiceState::heuristic_collection_name_check(&long_name));
    }

    #[test]
    fn test_heuristic_accepts_short_collection_name() {
        assert!(SchemaServiceState::heuristic_collection_name_check("Photography"));
    }

    // ---- Duplicate schema prevention tests (same descriptive_name → expansion) ----

    /// Helper to create a simple schema with given name, descriptive_name, and fields.
    /// Pre-populates field_data_classifications so classification doesn't require
    /// a live ANTHROPIC_API_KEY (the key IS in CI secrets but this keeps tests
    /// fast and deterministic).
    fn make_schema(name: &str, descriptive_name: &str, fields: &[&str]) -> (Schema, HashMap<String, String>) {
        use crate::schema::types::schema::DeclarativeSchemaType;
        use crate::schema::types::data_classification::DataClassification;
        let field_vec: Vec<String> = fields.iter().map(|f| f.to_string()).collect();
        let mut schema = Schema::new(
            name.to_string(),
            DeclarativeSchemaType::Single,
            None,
            Some(field_vec),
            None,
            None,
        );
        schema.descriptive_name = Some(descriptive_name.to_string());
        for f in fields {
            schema.field_descriptions.insert(f.to_string(), format!("{} description", f));
            schema.field_data_classifications.insert(
                f.to_string(),
                DataClassification::new(1, "general").expect("valid classification"),
            );
        }
        let mappers = HashMap::new();
        (schema, mappers)
    }

    #[tokio::test]
    async fn test_same_descriptive_name_different_fields_expands() {
        let state = create_mock_state();

        // Register first schema
        let (schema1, mappers1) = make_schema(
            "nature_shots",
            "Nature Photography",
            &["camera_model", "gps_latitude", "focal_length"],
        );
        let result1 = state.add_schema(schema1, mappers1).await;
        assert!(result1.is_ok(), "First schema should succeed: {:?}", result1);
        let outcome1 = result1.unwrap();
        assert!(
            matches!(outcome1, SchemaAddOutcome::Added(_, _)),
            "First schema should be Added, got: {:?}",
            std::mem::discriminant(&outcome1)
        );

        // Register second schema with SAME descriptive_name but different fields
        let (schema2, mappers2) = make_schema(
            "nature_shots",
            "Nature Photography",
            &["camera_model", "gps_latitude", "focal_length", "shutter_speed", "iso"],
        );
        let result2 = state.add_schema(schema2, mappers2).await;
        assert!(result2.is_ok(), "Second schema should succeed: {:?}", result2);
        let outcome2 = result2.unwrap();

        // Must be Expanded or AlreadyExists — NOT Added (which would create a duplicate)
        assert!(
            !matches!(outcome2, SchemaAddOutcome::Added(_, _)),
            "Second schema with same descriptive_name must NOT create a duplicate — should expand or reuse, got Added"
        );
    }

    #[tokio::test]
    async fn test_same_descriptive_name_same_fields_reuses() {
        let state = create_mock_state();

        let (schema1, mappers1) = make_schema(
            "nature_shots",
            "Nature Photography",
            &["camera_model", "gps_latitude"],
        );
        let result1 = state.add_schema(schema1, mappers1).await.unwrap();
        assert!(matches!(result1, SchemaAddOutcome::Added(_, _)));

        // Same descriptive_name and same fields → should return AlreadyExists
        let (schema2, mappers2) = make_schema(
            "nature_shots",
            "Nature Photography",
            &["camera_model", "gps_latitude"],
        );
        let result2 = state.add_schema(schema2, mappers2).await.unwrap();
        assert!(
            matches!(result2, SchemaAddOutcome::AlreadyExists(_, _)),
            "Same name + same fields should reuse existing, got: {:?}",
            std::mem::discriminant(&result2)
        );
    }

    #[tokio::test]
    async fn test_same_descriptive_name_subset_fields_reuses() {
        let state = create_mock_state();

        let (schema1, mappers1) = make_schema(
            "nature_shots",
            "Nature Photography",
            &["camera_model", "gps_latitude", "focal_length"],
        );
        state.add_schema(schema1, mappers1).await.unwrap();

        // Subset of fields → should return AlreadyExists (existing is a superset)
        let (schema2, mappers2) = make_schema(
            "nature_shots",
            "Nature Photography",
            &["camera_model", "gps_latitude"],
        );
        let result2 = state.add_schema(schema2, mappers2).await.unwrap();
        assert!(
            matches!(result2, SchemaAddOutcome::AlreadyExists(_, _)),
            "Subset of existing fields should reuse existing, got: {:?}",
            std::mem::discriminant(&result2)
        );
    }

    #[tokio::test]
    async fn test_no_duplicate_schemas_after_expansion() {
        let state = create_mock_state();

        let (schema1, mappers1) = make_schema(
            "nature_shots",
            "Nature Photography",
            &["camera_model", "gps_latitude"],
        );
        state.add_schema(schema1, mappers1).await.unwrap();

        let (schema2, mappers2) = make_schema(
            "nature_shots",
            "Nature Photography",
            &["camera_model", "gps_latitude", "focal_length"],
        );
        state.add_schema(schema2, mappers2).await.unwrap();

        // Count non-superseded schemas with this descriptive_name
        let schemas = state.schemas.read().unwrap();
        let active_count = schemas
            .values()
            .filter(|s| {
                s.superseded_by.is_none()
                    && s.descriptive_name.as_deref() == Some("Nature Photography")
            })
            .count();
        assert_eq!(
            active_count, 1,
            "Should have exactly 1 active schema for 'Nature Photography', found {}",
            active_count
        );
    }
}
