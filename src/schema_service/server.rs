use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer as ActixHttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::Schema;
#[cfg(feature = "aws-backend")]
use crate::storage::DynamoDbSchemaStore;

#[cfg(feature = "aws-backend")]
pub use crate::storage::CloudConfig;

/// Response containing a list of available schema names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemasListResponse {
    pub schemas: Vec<String>,
}

/// Response containing all available schemas with their definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableSchemasResponse {
    pub schemas: Vec<Schema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSimilarityResponse {
    pub similarity: f64,
    pub closest_schema: Schema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaAddOutcome {
    Added(Schema, HashMap<String, String>), // Schema and mutation_mappers
    TooSimilar(SchemaSimilarityResponse),
}

/// Error response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Request structure for adding a schema with mutation mappers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddSchemaRequest {
    pub schema: Schema,
    pub mutation_mappers: HashMap<String, String>,
}

/// Response structure for adding a schema with mutation mappers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddSchemaResponse {
    pub schema: Schema,
    pub mutation_mappers: HashMap<String, String>,
}

/// Reload response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadResponse {
    pub success: bool,
    pub schemas_loaded: usize,
}

/// Health check response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

/// Conflict response for similar schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResponse {
    pub error: String,
    pub similarity: f64,
    pub closest_schema: Schema,
}

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
    schemas: Arc<RwLock<HashMap<String, Schema>>>,
    storage: SchemaStorage,
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

        let state = Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
            storage: SchemaStorage::Sled { db, schemas_tree },
        };

        // Load schemas synchronously for sled
        state.load_schemas_sync()?;

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
    /// No locking needed - topology hashes ensure idempotent writes!
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
            storage: SchemaStorage::Cloud {
                store: Arc::new(store),
            },
        };

        // Load schemas on initialization
        state.load_schemas().await?;

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

    pub async fn add_schema(
        &self,
        mut schema: Schema,
        mutation_mappers: HashMap<String, String>,
    ) -> FoldDbResult<SchemaAddOutcome> {
        // Validate that all fields have topologies defined
        if let Some(ref fields) = schema.fields {
            for field_name in fields {
                if !schema.field_topologies.contains_key(field_name) {
                    return Err(FoldDbError::Config(format!(
                        "Field '{}' is missing a topology definition. All fields must have a topology.",
                        field_name
                    )));
                }
            }
        }

        // Ensure topology_hash is computed
        if schema.topology_hash.is_none() {
            schema.compute_schema_topology_hash();
        }

        // Get the original schema name before we modify it
        let original_schema_name = schema.name.clone();

        // Use topology_hash as the schema identifier
        let topology_hash = schema
            .get_topology_hash()
            .ok_or_else(|| {
                FoldDbError::Config("Schema must have topology_hash computed".to_string())
            })?
            .clone();

        log_feature!(
            LogFeature::Schema,
            info,
            "Schema '{}' topology_hash: {}",
            original_schema_name,
            topology_hash
        );

        // Use topology_hash as unique identifier (already includes field names)
        let schema_name = topology_hash.clone();

        // Check if this exact combination already exists
        {
            let schemas = self.schemas.read().map_err(|_| {
                FoldDbError::Config("Failed to acquire schemas read lock".to_string())
            })?;

            if schemas.contains_key(&schema_name) {
                let existing_schema = schemas.get(&schema_name).unwrap();
                log_feature!(
                    LogFeature::Schema,
                    info,
                    "Schema '{}' already exists - using existing schema",
                    schema_name
                );

                return Ok(SchemaAddOutcome::TooSimilar(SchemaSimilarityResponse {
                    similarity: 1.0,
                    closest_schema: existing_schema.clone(),
                }));
            }
        }

        schema.name = schema_name.clone();

        // Persist to storage backend
        match &self.storage {
            SchemaStorage::Sled { db, schemas_tree } => {
                let serialized_schema = serde_json::to_vec(&schema).map_err(|error| {
                    FoldDbError::Serialization(format!(
                        "Failed to serialize schema '{}': {}",
                        schema_name, error
                    ))
                })?;

                schemas_tree
                    .insert(schema_name.as_bytes(), serialized_schema)
                    .map_err(|error| {
                        FoldDbError::Config(format!(
                            "Failed to insert schema '{}' into sled database: {}",
                            schema_name, error
                        ))
                    })?;

                db.flush().map_err(|error| {
                    FoldDbError::Config(format!("Failed to flush sled database: {}", error))
                })?;

                log_feature!(
                    LogFeature::Schema,
                    info,
                    "Schema '{}' persisted to sled database",
                    schema_name
                );
            }
            #[cfg(feature = "aws-backend")]
            SchemaStorage::Cloud { store } => {
                // No locking needed! Topology hash ensures idempotent writes
                store.put_schema(&schema, &mutation_mappers).await?;

                log_feature!(
                    LogFeature::Schema,
                    info,
                    "Schema '{}' persisted to DynamoDB (no locking needed!)",
                    schema_name
                );
            }
        }

        // Insert into in-memory cache
        {
            let mut schemas = self.schemas.write().map_err(|_| {
                FoldDbError::Config("Failed to acquire schemas write lock".to_string())
            })?;
            schemas.insert(schema_name.clone(), schema.clone());
        }

        log_feature!(
            LogFeature::Schema,
            info,
            "Schema '{}' successfully added to registry",
            schema_name
        );

        Ok(SchemaAddOutcome::Added(schema, mutation_mappers))
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
}

