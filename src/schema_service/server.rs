use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer as ActixHttpServer, Responder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

/// Shared state for the schema service
#[derive(Clone)]
pub struct SchemaServiceState {
    schemas: Arc<Mutex<HashMap<String, Value>>>,
    schemas_directory: String,
}

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
        
        let mut schemas = self.schemas.lock().map_err(|_| {
            FoldDbError::Config("Failed to acquire schemas lock".to_string())
        })?;
        
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
                    FoldDbError::Config(format!("Failed to read schema file '{}': {}", path.display(), e))
                })?;
                
                let schema_value: Value = serde_json::from_str(&content).map_err(|e| {
                    FoldDbError::Config(format!("Failed to parse schema file '{}': {}", path.display(), e))
                })?;
                
                if let Some(name) = schema_value.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()) {
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
}

/// List all available schemas
async fn list_schemas(state: web::Data<SchemaServiceState>) -> impl Responder {
    log_feature!(LogFeature::Schema, info, "Schema service: listing schemas");
    
    let schemas = match state.schemas.lock() {
        Ok(s) => s,
        Err(e) => {
            log_feature!(LogFeature::Schema, error, "Failed to acquire schemas lock: {}", e);
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
            log_feature!(LogFeature::Schema, error, "Failed to acquire schemas lock: {}", e);
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
    log_feature!(LogFeature::Schema, info, "Schema service: reloading schemas");
    
    match state.load_schemas() {
        Ok(_) => {
            let schemas = match state.schemas.lock() {
                Ok(s) => s,
                Err(e) => {
                    log_feature!(LogFeature::Schema, error, "Failed to acquire schemas lock: {}", e);
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
            log_feature!(
                LogFeature::Schema,
                error,
                "Failed to reload schemas: {}",
                e
            );
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": format!("Failed to reload schemas: {}", e)}))
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
            
            App::new()
                .wrap(cors)
                .app_data(state.clone())
                .service(
                    web::scope("/api")
                        .route("/health", web::get().to(health_check))
                        .route("/schemas", web::get().to(list_schemas))
                        .route("/schemas/reload", web::post().to(reload_schemas))
                        .route("/schema/{name}", web::get().to(get_schema))
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

