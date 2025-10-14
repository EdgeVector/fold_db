use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer as ActixHttpServer, Responder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use strsim::normalized_levenshtein;

use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;

/// Response containing a list of available schema names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemasListResponse {
    pub schemas: Vec<String>,
}

/// Response containing a single schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaResponse {
    pub name: String,
    pub definition: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSimilarityResponse {
    pub similarity: f64,
    pub closest_schema: SchemaResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaAddOutcome {
    Added(SchemaResponse),
    TooSimilar(SchemaSimilarityResponse),
}

/// Shared state for the schema service
#[derive(Clone)]
pub struct SchemaServiceState {
    schemas: Arc<Mutex<HashMap<String, Value>>>,
    schemas_directory: String,
}

const SCHEMA_SIMILARITY_THRESHOLD: f64 = 0.9;

impl SchemaServiceState {
    pub fn new(schemas_directory: String) -> FoldDbResult<Self> {
        let state = Self {
            schemas: Arc::new(Mutex::new(HashMap::new())),
            schemas_directory,
        };

        // Load schemas on initialization
        state.load_schemas()?;

        Ok(state)
    }

    /// Load all schemas from the configured directory
    pub fn load_schemas(&self) -> FoldDbResult<()> {
        let dir_path = PathBuf::from(&self.schemas_directory);

        if !dir_path.exists() {
            log_feature!(
                LogFeature::Schema,
                warn,
                "Schema directory '{}' does not exist",
                self.schemas_directory
            );
            return Ok(());
        }

        let mut schemas = self
            .schemas
            .lock()
            .map_err(|_| FoldDbError::Config("Failed to acquire schemas lock".to_string()))?;

        schemas.clear();

        let entries = std::fs::read_dir(&dir_path).map_err(|e| {
            FoldDbError::Config(format!(
                "Failed to read schema directory '{}': {}",
                self.schemas_directory, e
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                FoldDbError::Config(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.extension().map(|ext| ext == "json").unwrap_or(false) {
                let content = std::fs::read_to_string(&path).map_err(|e| {
                    FoldDbError::Config(format!(
                        "Failed to read schema file '{}': {}",
                        path.display(),
                        e
                    ))
                })?;

                let schema_value: Value = serde_json::from_str(&content).map_err(|e| {
                    FoldDbError::Config(format!(
                        "Failed to parse schema file '{}': {}",
                        path.display(),
                        e
                    ))
                })?;

                if let Some(name) = schema_value
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                {
                    log_feature!(
                        LogFeature::Schema,
                        info,
                        "Loaded schema '{}' from {}",
                        name,
                        path.display()
                    );
                    schemas.insert(name, schema_value);
                } else {
                    log_feature!(
                        LogFeature::Schema,
                        warn,
                        "Schema file '{}' missing 'name' field",
                        path.display()
                    );
                }
            }
        }

        log_feature!(
            LogFeature::Schema,
            info,
            "Schema service loaded {} schemas from '{}'",
            schemas.len(),
            self.schemas_directory
        );

        Ok(())
    }

    pub fn add_schema(&self, schema_value: Value) -> FoldDbResult<SchemaAddOutcome> {
        let schema_name = schema_value
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| FoldDbError::Config("Schema payload missing 'name' field".to_string()))?
            .to_string();

        let canonical_new = Self::normalized_json_string_without_name(&schema_value)?;

        let mut schemas = self
            .schemas
            .lock()
            .map_err(|_| FoldDbError::Config("Failed to acquire schemas lock".to_string()))?;

        let mut closest_match: Option<(String, Value, f64)> = None;

        for (existing_name, existing_definition) in schemas.iter() {
            let canonical_existing =
                Self::normalized_json_string_without_name(existing_definition)?;
            let similarity = normalized_levenshtein(&canonical_new, &canonical_existing);

            if closest_match
                .as_ref()
                .map(|(_, _, current_similarity)| similarity > *current_similarity)
                .unwrap_or(true)
            {
                closest_match = Some((
                    existing_name.clone(),
                    existing_definition.clone(),
                    similarity,
                ));
            }
        }

        if let Some((existing_name, existing_definition, similarity)) = closest_match {
            if similarity >= SCHEMA_SIMILARITY_THRESHOLD {
                return Ok(SchemaAddOutcome::TooSimilar(SchemaSimilarityResponse {
                    similarity,
                    closest_schema: SchemaResponse {
                        name: existing_name,
                        definition: existing_definition,
                    },
                }));
            }
        }

        let mut schema_file_path = PathBuf::from(&self.schemas_directory);
        schema_file_path.push(format!("{}.json", schema_name));

        if let Some(parent) = schema_file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                FoldDbError::Config(format!(
                    "Failed to ensure schema directory '{}': {}",
                    parent.display(),
                    error
                ))
            })?;
        }

        let serialized_schema = serde_json::to_string_pretty(&schema_value).map_err(|error| {
            FoldDbError::Serialization(format!(
                "Failed to serialize schema '{}': {}",
                schema_name, error
            ))
        })?;

        std::fs::write(&schema_file_path, serialized_schema).map_err(|error| {
            FoldDbError::Config(format!(
                "Failed to write schema file '{}': {}",
                schema_file_path.display(),
                error
            ))
        })?;

        schemas.insert(schema_name.clone(), schema_value.clone());

        Ok(SchemaAddOutcome::Added(SchemaResponse {
            name: schema_name,
            definition: schema_value,
        }))
    }

    fn normalized_json_string(value: &Value) -> FoldDbResult<String> {
        let normalized = Self::normalize_value(value);
        serde_json::to_string(&normalized).map_err(|error| {
            FoldDbError::Serialization(format!("Failed to canonicalize schema: {}", error))
        })
    }

    fn normalized_json_string_without_name(value: &Value) -> FoldDbResult<String> {
        let mut sanitized = value.clone();
        if let Value::Object(map) = &mut sanitized {
            map.remove("name");
        }
        Self::normalized_json_string(&sanitized)
    }

    fn normalize_value(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut entries: Vec<_> = map.iter().collect();
                entries.sort_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));
                let mut sorted_map = serde_json::Map::with_capacity(entries.len());
                for (key, inner_value) in entries {
                    sorted_map.insert(key.clone(), Self::normalize_value(inner_value));
                }
                Value::Object(sorted_map)
            }
            Value::Array(items) => Value::Array(items.iter().map(Self::normalize_value).collect()),
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
                .json(serde_json::json!({"error": "Failed to acquire schemas lock"}));
        }
    };

    let schema_names: Vec<String> = schemas.keys().cloned().collect();

    HttpResponse::Ok().json(SchemasListResponse {
        schemas: schema_names,
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
                .json(serde_json::json!({"error": "Failed to acquire schemas lock"}));
        }
    };

    match schemas.get(&schema_name) {
        Some(definition) => HttpResponse::Ok().json(SchemaResponse {
            name: schema_name,
            definition: definition.clone(),
        }),
        None => {
            log_feature!(
                LogFeature::Schema,
                warn,
                "Schema '{}' not found",
                schema_name
            );
            HttpResponse::NotFound().json(serde_json::json!({"error": "Schema not found"}))
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
                        .json(serde_json::json!({"error": "Failed to acquire schemas lock"}));
                }
            };

            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "schemas_loaded": schemas.len()
            }))
        }
        Err(e) => {
            log_feature!(LogFeature::Schema, error, "Failed to reload schemas: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": format!("Failed to reload schemas: {}", e)}))
        }
    }
}