/// List all available schemas
async fn list_schemas(state: web::Data<SchemaServiceState>) -> impl Responder {
    let schemas = match state.schemas.read() {
        Ok(s) => s,
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to acquire schemas read lock: {}",
                e
            );
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to acquire schemas read lock".to_string(),
            });
        }
    };

    let schema_names: Vec<String> = schemas.keys().cloned().collect();

    HttpResponse::Ok().json(SchemasListResponse {
        schemas: schema_names,
    })
}

/// Get all available schemas with their full definitions
async fn get_available_schemas(state: web::Data<SchemaServiceState>) -> impl Responder {
    let schemas = match state.schemas.read() {
        Ok(s) => s,
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to acquire schemas read lock: {}",
                e
            );
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to acquire schemas read lock".to_string(),
            });
        }
    };

    let schema_list: Vec<Schema> = schemas.values().cloned().collect();

    HttpResponse::Ok().json(AvailableSchemasResponse {
        schemas: schema_list,
    })
}

/// Get a specific schema by name
async fn get_schema(
    path: web::Path<String>,
    state: web::Data<SchemaServiceState>,
) -> impl Responder {
    let schema_name = path.into_inner();
    log_feature!(
        LogFeature::Schema,
        info,
        "Schema service: getting schema '{}'",
        schema_name
    );

    let schemas = match state.schemas.read() {
        Ok(s) => s,
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to acquire schemas read lock: {}",
                e
            );
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to acquire schemas read lock".to_string(),
            });
        }
    };

    match schemas.get(&schema_name) {
        Some(schema) => HttpResponse::Ok().json(schema),
        None => {
            log_feature!(
                LogFeature::Schema,
                warn,
                "Schema '{}' not found",
                schema_name
            );
            HttpResponse::NotFound().json(ErrorResponse {
                error: "Schema not found".to_string(),
            })
        }
    }
}

/// Reload schemas from the directory
async fn reload_schemas(state: web::Data<SchemaServiceState>) -> impl Responder {
    log_feature!(
        LogFeature::Schema,
        info,
        "Schema service: reloading schemas"
    );

    match state.load_schemas().await {
        Ok(_) => {
            let schemas = match state.schemas.read() {
                Ok(s) => s,
                Err(e) => {
                    log_feature!(
                        LogFeature::Schema,
                        error,
                        "Failed to acquire schemas read lock: {}",
                        e
                    );
                    return HttpResponse::InternalServerError().json(ErrorResponse {
                        error: "Failed to acquire schemas read lock".to_string(),
                    });
                }
            };

            HttpResponse::Ok().json(ReloadResponse {
                success: true,
                schemas_loaded: schemas.len(),
            })
        }
        Err(e) => {
            log_feature!(LogFeature::Schema, error, "Failed to reload schemas: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to reload schemas: {}", e),
            })
        }
    }
}

