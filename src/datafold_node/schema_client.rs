use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use crate::schema::types::Schema;
use reqwest::StatusCode;
use serde::Deserialize;

/// Client for communicating with the schema service
#[derive(Clone)]
pub struct SchemaServiceClient {
    base_url: String,
    client: reqwest::Client,
}

// No longer needed - the server returns Schema directly

impl SchemaServiceClient {
    /// Create a new schema service client
    pub fn new(schema_service_url: &str) -> Self {
        Self {
            base_url: schema_service_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Add a schema definition to the schema service.
    pub async fn add_schema(&self, schema: &Schema) -> FoldDbResult<Schema> {
        let url = format!("{}/api/schemas", self.base_url);

        log_feature!(
            LogFeature::Schema,
            info,
            "Adding schema via schema service at {}",
            url
        );

        let response = self
            .client
            .post(&url)
            .json(schema)
            .send()
            .await
            .map_err(|error| {
                FoldDbError::Config(format!(
                    "Failed to submit schema to schema service: {}",
                    error
                ))
            })?;

        if response.status() == StatusCode::CREATED {
            let schema = response
                .json::<Schema>()
                .await
                .map_err(|error| {
                    FoldDbError::Config(format!(
                        "Failed to parse schema creation response: {}",
                        error
                    ))
                })?;

            log_feature!(
                LogFeature::Schema,
                info,
                "Schema '{}' added via schema service",
                schema.name
            );

            return Ok(schema);
        }

        if response.status() == StatusCode::CONFLICT {
            #[derive(Deserialize)]
            struct ErrorBody {
                error: String,
            }
            
            let error_message = response
                .json::<ErrorBody>()
                .await
                .map(|body| body.error)
                .unwrap_or_else(|_| "Schema service reported a conflict when adding schema".to_string());

            return Err(FoldDbError::Config(format!(
                "Schema service conflict: {}",
                error_message
            )));
        }

        let status = response.status();
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

        log_feature!(
            LogFeature::Schema,
            info,
            "Fetching schema list from {}",
            url
        );

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
            schemas: Vec<String>,
        }

        let schemas_response: SchemasListResponse = response.json().await.map_err(|e| {
            FoldDbError::Config(format!("Failed to parse schema list response: {}", e))
        })?;

        log_feature!(
            LogFeature::Schema,
            info,
            "Received {} schemas from schema service",
            schemas_response.schemas.len()
        );

        Ok(schemas_response.schemas)
    }

    /// Get all available schemas with their full definitions from the schema service
    pub async fn get_available_schemas(&self) -> FoldDbResult<Vec<Schema>> {
        let url = format!("{}/api/schemas/available", self.base_url);

        log_feature!(
            LogFeature::Schema,
            info,
            "Fetching all available schemas from {}",
            url
        );

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

        log_feature!(
            LogFeature::Schema,
            info,
            "Received {} available schemas from schema service",
            schemas_response.schemas.len()
        );

        Ok(schemas_response.schemas)
    }

    /// Get a specific schema definition from the schema service
    pub async fn get_schema(&self, name: &str) -> FoldDbResult<Schema> {
        let url = format!("{}/api/schema/{}", self.base_url, name);

        log_feature!(
            LogFeature::Schema,
            info,
            "Fetching schema '{}' from {}",
            name,
            url
        );

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

        log_feature!(
            LogFeature::Schema,
            info,
            "Successfully fetched schema '{}' from schema service",
            name
        );

        Ok(schema)
    }

    /// Load all schemas from the schema service into the node
    pub async fn load_all_schemas(
        &self,
        schema_manager: &crate::schema::SchemaCore,
    ) -> FoldDbResult<usize> {
        let schema_names = self.list_schemas().await?;
        let mut loaded_count = 0;

        for name in schema_names {
            let schema = self.get_schema(&name).await?;

            let json_str = serde_json::to_string(&schema).map_err(|e| {
                FoldDbError::Config(format!("Failed to serialize schema '{}': {}", name, e))
            })?;

            schema_manager
                .load_schema_from_json(&json_str)
                .map_err(|e| {
                    FoldDbError::Config(format!("Failed to load schema '{}': {}", name, e))
                })?;

            loaded_count += 1;
            log_feature!(
                LogFeature::Schema,
                info,
                "Loaded schema '{}' from schema service",
                name
            );
        }

        Ok(loaded_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::SchemaType;
    use crate::schema_service::server::{SchemaAddOutcome, SchemaServiceState, ErrorResponse, ConflictResponse};
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
            App::new()
                .app_data(web::Data::new(state))
                .service(web::scope("/api").route(
                "/schemas",
                web::post().to(
                    |payload: web::Json<Schema>, state: web::Data<SchemaServiceState>| async move {
                        let schema = payload.into_inner();
                        
                        match state.add_schema(schema) {
                            Ok(SchemaAddOutcome::Added(schema)) => {
                                HttpResponse::Created().json(schema)
                            }
                            Ok(SchemaAddOutcome::TooSimilar(conflict)) => HttpResponse::Conflict()
                                .json(ConflictResponse {
                                    error: "Schema too similar to existing schema".to_string(),
                                    similarity: conflict.similarity,
                                    closest_schema: conflict.closest_schema,
                                }),
                            Err(error) => HttpResponse::BadRequest()
                                .json(ErrorResponse {
                                    error: format!("Failed to add schema: {}", error),
                                }),
                        }
                    },
                ),
            ))
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
        let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();
        let state = SchemaServiceState::new(db_path)
            .expect("failed to create schema service state");

        let (base_url, handle) = spawn_schema_service(state).await;

        let client = SchemaServiceClient::new(&base_url);
        let schema = Schema::new(
            "TestSchema".to_string(),
            SchemaType::Single,
            None,
            Some(vec!["id".to_string()]),
            None,
            None,
        );

        let response = client
            .add_schema(&schema)
            .await
            .expect("schema addition should succeed");

        assert_eq!(response.name, "TestSchema");

        handle.stop(true).await;
    }

    #[actix_web::test]
    async fn add_schema_conflict_is_reported() {
        let temp_dir = tempdir().expect("failed to create tempdir");
        let db_path = temp_dir.path().join("test_schema_db").to_string_lossy().to_string();
        let state = SchemaServiceState::new(db_path)
            .expect("failed to create schema service state");

        let (base_url, handle) = spawn_schema_service(state).await;

        let client = SchemaServiceClient::new(&base_url);
        let schema = Schema::new(
            "ExistingSchema".to_string(),
            SchemaType::Single,
            None,
            Some(vec!["id".to_string()]),
            None,
            None,
        );

        client
            .add_schema(&schema)
            .await
            .expect("initial schema creation should succeed");

        let error = client
            .add_schema(&schema)
            .await
            .expect_err("duplicate schema creation should fail");

        match error {
            FoldDbError::Config(message) => {
                assert!(message.contains("Schema service conflict"));
            }
            other => panic!("unexpected error type: {:?}", other),
        }

        handle.stop(true).await;
    }
}
