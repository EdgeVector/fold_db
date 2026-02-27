use crate::error::{FoldDbError, FoldDbResult};
use crate::schema::types::Schema;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Client for communicating with the schema service
#[derive(Clone)]
pub struct SchemaServiceClient {
    base_url: String,
    client: reqwest::Client,
}

/// Request structure for adding a schema with mutation mappers
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AddSchemaRequest {
    schema: Schema,
    mutation_mappers: HashMap<String, String>,
}

/// Response structure for adding a schema with mutation mappers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddSchemaResponse {
    pub schema: Schema,
    pub mutation_mappers: HashMap<String, String>,
}

impl SchemaServiceClient {
    /// Create a new schema service client
    pub fn new(schema_service_url: &str) -> Self {
        // Create client with timeout to prevent hanging
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            base_url: schema_service_url.to_string(),
            client,
        }
    }

    /// Add a schema definition to the schema service.
    pub async fn add_schema(
        &self,
        schema: &Schema,
        mutation_mappers: HashMap<String, String>,
    ) -> FoldDbResult<AddSchemaResponse> {
        let url = format!("{}/api/schemas", self.base_url);

        let request = AddSchemaRequest {
            schema: schema.clone(),
            mutation_mappers,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|error| {
                FoldDbError::Config(format!(
                    "Failed to submit schema to schema service at {}: {}. Is the schema service running?",
                    url,
                    error
                ))
            })?;

        let status = response.status();

        if status == StatusCode::CREATED || status == StatusCode::OK {
            return response.json::<AddSchemaResponse>().await.map_err(|error| {
                FoldDbError::Config(format!("Failed to parse schema response: {}", error))
            });
        }

        if status == StatusCode::CONFLICT {
            #[derive(Deserialize)]
            struct ConflictBody {
                closest_schema: Schema,
            }

            let conflict_body = response.json::<ConflictBody>().await.map_err(|error| {
                FoldDbError::Config(format!(
                    "Failed to parse schema conflict response: {}",
                    error
                ))
            })?;

            return Ok(AddSchemaResponse {
                schema: conflict_body.closest_schema,
                mutation_mappers: HashMap::new(),
            });
        }

        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<empty>".to_string());
        Err(FoldDbError::Config(format!(
            "Schema service add schema failed with status {}: {}",
            status, body
        )))
    }

    /// List all available schemas from the schema service
    pub async fn list_schemas(&self) -> FoldDbResult<Vec<String>> {
        let url = format!("{}/api/schemas", self.base_url);

        let response = self.client.get(&url).send().await.map_err(|e| {
            FoldDbError::Config(format!("Failed to fetch schemas from service: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(FoldDbError::Config(format!(
                "Schema service returned error: {}",
                response.status()
            )));
        }

        #[derive(Deserialize)]
        struct SchemasListResponse {
            schemas: Vec<serde_json::Value>,
        }

        let schemas_response: SchemasListResponse = response.json().await.map_err(|e| {
            FoldDbError::Config(format!("Failed to parse schema list response: {}", e))
        })?;

        let names: Vec<String> = schemas_response
            .schemas
            .into_iter()
            .filter_map(|v| {
                if let Some(s) = v.as_str() {
                    Some(s.to_string())
                } else if let Some(obj) = v.as_object() {
                    // Try "name" or "schema.name"
                    obj.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            obj.get("schema")
                                .and_then(|s| s.get("name"))
                                .and_then(|n| n.as_str())
                                .map(|s| s.to_string())
                        })
                } else {
                    None
                }
            })
            .collect();

        Ok(names)
    }

    /// Get all available schemas with their full definitions from the schema service
    pub async fn get_available_schemas(&self) -> FoldDbResult<Vec<Schema>> {
        let url = format!("{}/api/schemas/available", self.base_url);

        let response = self.client.get(&url).send().await.map_err(|e| {
            FoldDbError::Config(format!("Failed to fetch available schemas: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(FoldDbError::Config(format!(
                "Schema service returned error: {}",
                response.status()
            )));
        }

        #[derive(Deserialize)]
        struct AvailableSchemasResponse {
            schemas: Vec<Schema>,
        }

        let schemas_response: AvailableSchemasResponse = response.json().await.map_err(|e| {
            FoldDbError::Config(format!("Failed to parse available schemas response: {}", e))
        })?;

        Ok(schemas_response.schemas)
    }

    /// Get a specific schema definition from the schema service
    pub async fn get_schema(&self, name: &str) -> FoldDbResult<Schema> {
        let url = format!("{}/api/schema/{}", self.base_url, name);

        let response = self.client.get(&url).send().await.map_err(|e| {
            FoldDbError::Config(format!("Failed to fetch schema '{}': {}", name, e))
        })?;

        if !response.status().is_success() {
            return Err(FoldDbError::Config(format!(
                "Schema service returned error for '{}': {}",
                name,
                response.status()
            )));
        }

        let schema: Schema = response.json().await.map_err(|e| {
            FoldDbError::Config(format!("Failed to parse schema '{}' response: {}", name, e))
        })?;

        Ok(schema)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::SchemaType;
    use crate::schema_service::server::{
        ConflictResponse, ErrorResponse, SchemaAddOutcome, SchemaServiceState,
    };
    use actix_web::{rt::time::sleep, web, App, HttpResponse, HttpServer};
    use std::net::TcpListener;
    use std::time::Duration;
    use tempfile::tempdir;

    async fn spawn_schema_service(
        state: SchemaServiceState,
    ) -> (String, actix_web::dev::ServerHandle) {
        let server_state = state.clone();

        let listener = TcpListener::bind(("127.0.0.1", 0))
            .expect("failed to bind schema service test listener");
        let bound_address = listener
            .local_addr()
            .expect("failed to read schema service test listener address");

        let server = HttpServer::new(move || {
            let state = server_state.clone();
            App::new().app_data(web::Data::new(state)).service(
                web::scope("/api").route(
                    "/schemas",
                    web::post().to(
                        |payload: web::Json<AddSchemaRequest>,
                         state: web::Data<SchemaServiceState>| async move {
                            let request = payload.into_inner();

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
                                Ok(SchemaAddOutcome::AlreadyExists(schema)) => {
                                    HttpResponse::Ok().json(AddSchemaResponse {
                                        schema,
                                        mutation_mappers: HashMap::new(),
                                    })
                                }
                                Ok(SchemaAddOutcome::TooSimilar(conflict)) => {
                                    HttpResponse::Conflict().json(ConflictResponse {
                                        error: "Schema too similar to existing schema".to_string(),
                                        similarity: conflict.similarity,
                                        closest_schema: conflict.closest_schema,
                                    })
                                }
                                Err(error) => HttpResponse::BadRequest().json(ErrorResponse {
                                    error: format!("Failed to add schema: {}", error),
                                }),
                            }
                        },
                    ),
                ),
            )
        })
        .listen(listener)
        .expect("failed to listen for test schema service")
        .run();

        let address = bound_address;
        let handle = server.handle();
        actix_web::rt::spawn(server);
        sleep(Duration::from_millis(50)).await;

        (format!("http://{}", address), handle)
    }

    #[actix_web::test]
    async fn add_schema_succeeds() {
        let temp_dir = tempdir().expect("failed to create tempdir");
        let db_path = temp_dir
            .path()
            .join("test_schema_db")
            .to_string_lossy()
            .to_string();
        let state =
            SchemaServiceState::new(db_path).expect("failed to create schema service state");

        let (base_url, handle) = spawn_schema_service(state).await;

        let client = SchemaServiceClient::new(&base_url);
        let mut schema = Schema::new(
            "TestSchema".to_string(),
            SchemaType::Single,
            None,
            Some(vec!["id".to_string()]),
            None,
            None,
        );

        // Add classifications
        schema.field_classifications.insert("id".to_string(), vec!["word".to_string()]);

        let response = client
            .add_schema(&schema, HashMap::new())
            .await
            .expect("schema addition should succeed");

        // Schema name should be the identity_hash (64 char hex string)
        assert_eq!(response.schema.name.len(), 64);

        handle.stop(true).await;
    }

    #[actix_web::test]
    async fn list_schemas_handles_objects() {
        let temp_dir = tempdir().expect("failed to create tempdir");
        let _db_path = temp_dir
            .path()
            .join("test_schema_list_db")
            .to_string_lossy()
            .to_string();

        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("failed to bind");
        let port = listener.local_addr().unwrap().port();

        let server = HttpServer::new(|| {
            App::new().route(
                "/api/schemas",
                web::get().to(|| async {
                    HttpResponse::Ok().json(serde_json::json!({
                        "schemas": [
                            "simple_string_schema",
                            {"name": "object_schema", "state": "approved"},
                            {"schema": {"name": "nested_schema"}}
                        ]
                    }))
                }),
            )
        })
        .listen(listener)
        .expect("failed to listen")
        .run();

        let handle = server.handle();
        actix_web::rt::spawn(server);
        sleep(Duration::from_millis(50)).await;

        let client = SchemaServiceClient::new(&format!("http://127.0.0.1:{}", port));
        let schemas = client.list_schemas().await.expect("failed to list schemas");

        assert!(schemas.contains(&"simple_string_schema".to_string()));
        assert!(schemas.contains(&"object_schema".to_string()));
        assert!(schemas.contains(&"nested_schema".to_string()));
        assert_eq!(schemas.len(), 3);

        handle.stop(true).await;
    }
}