async fn add_schema(
    payload: web::Json<AddSchemaRequest>,
    state: web::Data<SchemaServiceState>,
) -> impl Responder {
    let request = payload.into_inner();
    let schema_name = request.schema.name.clone();

    log_feature!(
        LogFeature::Schema,
        info,
        "Schema service: adding schema '{}' with {} mutation mappers",
        schema_name,
        request.mutation_mappers.len()
    );

    match state
        .add_schema(request.schema, request.mutation_mappers)
        .await
    {
        Ok(SchemaAddOutcome::Added(schema, mutation_mappers)) => {
            HttpResponse::Created().json(AddSchemaResponse {
                schema,
                mutation_mappers,
            })
        }
        Ok(SchemaAddOutcome::TooSimilar(conflict)) => {
            HttpResponse::Conflict().json(ConflictResponse {
                error: "Schema too similar to existing schema".to_string(),
                similarity: conflict.similarity,
                closest_schema: conflict.closest_schema,
            })
        }
        Err(error) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to add schema '{}': {}",
                schema_name,
                error
            );
            HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("Failed to add schema: {}", error),
            })
        }
    }
}

/// Health check endpoint
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
    })
}

/// Request for resetting the schema service database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetRequest {
    pub confirm: bool,
}

/// Response for resetting the schema service database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetResponse {
    pub success: bool,
    pub message: String,
}

/// Reset the schema service database
async fn reset_database(
    state: web::Data<SchemaServiceState>,
    req: web::Json<ResetRequest>,
) -> impl Responder {
    // Require explicit confirmation
    if !req.confirm {
        return HttpResponse::BadRequest().json(ResetResponse {
            success: false,
            message: "Reset confirmation required. Set 'confirm' to true.".to_string(),
        });
    }

    log_feature!(
        LogFeature::Schema,
        info,
        "Resetting schema service database"
    );

    // Clear the in-memory schemas map
    {
        let mut schemas = state.schemas.write().unwrap();
        schemas.clear();
    }

    // Clear storage backend
    match &state.storage {
        SchemaStorage::Sled { db, schemas_tree } => {
            // Clear all entries from the schemas tree
            if let Err(e) = schemas_tree.clear() {
                log_feature!(
                    LogFeature::Schema,
                    error,
                    "Failed to clear schemas tree: {}",
                    e
                );
                return HttpResponse::InternalServerError().json(ResetResponse {
                    success: false,
                    message: format!("Failed to reset sled database: {}", e),
                });
            }

            // Flush to ensure changes are persisted
            if let Err(e) = db.flush() {
                log_feature!(
                    LogFeature::Schema,
                    warn,
                    "Failed to flush database after reset: {}",
                    e
                );
            }
        }
        #[cfg(feature = "aws-backend")]
        SchemaStorage::Cloud { store } => {
            // Clear all schemas from DynamoDB
            if let Err(e) = store.clear_all_schemas().await {
                log_feature!(
                    LogFeature::Schema,
                    error,
                    "Failed to clear DynamoDB schemas: {}",
                    e
                );
                return HttpResponse::InternalServerError().json(ResetResponse {
                    success: false,
                    message: format!("Failed to reset DynamoDB: {}", e),
                });
            }
        }
    }

    log_feature!(
        LogFeature::Schema,
        info,
        "Schema service database reset successfully"
    );

    HttpResponse::Ok().json(ResetResponse {
        success: true,
        message: "Schema service database reset successfully. All schemas have been cleared."
            .to_string(),
    })
}

/// Schema Service HTTP Server
pub struct SchemaServiceServer {
    state: web::Data<SchemaServiceState>,
    bind_address: String,
}

impl SchemaServiceServer {
    /// Create a new schema service server with local sled storage
    pub fn new(db_path: String, bind_address: &str) -> FoldDbResult<Self> {
        let state = SchemaServiceState::new(db_path)?;

        Ok(Self {
            state: web::Data::new(state),
            bind_address: bind_address.to_string(),
        })
    }

    /// Create a new schema service server with Cloud backend
    #[cfg(feature = "aws-backend")]
    pub async fn new_with_cloud(config: CloudConfig, bind_address: &str) -> FoldDbResult<Self> {
        let state = SchemaServiceState::new_with_cloud(config).await?;

        Ok(Self {
            state: web::Data::new(state),
            bind_address: bind_address.to_string(),
        })
    }

