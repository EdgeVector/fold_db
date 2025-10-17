use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer as ActixHttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use strsim::normalized_levenshtein;

use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::Schema;

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

/// Shared state for the schema service
#[derive(Clone)]
pub struct SchemaServiceState {
    schemas: Arc<Mutex<HashMap<String, Schema>>>,
    db: sled::Db,
    schemas_tree: sled::Tree,
}

const SCHEMA_SIMILARITY_THRESHOLD: f64 = 0.9;
const FIELD_OVERLAP_THRESHOLD: f64 = 0.6;

impl SchemaServiceState {
    pub fn new(db_path: String) -> FoldDbResult<Self> {
        let db = sled::open(&db_path).map_err(|e| {
            FoldDbError::Config(format!("Failed to open schema service database at '{}': {}", db_path, e))
        })?;

        let schemas_tree = db.open_tree("schemas").map_err(|e| {
            FoldDbError::Config(format!("Failed to open schemas tree: {}", e))
        })?;

        let state = Self {
            schemas: Arc::new(Mutex::new(HashMap::new())),
            db,
            schemas_tree,
        };

        // Load schemas on initialization
        state.load_schemas()?;

        Ok(state)
    }

    /// Load all schemas from the sled database
    pub fn load_schemas(&self) -> FoldDbResult<()> {
        let mut schemas = self
            .schemas
            .lock()
            .map_err(|_| FoldDbError::Config("Failed to acquire schemas lock".to_string()))?;

        schemas.clear();

        let mut count = 0;
        for result in self.schemas_tree.iter() {
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
                "Loaded schema '{}' from database",
                name
            );

            schemas.insert(name, schema);
            count += 1;
        }

        log_feature!(
            LogFeature::Schema,
            info,
            "Schema service loaded {} schemas from database",
            count
        );

