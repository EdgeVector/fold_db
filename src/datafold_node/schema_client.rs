use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::Value;

/// Client for communicating with the schema service
#[derive(Clone)]
pub struct SchemaServiceClient {
    base_url: String,
    client: reqwest::Client,
}

/// Response returned when a schema is successfully added via the schema service.
#[derive(Debug, Deserialize)]
pub struct SchemaAddResponse {
    pub name: String,
    pub definition: Value,
}

impl SchemaServiceClient {
    /// Create a new schema service client
    pub fn new(schema_service_url: &str) -> Self {
        Self {
            base_url: schema_service_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Add a schema definition to the schema service.
    pub async fn add_schema(&self, schema_definition: &Value) -> FoldDbResult<SchemaAddResponse> {
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
            .json(schema_definition)
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
                .json::<SchemaAddResponse>()
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
            let body: Value = response.json().await.unwrap_or(Value::Null);
            let message = body
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or("Schema service reported a conflict when adding schema");

            return Err(FoldDbError::Config(format!(
                "Schema service conflict: {}",
                message
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

        let json: serde_json::Value = response.json().await.map_err(|e| {
            FoldDbError::Config(format!("Failed to parse schema list response: {}", e))
        })?;

        let schemas = json
            .get("schemas")
            .and_then(|v| v.as_array())
            .ok_or_else(|| FoldDbError::Config("Invalid schema list response".to_string()))?;

        let schema_names: Vec<String> = schemas
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        log_feature!(
            LogFeature::Schema,
            info,
            "Received {} schemas from schema service",
            schema_names.len()
        );

        Ok(schema_names)
    }

    /// Get a specific schema definition from the schema service
    pub async fn get_schema(&self, name: &str) -> FoldDbResult<Value> {
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

        let json: serde_json::Value = response.json().await.map_err(|e| {
            FoldDbError::Config(format!("Failed to parse schema '{}' response: {}", name, e))
        })?;

        let definition = json
            .get("definition")
            .ok_or_else(|| FoldDbError::Config(format!("Invalid schema response for '{}'", name)))?
            .clone();

        log_feature!(
            LogFeature::Schema,
            info,
            "Successfully fetched schema '{}' from schema service",
            name
        );

        Ok(definition)
    }

    /// Load all schemas from the schema service into the node
    pub async fn load_all_schemas(
        &self,
        schema_manager: &crate::schema::SchemaCore,
    ) -> FoldDbResult<usize> {
        let schema_names = self.list_schemas().await?;
        let mut loaded_count = 0;

        for name in schema_names {
            let definition = self.get_schema(&name).await?;

            let json_str = serde_json::to_string(&definition).map_err(|e| {
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
    use crate::schema_service::server::{SchemaAddOutcome, SchemaServiceState};
    use actix_web::{rt::time::sleep, web, App, HttpResponse, HttpServer};
    use serde_json::{json, Value};
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
                    |payload: web::Json<Value>, state: web::Data<SchemaServiceState>| async move {
                        match state.add_schema(payload.into_inner()) {
                            Ok(SchemaAddOutcome::Added(schema)) => {
                                HttpResponse::Created().json(schema)
                            }
                            Ok(SchemaAddOutcome::TooSimilar(conflict)) => HttpResponse::Conflict()
                                .json(json!({
                                    "error": "Schema too similar to existing schema",
                                    "similarity": conflict.similarity,
                                    "closest_schema": conflict.closest_schema
                                })),
                            Err(error) => HttpResponse::BadRequest()
                                .json(json!({"error": format!("Failed to add schema: {}", error)})),
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
        let state = SchemaServiceState::new(temp_dir.path().to_string_lossy().to_string())
            .expect("failed to create schema service state");

        let (base_url, handle) = spawn_schema_service(state).await;

        let client = SchemaServiceClient::new(&base_url);
        let schema = json!({
            "name": "TestSchema",
            "fields": [
                {"name": "id", "type": "string"}
            ]
        });

        let response = client
            .add_schema(&schema)
            .await
            .expect("schema addition should succeed");

        assert_eq!(response.name, "TestSchema");
        assert_eq!(response.definition.get("name").unwrap(), "TestSchema");

        handle.stop(true).await;
    }

    #[actix_web::test]
    async fn add_schema_conflict_is_reported() {
        let temp_dir = tempdir().expect("failed to create tempdir");
        let state = SchemaServiceState::new(temp_dir.path().to_string_lossy().to_string())
            .expect("failed to create schema service state");

        let (base_url, handle) = spawn_schema_service(state).await;

        let client = SchemaServiceClient::new(&base_url);
        let schema = json!({
            "name": "ExistingSchema",
            "fields": [
                {"name": "id", "type": "string"}
            ]
        });

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