    /// Run the schema service server
    pub async fn run(&self) -> FoldDbResult<()> {
        log_feature!(
            LogFeature::HttpServer,
            info,
            "Schema service starting on {}",
            self.bind_address
        );

        let state = self.state.clone();

        let server = ActixHttpServer::new(move || {
            let cors = Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
                .max_age(3600);

            App::new().wrap(cors).app_data(state.clone()).service(
                web::scope("/api")
                    .route("/health", web::get().to(health_check))
                    .service(
                        web::resource("/schemas")
                            .route(web::get().to(list_schemas))
                            .route(web::post().to(add_schema)),
                    )
                    .route("/schemas/reload", web::post().to(reload_schemas))
                    .route("/schemas/available", web::get().to(get_available_schemas))
                    .route("/schema/{name}", web::get().to(get_schema))
                    .route("/system/reset", web::post().to(reset_database)),
            )
        })
        .bind(&self.bind_address)
        .map_err(|e| FoldDbError::Config(format!("Failed to bind schema service: {}", e)))?
        .run();

        server
            .await
            .map_err(|e| FoldDbError::Config(format!("Schema service error: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn add_schema_adds_new_schema() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir
            .path()
            .join("test_schema_db")
            .to_string_lossy()
            .to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        let mut new_schema = Schema::new(
            "NewSchema".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "value".to_string()]),
            None,
            None,
        );

        // Add required topologies
        new_schema.set_field_topology(
            "id".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );
        new_schema.set_field_topology(
            "value".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );

        let outcome = state
            .add_schema(new_schema.clone(), HashMap::new())
            .await
            .expect("failed to add schema");

        let added_schema = match outcome {
            SchemaAddOutcome::Added(schema, _mutation_mappers) => schema,
            SchemaAddOutcome::TooSimilar(_) => panic!("schema should have been added"),
        };

        // Schema name should be the topology_hash (64 char hex string)
        assert_eq!(added_schema.name.len(), 64); // 64 char hash
        assert_eq!(&added_schema.name, new_schema.get_topology_hash().unwrap());

        // Topology should match
        assert_eq!(added_schema.field_topologies, new_schema.field_topologies);

        let stored_schemas = state
            .schemas
            .read()
            .expect("failed to acquire read lock on schema map after addition");

        // Check stored by combined name
        assert!(stored_schemas.contains_key(&added_schema.name));

        // Check the underlying storage
        // Check the underlying storage
        match &state.storage {
            SchemaStorage::Sled { schemas_tree, .. } => {
                let db_value = schemas_tree
                    .get(added_schema.name.as_bytes())
                    .expect("failed to query database")
                    .expect("schema should exist in database");

                let stored_schema: Schema =
                    serde_json::from_slice(&db_value).expect("failed to deserialize stored schema");

                assert_eq!(stored_schema.name, added_schema.name);
            }
            #[cfg(feature = "aws-backend")]
            _ => panic!("Expected Sled storage"),
        }
    }

    #[tokio::test]
    async fn add_schema_detects_similar_schema() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir
            .path()
            .join("test_schema_db")
            .to_string_lossy()
            .to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        let mut existing_schema = Schema::new(
            "Existing".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "value".to_string()]),
            None,
            None,
        );

        // Add required topologies
        existing_schema.set_field_topology(
            "id".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );
        existing_schema.set_field_topology(
            "value".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );

        let mut similar_schema = Schema::new(
            "PotentialDuplicate".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "value".to_string()]),
            None,
            None,
        );

        // Add required topologies
        similar_schema.set_field_topology(
            "id".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );
        similar_schema.set_field_topology(
            "value".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );

        // First schema gets added
        let outcome1 = state
            .add_schema(existing_schema.clone(), HashMap::new())
            .await
            .expect("failed to add existing schema");

        let existing_name = match outcome1 {
            SchemaAddOutcome::Added(schema, _) => {
                // Should be topology_hash (64 char hex string)
                assert_eq!(schema.name.len(), 64);
                schema.name
            }
            SchemaAddOutcome::TooSimilar(_) => panic!("first schema should be added"),
        };

        // Second schema with SAME topology and SAME name should be detected as exact duplicate
        let outcome2 = state
            .add_schema(similar_schema.clone(), HashMap::new())
            .await
            .expect("failed to evaluate schema similarity");

        match outcome2 {
            SchemaAddOutcome::TooSimilar(conflict) => {
                assert_eq!(conflict.similarity, 1.0); // Exact duplicate
                assert_eq!(conflict.closest_schema.name, existing_name);
                assert_eq!(
                    conflict.closest_schema.field_topologies,
                    existing_schema.field_topologies
                );
            }
            SchemaAddOutcome::Added(_, _) => {
                panic!("schema with same name and topology should be rejected as duplicate")
            }
        }
    }

    #[tokio::test]
    async fn add_schema_with_different_topology_creates_separate_schema() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir
            .path()
            .join("test_schema_db")
            .to_string_lossy()
            .to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        // First schema: 2 fields
        let mut schema1 = Schema::new(
            "UserBasic".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "name".to_string()]),
            None,
            None,
        );

        schema1.set_field_topology(
            "id".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );
        schema1.set_field_topology(
            "name".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );

        let outcome1 = state
            .add_schema(schema1.clone(), HashMap::new())
            .await
            .expect("failed to add first schema");

        let schema1_name = match outcome1 {
            SchemaAddOutcome::Added(schema, _) => schema.name,
            other => panic!("expected schema addition, got {:?}", other),
        };

        // Second schema: 3 fields (different topology!)
        let mut schema2 = Schema::new(
            "UserExtended".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec![
                "id".to_string(),
                "name".to_string(),
                "email".to_string(),
            ]),
            None,
            None,
        );

        schema2.set_field_topology(
            "id".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );
        schema2.set_field_topology(
            "name".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );
        schema2.set_field_topology(
            "email".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );

        let outcome2 = state
            .add_schema(schema2.clone(), HashMap::new())
            .await
            .expect("failed to add second schema");

        let schema2_name = match outcome2 {
            SchemaAddOutcome::Added(schema, _) => schema.name,
            other => panic!("expected schema addition, got {:?}", other),
        };

        // Should be topology hashes (64 char hex strings)
        assert_eq!(schema1_name.len(), 64);
        assert_eq!(schema2_name.len(), 64);

        // Different topologies should produce different names
        assert_ne!(schema1_name, schema2_name);
    }

    #[tokio::test]
    async fn add_schema_rejects_missing_topology() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir
            .path()
            .join("test_schema_db")
            .to_string_lossy()
            .to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        // Schema without topology
        let invalid_schema = Schema::new(
            "TestSchema".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string()]),
            None,
            None,
        );

        let error = state
            .add_schema(invalid_schema, HashMap::new())
            .await
            .expect_err("schema without topology should be rejected");

        match error {
            FoldDbError::Config(message) => {
                assert!(message.contains("missing a topology definition"));
            }
            other => panic!("expected config error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn get_available_schemas_returns_all_schemas() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir
            .path()
            .join("test_schema_db")
            .to_string_lossy()
            .to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        let mut schema1 = Schema::new(
            "UserSchema".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec![
                "user_id".to_string(),
                "username".to_string(),
                "email".to_string(),
            ]),
            None,
            None,
        );

        // Add required topologies for schema1
        schema1.set_field_topology(
            "user_id".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );
        schema1.set_field_topology(
            "username".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );
        schema1.set_field_topology(
            "email".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );

        let mut schema2 = Schema::new(
            "ProductSchema".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec![
                "product_id".to_string(),
                "title".to_string(),
                "price".to_string(),
                "description".to_string(),
            ]),
            None,
            None,
        );

        // Add required topologies for schema2
        schema2.set_field_topology(
            "product_id".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );
        schema2.set_field_topology(
            "title".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );
        schema2.set_field_topology(
            "price".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::Number,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );
        schema2.set_field_topology(
            "description".to_string(),
            crate::schema::types::JsonTopology::new(
                crate::schema::types::TopologyNode::Primitive {
                    value: crate::schema::types::PrimitiveType::String,
                    classifications: Some(vec!["word".to_string()]),
                },
            ),
        );

        let outcome1 = state
            .add_schema(schema1.clone(), HashMap::new())
            .await
            .expect("failed to add schema1");
        let schema1_name = match outcome1 {
            SchemaAddOutcome::Added(s, _) => s.name,
            _ => panic!("schema1 should be added"),
        };

        let outcome2 = state
            .add_schema(schema2.clone(), HashMap::new())
            .await
            .expect("failed to add schema2");
        let schema2_name = match outcome2 {
            SchemaAddOutcome::Added(s, _) => s.name,
            _ => panic!("schema2 should be added"),
        };

        let schemas = state
            .schemas
            .read()
            .expect("failed to acquire read lock on schemas");
        assert_eq!(schemas.len(), 2);

        // Schemas are now stored by topology_hash
        assert!(schemas.contains_key(&schema1_name));
        assert!(schemas.contains_key(&schema2_name));

        // Different topologies should produce different names
        assert_ne!(schema1_name, schema2_name);
    }
}