        Ok(())
    }

    pub fn add_schema(&self, mut schema: Schema, mut mutation_mappers: HashMap<String, String>) -> FoldDbResult<SchemaAddOutcome> {
        let schema_name = &schema.name;

        Self::validate_schema_name(schema_name)?;

        let schema_name = schema_name.to_string();

        // Serialize Schema to JSON for similarity comparison
        let normalized_new_value = serde_json::to_value(&schema).map_err(|e| {
            FoldDbError::Serialization(format!(
                "Failed to serialize new schema for comparison: {}",
                e
            ))
        })?;
        let canonical_new = Self::normalized_json_string_without_name(&normalized_new_value)?;

        let mut schemas = self
            .schemas
            .lock()
            .map_err(|_| FoldDbError::Config("Failed to acquire schemas lock".to_string()))?;

        let mut closest_match: Option<(String, Schema, f64)> = None;

        for (existing_name, existing_schema) in schemas.iter() {
            // Convert Schema to JSON for similarity comparison
            let existing_value = serde_json::to_value(existing_schema).map_err(|e| {
                FoldDbError::Serialization(format!(
                    "Failed to serialize existing schema for comparison: {}",
                    e
                ))
            })?;
            // Serialize existing Schema to JSON for comparison
            let canonical_existing =
                Self::normalized_json_string_without_name(&existing_value)?;
            let similarity = normalized_levenshtein(&canonical_new, &canonical_existing);

            if closest_match
                .as_ref()
                .map(|(_, _, current_similarity)| similarity > *current_similarity)
                .unwrap_or(true)
            {
                closest_match = Some((
                    existing_name.clone(),
                    existing_schema.clone(),
                    similarity,
                ));
            }
        }

        if let Some((existing_name, existing_schema, similarity)) = closest_match {
            // Combine fields with field_mappers keys for similarity checking
            let mut new_all_fields: std::collections::HashSet<String> = schema.fields
                .as_ref()
                .map(|f| f.iter().cloned().collect())
                .unwrap_or_default();
            if let Some(ref mappers) = schema.field_mappers {
                new_all_fields.extend(mappers.keys().cloned());
            }
            
            let mut existing_all_fields: std::collections::HashSet<String> = existing_schema.fields
                .as_ref()
                .map(|f| f.iter().cloned().collect())
                .unwrap_or_default();
            if let Some(ref mappers) = existing_schema.field_mappers {
                existing_all_fields.extend(mappers.keys().cloned());
            }
            
            let shared_fields: Vec<_> = new_all_fields.intersection(&existing_all_fields).cloned().collect();
            let shared_count = shared_fields.len();
            
            let new_field_count = new_all_fields.len();
            let existing_field_count = existing_all_fields.len();
            let counts_differ = new_field_count != existing_field_count;
            
            let overlap_ratio = if new_field_count == 0 && existing_field_count == 0 {
                0.0
            } else {
                shared_count as f64 / new_field_count.max(existing_field_count) as f64
            };

            let should_apply_field_mappers = counts_differ
                && (similarity >= SCHEMA_SIMILARITY_THRESHOLD
                    || overlap_ratio >= FIELD_OVERLAP_THRESHOLD);

            if should_apply_field_mappers {
                // Add field mappers for shared fields (only for fields, not already mapped fields)
                let new_fields_only: std::collections::HashSet<_> = schema.fields
                    .as_ref()
                    .map(|f| f.iter().cloned().collect())
                    .unwrap_or_default();
                let mut field_mappers = schema.field_mappers.take().unwrap_or_default();
                for field_name in shared_fields {
                    if new_fields_only.contains(&field_name) {
                        field_mappers
                            .entry(field_name.clone())
                            .or_insert_with(|| crate::schema::types::FieldMapper::new(&existing_name, &field_name));
                        
                        // Update mutation_mappers: any mapper pointing to this field should now point to existing schema
                        let target_value = format!("{}.{}", existing_name, field_name);
                        for (json_field, schema_field) in mutation_mappers.iter_mut() {
                            // Check if the mutation mapper points to this field
                            let field_to_check = if schema_field.contains('.') {
                                schema_field.rsplit('.').next().unwrap_or(schema_field)
                            } else {
                                schema_field.as_str()
                            };
                            
                            if field_to_check == field_name {
                                log_feature!(
                                    LogFeature::Schema,
                                    info,
                                    "Updating mutation mapper: {} -> {} (was: {})",
                                    json_field,
                                    target_value,
                                    schema_field
                                );
                                *schema_field = target_value.clone();
                            }
                        }
                    }
                }
                schema.field_mappers = Some(field_mappers);
            } else if similarity >= SCHEMA_SIMILARITY_THRESHOLD {
                return Ok(SchemaAddOutcome::TooSimilar(SchemaSimilarityResponse {
                    similarity,
                    closest_schema: existing_schema,
                }));
            }
        }

        let serialized_schema = serde_json::to_vec(&schema).map_err(|error| {
            FoldDbError::Serialization(format!(
                "Failed to serialize schema '{}': {}",
                schema_name, error
            ))
        })?;

        self.schemas_tree
            .insert(schema_name.as_bytes(), serialized_schema)
            .map_err(|error| {
                FoldDbError::Config(format!(
                    "Failed to insert schema '{}' into database: {}",
                    schema_name, error
                ))
            })?;

        self.db.flush().map_err(|error| {
            FoldDbError::Config(format!("Failed to flush database: {}", error))
        })?;

        schemas.insert(schema_name.clone(), schema.clone());

        Ok(SchemaAddOutcome::Added(schema, mutation_mappers))
    }

    fn validate_schema_name(schema_name: &str) -> FoldDbResult<()> {
        if schema_name.is_empty() {
            return Err(FoldDbError::Config(
                "Schema name must not be empty".to_string(),
            ));
        }

        if schema_name.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        }) {
            return Ok(());
        }

        Err(FoldDbError::Config(format!(
            "Schema name '{}' contains invalid characters",
            schema_name
        )))
    }

    fn normalized_json_string(value: &serde_json::Value) -> FoldDbResult<String> {
        let normalized = Self::normalize_value(value);
        serde_json::to_string(&normalized).map_err(|error| {
            FoldDbError::Serialization(format!("Failed to canonicalize schema: {}", error))
        })
    }

    fn normalized_json_string_without_name(value: &serde_json::Value) -> FoldDbResult<String> {
        let mut sanitized = value.clone();
        if let serde_json::Value::Object(map) = &mut sanitized {
            map.remove("name");
        }
        Self::normalized_json_string(&sanitized)
    }

    fn normalize_value(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut entries: Vec<_> = map.iter().collect();
                entries.sort_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));
                let mut sorted_map = serde_json::Map::with_capacity(entries.len());
                for (key, inner_value) in entries {
                    sorted_map.insert(key.clone(), Self::normalize_value(inner_value));
                }
                serde_json::Value::Object(sorted_map)
            }
            serde_json::Value::Array(items) => serde_json::Value::Array(items.iter().map(Self::normalize_value).collect()),
            _ => value.clone(),
        }
    }

}