async fn add_schema(
    payload: web::Json<Value>,
    state: web::Data<SchemaServiceState>,
) -> impl Responder {
    let schema_value = payload.into_inner();
    let schema_name = schema_value
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or("<unknown>")
        .to_string();

    log_feature!(
        LogFeature::Schema,
        info,
        "Schema service: adding schema '{}'",
        schema_name
    );

    match state.add_schema(schema_value) {
        Ok(SchemaAddOutcome::Added(schema)) => HttpResponse::Created().json(schema),
        Ok(SchemaAddOutcome::TooSimilar(conflict)) => {
            HttpResponse::Conflict().json(serde_json::json!({
                "error": "Schema too similar to existing schema",
                "similarity": conflict.similarity,
                "closest_schema": conflict.closest_schema,
            }))
        }
        Err(error) => {
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to add schema '{}': {}",
                schema_name,
                error
            );
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Failed to add schema: {}", error)
            }))
        }
    }
}

/// Health check endpoint
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "healthy"}))
}

/// Schema Service HTTP Server
pub struct SchemaServiceServer {
    state: web::Data<SchemaServiceState>,
    bind_address: String,
}

impl SchemaServiceServer {
    /// Create a new schema service server
    pub fn new(schemas_directory: String, bind_address: &str) -> FoldDbResult<Self> {
        let state = SchemaServiceState::new(schemas_directory)?;

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
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn add_schema_adds_new_schema() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let schemas_directory = temp_dir.path().to_string_lossy().to_string();

        let state = SchemaServiceState::new(schemas_directory.clone())
            .expect("failed to initialize schema service state");

        let new_schema = json!({
            "name": "NewSchema",
            "fields": [
                {"name": "id", "type": "string"},
                {"name": "value", "type": "number"}
            ]
        });

        let outcome = state
            .add_schema(new_schema.clone())
            .expect("failed to add schema");

        match outcome {
            SchemaAddOutcome::Added(response) => {
                assert_eq!(response.name, "NewSchema");
                assert_eq!(response.definition, new_schema);
            }
            SchemaAddOutcome::TooSimilar(_) => panic!("schema should have been added"),
        }

        let stored_schemas = state
            .schemas
            .lock()
            .expect("failed to lock schema map after addition");

        assert!(stored_schemas.contains_key("NewSchema"));

        let expected_path = PathBuf::from(schemas_directory).join("NewSchema.json");
        assert!(expected_path.exists());
    }

    #[test]
    fn add_schema_detects_similar_schema() {
        let temp_dir = tempdir().expect("failed to create temp directory");
        let schemas_directory = temp_dir.path().to_string_lossy().to_string();

        let existing_schema = json!({
            "name": "Existing",
            "fields": [
                {"name": "id", "type": "string"},
                {"name": "value", "type": "number"}
            ]
        });

        let existing_path = temp_dir.path().join("Existing.json");
        fs::write(
            &existing_path,
            serde_json::to_string_pretty(&existing_schema)
                .expect("failed to serialize existing schema"),
        )
        .expect("failed to write existing schema");

        let state = SchemaServiceState::new(schemas_directory.clone())
            .expect("failed to initialize schema service state");

        let similar_schema = json!({
            "name": "PotentialDuplicate",
            "fields": [
                {"name": "id", "type": "string"},
                {"name": "value", "type": "number"}
            ]
        });

        let outcome = state
            .add_schema(similar_schema.clone())
            .expect("failed to evaluate schema similarity");

        match outcome {
            SchemaAddOutcome::TooSimilar(conflict) => {
                assert!(conflict.similarity >= SCHEMA_SIMILARITY_THRESHOLD);
                assert_eq!(conflict.closest_schema.name, "Existing");
                assert_eq!(conflict.closest_schema.definition, existing_schema);
            }
            SchemaAddOutcome::Added(_) => panic!("schema should have been rejected as similar"),
        }

        let duplicate_path = PathBuf::from(schemas_directory).join("PotentialDuplicate.json");
        assert!(!duplicate_path.exists());
    }
}
