use crate::error::{FoldDbError, FoldDbResult};
use crate::log_feature;
use crate::logging::features::LogFeature;
use serde_json::Value;

/// Client for communicating with the schema service
#[derive(Clone)]
pub struct SchemaServiceClient {
    base_url: String,
    client: reqwest::Client,
}

impl SchemaServiceClient {
    /// Create a new schema service client
    pub fn new(schema_service_url: &str) -> Self {
        Self {
            base_url: schema_service_url.to_string(),
            client: reqwest::Client::new(),
        }
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
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| FoldDbError::Config(format!("Failed to fetch schemas from service: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(FoldDbError::Config(format!(
                "Schema service returned error: {}",
                response.status()
            )));
        }
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| FoldDbError::Config(format!("Failed to parse schema list response: {}", e)))?;
        
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
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| FoldDbError::Config(format!("Failed to fetch schema '{}': {}", name, e)))?;
        
        if !response.status().is_success() {
            return Err(FoldDbError::Config(format!(
                "Schema service returned error for '{}': {}",
                name,
                response.status()
            )));
        }
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| FoldDbError::Config(format!("Failed to parse schema '{}' response: {}", name, e)))?;
        
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
            
            schema_manager.load_schema_from_json(&json_str).map_err(|e| {
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