/// List all available schemas
async fn list_schemas(state: web::Data<SchemaServiceState>) -> impl Responder {
    log_feature!(LogFeature::Schema, info, "Schema service: listing schemas");

    let schemas = match state.schemas.lock() {
        Ok(s) => s,
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to acquire schemas lock: {}",
                e
            );
            return HttpResponse::InternalServerError()
                .json(ErrorResponse {
                    error: "Failed to acquire schemas lock".to_string(),
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
    log_feature!(LogFeature::Schema, info, "Schema service: getting all available schemas");

    let schemas = match state.schemas.lock() {
        Ok(s) => s,
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to acquire schemas lock: {}",
                e
            );
            return HttpResponse::InternalServerError()
                .json(ErrorResponse {
                    error: "Failed to acquire schemas lock".to_string(),
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

    let schemas = match state.schemas.lock() {
        Ok(s) => s,
        Err(e) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to acquire schemas lock: {}",
                e
            );
            return HttpResponse::InternalServerError()
                .json(ErrorResponse {
                    error: "Failed to acquire schemas lock".to_string(),
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

    match state.load_schemas() {
        Ok(_) => {
            let schemas = match state.schemas.lock() {
                Ok(s) => s,
                Err(e) => {
                    log_feature!(
                        LogFeature::Schema,
                        error,
                        "Failed to acquire schemas lock: {}",
                        e
                    );
                    return HttpResponse::InternalServerError()
                        .json(ErrorResponse {
                            error: "Failed to acquire schemas lock".to_string(),
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
            HttpResponse::InternalServerError()
                .json(ErrorResponse {
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

    match state.add_schema(request.schema, request.mutation_mappers) {
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

/// Schema Service HTTP Server
pub struct SchemaServiceServer {
    state: web::Data<SchemaServiceState>,
    bind_address: String,
}

impl SchemaServiceServer {
    /// Create a new schema service server
    pub fn new(db_path: String, bind_address: &str) -> FoldDbResult<Self> {
        let state = SchemaServiceState::new(db_path)?;

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
                    .route("/schema/{name}", web::get().to(get_schema)),
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

    use crate::schema::types::FieldMapper;

    #[test]
    fn add_schema_adds_new_schema() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        let new_schema = Schema::new(
            "NewSchema".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "value".to_string()]),
            None,
            None,
        );

        let outcome = state
            .add_schema(new_schema.clone(), HashMap::new())
            .expect("failed to add schema");

        match outcome {
            SchemaAddOutcome::Added(schema, _mutation_mappers) => {
                assert_eq!(schema.name, "NewSchema");
                assert_eq!(schema, new_schema);
            }
            SchemaAddOutcome::TooSimilar(_) => panic!("schema should have been added"),
        }

        let stored_schemas = state
            .schemas
            .lock()
            .expect("failed to lock schema map after addition");

        assert!(stored_schemas.contains_key("NewSchema"));

        let db_value = state
            .schemas_tree
            .get(b"NewSchema")
            .expect("failed to query database")
            .expect("schema should exist in database");
        
        let stored_schema: Schema = serde_json::from_slice(&db_value)
            .expect("failed to deserialize stored schema");
        
        assert_eq!(stored_schema.name, "NewSchema");
    }

    #[test]
    fn add_schema_detects_similar_schema() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        let existing_schema = Schema::new(
            "Existing".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "value".to_string()]),
            None,
            None,
        );

        state
            .add_schema(existing_schema.clone(), HashMap::new())
            .expect("failed to add existing schema");

        let similar_schema = Schema::new(
            "PotentialDuplicate".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "value".to_string()]),
            None,
            None,
        );

        let outcome = state
            .add_schema(similar_schema.clone(), HashMap::new())
            .expect("failed to evaluate schema similarity");

        match outcome {
            SchemaAddOutcome::TooSimilar(conflict) => {
                assert!(conflict.similarity >= SCHEMA_SIMILARITY_THRESHOLD);
                assert_eq!(conflict.closest_schema.name, "Existing");
                assert_eq!(conflict.closest_schema, existing_schema);
            }
            SchemaAddOutcome::Added(_, _) => panic!("schema should have been rejected as similar"),
        }

        assert!(state
            .schemas_tree
            .get(b"PotentialDuplicate")
            .expect("failed to query database")
            .is_none());
    }

    #[test]
    fn add_schema_creates_field_mappers_for_similar_schema_with_different_fields() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        let existing_schema = Schema::new(
            "Existing".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "name".to_string()]),
            None,
            None,
        );

        state
            .add_schema(existing_schema, HashMap::new())
            .expect("failed to add existing schema");

        let new_schema = Schema::new(
            "ExistingPublic".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string(), "name".to_string(), "display_name".to_string()]),
            None,
            None,
        );

        let outcome = state
            .add_schema(new_schema.clone(), HashMap::new())
            .expect("failed to add schema with field mapper");

        let added_schema = match outcome {
            SchemaAddOutcome::Added(schema, _mutation_mappers) => schema,
            other => panic!("expected schema addition, got {:?}", other),
        };

        assert_eq!(added_schema.name, "ExistingPublic");

        let field_mappers = added_schema
            .field_mappers
            .as_ref()
            .expect("field mappers should exist");

        assert_eq!(
            field_mappers.get("id"),
            Some(&FieldMapper::new("Existing", "id"))
        );
        assert_eq!(
            field_mappers.get("name"),
            Some(&FieldMapper::new("Existing", "name"))
        );
        assert!(!field_mappers.contains_key("display_name"));

        let stored_schemas = state
            .schemas
            .lock()
            .expect("failed to lock schema map after field mapper addition");

        let stored_schema = stored_schemas
            .get("ExistingPublic")
            .expect("schema should be stored");

        assert!(stored_schema.field_mappers.is_some());
        let mappers = stored_schema.field_mappers.as_ref().unwrap();
        assert_eq!(mappers.get("id"), Some(&FieldMapper::new("Existing", "id")));

        let db_value = state
            .schemas_tree
            .get(b"ExistingPublic")
            .expect("failed to query database")
            .expect("schema should exist in database");
        
        let stored_db_schema: Schema = serde_json::from_slice(&db_value)
            .expect("failed to deserialize stored schema");
        
        assert!(stored_db_schema.field_mappers.is_some());
        let mappers = stored_db_schema.field_mappers.as_ref().unwrap();
        assert_eq!(mappers.get("id"), Some(&FieldMapper::new("Existing", "id")));
    }

    #[test]
    fn add_schema_rejects_invalid_name() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        let invalid_schema = Schema::new(
            "../traversal".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["id".to_string()]),
            None,
            None,
        );

        let error = state
            .add_schema(invalid_schema, HashMap::new())
            .expect_err("schema with invalid name should be rejected");

        match error {
            FoldDbError::Config(message) => {
                assert!(message.contains("invalid characters"));
            }
            other => panic!("expected config error, got {:?}", other),
        }

        assert!(state
            .schemas_tree
            .get(b"../traversal")
            .expect("failed to query database")
            .is_none());
    }

    #[test]
    fn get_available_schemas_returns_all_schemas() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();

        let state = SchemaServiceState::new(db_path.clone())
            .expect("failed to initialize schema service state");

        let schema1 = Schema::new(
            "UserSchema".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["user_id".to_string(), "username".to_string(), "email".to_string()]),
            None,
            None,
        );

        let schema2 = Schema::new(
            "ProductSchema".to_string(),
            crate::schema::types::SchemaType::Single,
            None,
            Some(vec!["product_id".to_string(), "title".to_string(), "price".to_string(), "description".to_string()]),
            None,
            None,
        );

        state.add_schema(schema1.clone(), HashMap::new()).expect("failed to add schema1");
        state.add_schema(schema2.clone(), HashMap::new()).expect("failed to add schema2");

        let schemas = state.schemas.lock().expect("failed to lock schemas");
        assert_eq!(schemas.len(), 2);
        assert!(schemas.contains_key("UserSchema"));
        assert!(schemas.contains_key("ProductSchema"));

        assert_eq!(schemas.get("UserSchema").unwrap().name, "UserSchema");
        assert_eq!(schemas.get("ProductSchema").unwrap().name, "ProductSchema");
    }
}
