// OpenRouter API service for AI-powered schema analysis

use crate::ingestion::config::OpenRouterConfig;
use crate::ingestion::{AISchemaResponse, IngestionError, IngestionResult, StructureAnalyzer};
use crate::log_feature;
use crate::logging::features::LogFeature;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Prompt header shared across requests
const PROMPT_HEADER: &str = r#"Tell me which of these schemas to use for this sample json data. If none are available, then create a new one. Return the value in this format:
{
  \"existing_schemas\": [<list_of_schema_names>],
  \"new_schemas\": <single_schema_definition>,
  \"mutation_mappers\": {json_field_name: schema_field_name}
}

Where:
- existing_schemas is an array of schema names that match the input data
- new_schemas is a single schema definition if no existing schemas match
- mutation_mappers maps ONLY TOP-LEVEL JSON field names to schema field names (e.g., {\"id\": \"id\", \"user\": \"user\"})

CRITICAL - Mutation Mappers:
- ONLY use top-level field names in mutation_mappers (e.g., \"user\", \"comments\", \"id\")
- DO NOT use nested paths (e.g., \"user.name\", \"comments[*].content\") - they will not work
- Nested objects and arrays will be stored as-is in their top-level field
- Example: if JSON has {\"user\": {\"id\": 1, \"name\": \"Tom\"}}, mapper should be {\"user\": \"user\"}, NOT {\"user.id\": \"id\"}

IMPORTANT - Schema Types:
- For storing MULTIPLE entities/records, use \"key\": {\"range_field\": \"field_name\"}
- For storing ONE global value per field, omit the \"key\" field
- If the user is providing an ARRAY of objects, you MUST use a Range schema with a \"key\"
- The range_field should be a unique identifier field (like \"name\", \"id\", \"email\")

IMPORTANT - Schema Name and Descriptive Name:
- You MUST include \"name\": use any simple name like \"Schema\" (it will be replaced automatically)
- ALWAYS include \"descriptive_name\": a clear, human-readable description of what this schema stores
- Example: \"descriptive_name\": \"User Profile Information\" or \"Customer Order Records\"

IMPORTANT - Field Topologies with Classifications:
- EVERY Primitive leaf MUST include \"classifications\" array
- Analyze field semantic meaning and assign appropriate classification types
- Multiple classifications per field are encouraged (e.g., [\"name:person\", \"word\"])
- Available classification types:
  * \"word\" - general text, split into words for search
  * \"name:person\" - person names (kept whole: \"Jennifer Liu\")
  * \"name:company\" - company/organization names
  * \"name:place\" - location names (cities, countries, places)
  * \"email\" - email addresses
  * \"phone\" - phone numbers
  * \"url\" - URLs or domains
  * \"date\" - dates and timestamps
  * \"hashtag\" - hashtags (from social media)
  * \"username\" - usernames/handles
- Topology structure:
  * Primitives: {\"type\": \"Primitive\", \"value\": \"String\", \"classifications\": [\"name:person\", \"word\"]}
  * Objects: {\"type\": \"Object\", \"value\": {\"field_name\": {\"type\": \"Primitive\", \"value\": \"String\", \"classifications\": [\"word\"]}}}
  * Arrays of Primitives: {\"type\": \"Array\", \"value\": {\"type\": \"Primitive\", \"value\": \"String\", \"classifications\": [\"hashtag\", \"word\"]}}
  * Arrays of Objects: {\"type\": \"Array\", \"value\": {\"type\": \"Object\", \"value\": {\"field_name\": {\"type\": \"Primitive\", \"value\": \"String\", \"classifications\": [\"word\"]}}}}

CRITICAL - Using Flattened Path Structure:
- The superset structure now uses flattened dot-separated paths instead of nested structures
- Each path represents a field with its type (e.g., \"entities.user_mentions[0].id\": \"string\")
- Convert these flattened paths into proper nested topology structures
- For arrays of objects, paths like \"user_mentions[0].field\" mean:
  * user_mentions is an Array
  * Each array element is an Object  
  * Each object has the field \"field\"
  * Create topology: {\"type\": \"Array\", \"value\": {\"type\": \"Object\", \"value\": {\"field\": {\"type\": \"Primitive\", \"value\": \"String\", \"classifications\": [\"word\"]}}}}
- Group paths by their base path and create proper nested structures
- IMPORTANT: When you see paths like \"user_mentions[0].id\", \"user_mentions[0].name\", etc., this means:
  * user_mentions is an Array (not an Object)
  * Each array element is an Object with fields: id, name, etc.
  * The topology should be: {\"type\": \"Array\", \"value\": {\"type\": \"Object\", \"value\": {\"id\": {...}, \"name\": {...}}}}
- NEVER create an object with field names like \"[0].id\" - this is wrong!
- NEVER use generic \"Object\" types without specifying the exact fields inside
- ALWAYS specify the complete structure with all nested fields and their types
- For example, instead of {\"type\": \"Object\"}, use {\"type\": \"Object\", \"value\": {\"field1\": {\"type\": \"Primitive\", \"value\": \"String\"}, \"field2\": {\"type\": \"Array\", \"value\": {...}}}}

Example Range schema (for multiple records):
{
  \"name\": \"Schema\",
  \"descriptive_name\": \"User Profile Information\",
  \"key\": {\"range_field\": \"id\"},
  \"fields\": [\"id\", \"name\", \"age\"],
  \"field_topologies\": {
    \"id\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\", \"classifications\": [\"word\"]}},
    \"name\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\", \"classifications\": [\"name:person\", \"word\"]}},
    \"age\": {\"root\": {\"type\": \"Primitive\", \"value\": \"Number\", \"classifications\": [\"word\"]}}
  }
}

Example Single schema (for one global value):
{
  \"name\": \"Schema\",
  \"descriptive_name\": \"Global Counter Statistics\",
  \"fields\": [\"count\", \"total\"],
  \"field_topologies\": {
    \"count\": {\"root\": {\"type\": \"Primitive\", \"value\": \"Number\", \"classifications\": [\"word\"]}},
    \"total\": {\"root\": {\"type\": \"Primitive\", \"value\": \"Number\", \"classifications\": [\"word\"]}}
  }
}

Example with Arrays and Objects:
{
  \"name\": \"Schema\",
  \"descriptive_name\": \"Social Media Post\",
  \"key\": {\"range_field\": \"post_id\"},
  \"fields\": [\"post_id\", \"content\", \"hashtags\", \"media\"],
  \"field_topologies\": {
    \"post_id\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\", \"classifications\": [\"word\"]}},
    \"content\": {\"root\": {\"type\": \"Primitive\", \"value\": \"String\", \"classifications\": [\"word\"]}},
    \"hashtags\": {\"root\": {\"type\": \"Array\", \"value\": {\"type\": \"Primitive\", \"value\": \"String\", \"classifications\": [\"hashtag\", \"word\"]}}},
    \"media\": {\"root\": {\"type\": \"Array\", \"value\": {\"type\": \"Object\", \"value\": {\"url\": {\"type\": \"Primitive\", \"value\": \"String\", \"classifications\": [\"url\", \"word\"]}, \"type\": {\"type\": \"Primitive\", \"value\": \"String\", \"classifications\": [\"word\"]}}}}}
  }
}
"#;

/// Instructions appended to every prompt
const PROMPT_ACTIONS: &str = r#"Please analyze the sample data and either:
1. If existing schemas can handle this data EXACTLY (with perfect topology match), return their names in existing_schemas and provide mutation_mappers
2. If no existing schemas match PERFECTLY, create a new schema definition in new_schemas and provide mutation_mappers

IMPORTANT: Only recommend existing schemas if they have EXACTLY the same topology structure as the input data. If there are any differences in nested structures, array types, or field types, create a new schema instead.

CRITICAL: When in doubt, ALWAYS create a new schema rather than trying to match an existing one. It's better to have multiple schemas than to use an incorrect one.

FORCE NEW SCHEMA: For Twitter data with user_mentions arrays, ALWAYS create a new schema. Do not use existing schemas even if they seem to match.

CRITICAL RULES:
- If the original input was a JSON array (multiple objects), you MUST create a NEW Range schema with \"key\": {\"range_field\": \"unique_field\"}
- NEVER recommend a Single-type schema for array inputs - they will overwrite data
- When user provides an array, ignore any existing Single schemas and create a new Range schema with the \"key\" field
- NEVER use generic \"Object\" types - always specify the complete field structure with exact types and classifications
- ALWAYS provide complete topology definitions with all nested fields explicitly defined

The response must be valid JSON."#;

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "Invalid JSON".to_string())
}

fn pretty_json_or_empty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
}

/// OpenRouter API service
pub struct OpenRouterService {
    client: Client,
    config: OpenRouterConfig,
    max_retries: u32,
}

/// Request to OpenRouter API
#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

/// Message in OpenRouter request
#[derive(Debug, Serialize)]
struct OpenRouterMessage {
    role: String,
    content: String,
}

/// Response from OpenRouter API
#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
    usage: Option<OpenRouterUsage>,
}

/// Choice in OpenRouter response
#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterResponseMessage,
}

/// Response message from OpenRouter
#[derive(Debug, Deserialize)]
struct OpenRouterResponseMessage {
    content: String,
}

/// Usage information from OpenRouter
#[derive(Debug, Deserialize)]
struct OpenRouterUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

impl OpenRouterService {
    /// Create a new OpenRouter service
    pub fn new(
        config: OpenRouterConfig,
        timeout_seconds: u64,
        max_retries: u32,
    ) -> IngestionResult<Self> {
        config.validate()?;

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| {
                IngestionError::openrouter_error(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            config,
            max_retries,
        })
    }

    /// Get schema recommendation from AI
    pub async fn get_schema_recommendation(
        &self,
        sample_json: &Value,
        available_schemas: &Value,
    ) -> IngestionResult<AISchemaResponse> {
        // Create superset structure from all top-level elements
        let superset_structure = StructureAnalyzer::create_superset_structure(sample_json);
        
        // Get analysis statistics for logging
        let stats = StructureAnalyzer::get_analysis_stats(sample_json);
        
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Analyzed JSON structure: {} elements, {} unique fields",
            stats.total_elements,
            stats.unique_fields
        );
        
        if let Some(array) = sample_json.as_array() {
            if array.is_empty() {
                return Err(IngestionError::ai_response_validation_error(
                    "Cannot determine schema from empty JSON array".to_string()
                ));
            }
            
            log_feature!(
                LogFeature::Ingestion,
                info,
                "JSON data is an array with {} elements, created superset structure with {} fields",
                array.len(),
                stats.unique_fields
            );
            
            // Log field coverage information
            let common_fields = stats.get_common_fields();
            let partial_fields = stats.get_partial_fields();
            
            if !common_fields.is_empty() {
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "Common fields (100% coverage): {:?}",
                    common_fields
                );
            }
            
            if !partial_fields.is_empty() {
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "Partial fields (not in all elements): {:?}",
                    partial_fields
                );
            }
        }
        
        log_feature!(
            LogFeature::Ingestion,
            info,
            "=== SUPERSET STRUCTURE ANALYSIS ==="
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Raw JSON data structure (first 500 chars): {}",
            if let Ok(json_str) = serde_json::to_string_pretty(sample_json) {
                if json_str.len() > 500 {
                    format!("{}...[truncated]", &json_str[..500])
                } else {
                    json_str
                }
            } else {
                "Failed to serialize JSON".to_string()
            }
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Generated superset structure: {}",
            pretty_json(&superset_structure)
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "=== END SUPERSET STRUCTURE ANALYSIS ==="
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Available schemas: {}",
            pretty_json_or_empty(available_schemas)
        );

        let is_array_input = sample_json.is_array();
        let prompt = self.create_prompt(&superset_structure, available_schemas, is_array_input);

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Sending request to OpenRouter API with model: {}",
            self.config.model
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "AI Request Prompt (length: {} chars): {}",
            prompt.len(),
            if prompt.len() > 1000 {
                format!("{}...[truncated]", &prompt[..1000])
            } else {
                prompt.clone()
            }
        );

        let response = self.call_openrouter_api(&prompt).await?;

        self.parse_ai_response(&response)
    }

    /// Create the prompt for the AI
    fn create_prompt(&self, sample_json: &Value, available_schemas: &Value, is_array_input: bool) -> String {
        let array_note = if is_array_input {
            "\n\nIMPORTANT: The user provided a JSON ARRAY of multiple objects. You MUST create or use a Range schema with a range_key to store multiple entities."
        } else {
            ""
        };
        
        format!(
            "{header}\n\nSample JSON Data:\n{sample}\n\nAvailable Schemas:\n{schemas}{array_note}\n\n{actions}",
            header = PROMPT_HEADER,
            sample = pretty_json(sample_json),
            schemas = pretty_json_or_empty(available_schemas),
            array_note = array_note,
            actions = PROMPT_ACTIONS
        )
    }

    /// Call the OpenRouter API
    pub async fn call_openrouter_api(&self, prompt: &str) -> IngestionResult<String> {
        let request = OpenRouterRequest {
            model: self.config.model.clone(),
            messages: vec![OpenRouterMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: Some(4000),
            temperature: Some(0.1),
        };

        let mut last_error = None;

        for attempt in 1..=self.max_retries {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "OpenRouter API attempt {} of {}",
                attempt,
                self.max_retries
            );

            match self.make_api_request(&request).await {
                Ok(response) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        info,
                        "OpenRouter API call successful on attempt {}",
                        attempt
                    );
                    return Ok(response);
                }
                Err(e) => {
                    log_feature!(
                        LogFeature::Ingestion,
                        warn,
                        "OpenRouter API attempt {} failed: {}",
                        attempt,
                        e
                    );
                    last_error = Some(e);

                    if attempt < self.max_retries {
                        // Exponential backoff
                        let delay = Duration::from_secs(2_u64.pow(attempt - 1));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| IngestionError::openrouter_error("All API attempts failed")))
    }

    /// Make a single API request
    async fn make_api_request(&self, request: &OpenRouterRequest) -> IngestionResult<String> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/datafold/datafold")
            .header("X-Title", "DataFold Ingestion")
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(IngestionError::openrouter_error(format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
        }

        let openrouter_response: OpenRouterResponse = response.json().await?;

        if let Some(usage) = &openrouter_response.usage {
            log_feature!(
                LogFeature::Ingestion,
                info,
                "OpenRouter API usage - Prompt tokens: {:?}, Completion tokens: {:?}, Total tokens: {:?}",
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.total_tokens
            );
        }

        if openrouter_response.choices.is_empty() {
            return Err(IngestionError::openrouter_error(
                "No choices in API response",
            ));
        }

        Ok(openrouter_response.choices[0].message.content.clone())
    }

    /// Parse the AI response
    fn parse_ai_response(&self, response_text: &str) -> IngestionResult<AISchemaResponse> {

        // Try to extract JSON from the response
        let json_str = self.extract_json_from_response(response_text)?;
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Extracted JSON string: {}",
            json_str
        );

        // Parse the JSON
        let parsed: Value = serde_json::from_str(&json_str).map_err(|e| {
            IngestionError::ai_response_validation_error(format!(
                "Failed to parse AI response as JSON: {}. Response: {}",
                e, json_str
            ))
        })?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "Parsed JSON value: {}",
            pretty_json(&parsed)
        );

        // Validate and convert to AISchemaResponse
        let result = self.validate_and_convert_response(parsed)?;

        log_feature!(
            LogFeature::Ingestion,
            info,
            "=== FINAL PARSED AI RESPONSE ==="
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Existing schemas: {:?}",
            result.existing_schemas
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "New schemas: {}",
            result
                .new_schemas
                .as_ref()
                .map(pretty_json)
                .unwrap_or_else(|| "None".to_string())
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Mutation mappers: {:?}",
            result.mutation_mappers
        );
        log_feature!(
            LogFeature::Ingestion,
            info,
            "=== END PARSED AI RESPONSE ==="
        );

        Ok(result)
    }

    /// Extract JSON from the AI response text
    fn extract_json_from_response(&self, response_text: &str) -> IngestionResult<String> {
        // Look for JSON block markers
        if let Some(start) = response_text.find("```json") {
            let search_start = start + 7; // Length of "```json"
            if let Some(end_offset) = response_text[search_start..].find("```") {
                let json_end = search_start + end_offset;
                return Ok(response_text[search_start..json_end].trim().to_string());
            }
        }

        // Look for "Here's the response:" followed by JSON
        if let Some(start) = response_text.find("Here's the response:") {
            let json_start = response_text[start + 20..].trim(); // Skip "Here's the response:"
            if let Some(json_start_pos) = json_start.find('{') {
                let json_candidate = json_start[json_start_pos..].trim();
                if let Some(end) = json_candidate.rfind('}') {
                    let json_str = json_candidate[..=end].to_string();
                    if serde_json::from_str::<Value>(&json_str).is_ok() {
                        return Ok(json_str);
                    }
                }
            }
        }

        // Look for direct JSON (starts with { and ends with })
        if let Some(start) = response_text.find('{') {
            if let Some(end) = response_text.rfind('}') {
                if end > start {
                    let json_candidate = response_text[start..=end].to_string();
                    // Try to parse the JSON to validate it's complete
                    if serde_json::from_str::<Value>(&json_candidate).is_ok() {
                        return Ok(json_candidate);
                    }
                }
            }
        }

        // If no JSON found, try the entire response
        Ok(response_text.trim().to_string())
    }

    /// Validate that a schema has classifications on all primitive fields
    fn validate_schema_has_classifications(&self, schema_val: &Value) -> IngestionResult<()> {
        let schema_obj = schema_val.as_object().ok_or_else(|| {
            IngestionError::ai_response_validation_error("Schema must be a JSON object")
        })?;
        
        let schema_name = schema_obj.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        let field_topologies = schema_obj.get("field_topologies")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                IngestionError::ai_response_validation_error(
                    format!("Schema '{}' missing field_topologies", schema_name)
                )
            })?;
        
        // Check each field's topology for classifications
        for (field_name, topology_val) in field_topologies {
            let topology_obj = topology_val.as_object()
                .and_then(|obj| obj.get("root"))
                .and_then(|v| v.as_object())
                .ok_or_else(|| {
                    IngestionError::ai_response_validation_error(
                        format!("Schema '{}' field '{}' has invalid topology structure", schema_name, field_name)
                    )
                })?;
            
            Self::validate_topology_node_classifications(schema_name, field_name, topology_obj)?;
        }
        
        Ok(())
    }
    
    /// Recursively validate that primitive nodes have classifications
    fn validate_topology_node_classifications(
        schema_name: &str, 
        field_name: &str, 
        node: &serde_json::Map<String, Value>
    ) -> IngestionResult<()> {
        let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
        
        match node_type {
            "Primitive" => {
                // Check if classifications exist and is a non-empty array
                let classifications = node.get("classifications")
                    .and_then(|v| v.as_array());
                
                match classifications {
                    Some(arr) if !arr.is_empty() => Ok(()), // Valid
                    _ => Err(IngestionError::ai_response_validation_error(
                        format!(
                            "Schema '{}' field '{}' has a Primitive without classifications. \
                            AI must provide at least one classification (e.g., [\"word\"])",
                            schema_name, field_name
                        )
                    ))
                }
            }
            "Array" => {
                // Recurse into array value
                if let Some(value_obj) = node.get("value").and_then(|v| v.as_object()) {
                    Self::validate_topology_node_classifications(schema_name, field_name, value_obj)?;
                }
                Ok(())
            }
            "Object" => {
                // Recurse into object fields
                if let Some(value_obj) = node.get("value").and_then(|v| v.as_object()) {
                    for (_nested_field, nested_node) in value_obj {
                        if let Some(nested_obj) = nested_node.as_object() {
                            Self::validate_topology_node_classifications(schema_name, field_name, nested_obj)?;
                        }
                    }
                }
                Ok(())
            }
            _ => Ok(()) // Unknown type, skip validation
        }
    }
    
    /// Validate and convert the parsed response
    fn validate_and_convert_response(&self, parsed: Value) -> IngestionResult<AISchemaResponse> {
        let obj = parsed.as_object().ok_or_else(|| {
            IngestionError::ai_response_validation_error("Response must be a JSON object")
        })?;

        // Parse existing_schemas
        let existing_schemas = match obj.get("existing_schemas") {
            Some(Value::Array(arr)) => arr
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            Some(Value::String(s)) => vec![s.clone()],
            Some(Value::Null) | None => vec![],
            _ => {
                return Err(IngestionError::ai_response_validation_error(
                    "existing_schemas must be an array of strings or null",
                ))
            }
        };

        // Parse new_schemas
        let new_schemas = obj.get("new_schemas").cloned();
        
        // Validate that new schemas have classifications on all primitive fields
        if let Some(schema_val) = &new_schemas {
            match schema_val {
                Value::Array(schemas) => {
                    for schema in schemas {
                        self.validate_schema_has_classifications(schema)?;
                    }
                }
                Value::Object(_) => {
                    // Single schema object (expected format)
                    self.validate_schema_has_classifications(schema_val)?;
                }
                _ => {}
            }
        }

        // Parse mutation_mappers
        let mutation_mappers = match obj.get("mutation_mappers") {
            Some(Value::Object(map)) => {
                let mut result = std::collections::HashMap::new();
                for (key, value) in map {
                    if let Some(value_str) = value.as_str() {
                        result.insert(key.clone(), value_str.to_string());
                    }
                }
                result
            }
            Some(Value::Null) | None => std::collections::HashMap::new(),
            _ => {
                return Err(IngestionError::ai_response_validation_error(
                    "mutation_mappers must be an object with string values",
                ))
            }
        };

        Ok(AISchemaResponse {
            existing_schemas,
            new_schemas,
            mutation_mappers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_response() {
        let service = create_test_service();

        // Test with JSON block markers
        let response_with_markers = r###"Here's the analysis:
```json
{"existing_schemas": ["test"], "new_schemas": null, "mutation_mappers": {}}
```
That should work."###;

        let result = service.extract_json_from_response(response_with_markers);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("existing_schemas"));

        // Test with direct JSON
        let response_direct =
            r###"{"existing_schemas": ["test"], "new_schemas": null, "mutation_mappers": {}}"###;
        let result = service.extract_json_from_response(response_direct);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_and_convert_response() {
        let service = create_test_service();

        let test_json = serde_json::json!({
            "existing_schemas": ["schema1", "schema2"],
            "new_schemas": null,
            "mutation_mappers": {
                "field1": "schema.field1",
                "nested.field": "schema.nested_field"
            }
        });

        let result = service.validate_and_convert_response(test_json);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.existing_schemas.len(), 2);
        assert_eq!(response.mutation_mappers.len(), 2);
    }

    #[test]
    fn test_create_prompt_includes_sample_and_schemas() {
        let service = create_test_service();
        let sample = serde_json::json!({"a": 1});
        let schemas = serde_json::json!({"test": {"field": "string"}});

        let prompt = service.create_prompt(&sample, &schemas, false);
        assert!(prompt.contains("Sample JSON Data:"));
        assert!(prompt.contains("\"a\": 1"));
        assert!(prompt.contains("Available Schemas:"));
        assert!(prompt.contains("\"test\""));
        assert!(prompt.contains(PROMPT_HEADER));
        assert!(prompt.contains(PROMPT_ACTIONS));
    }

    #[test]
    fn test_pretty_json_helpers() {
        let value = serde_json::json!({"x": 1});
        assert!(pretty_json(&value).contains("\"x\": 1"));
        assert!(pretty_json_or_empty(&value).contains("\"x\": 1"));
    }

    fn create_test_service() -> OpenRouterService {
        let config = OpenRouterConfig {
            api_key: "test-key".to_string(),
            ..Default::default()
        };
        OpenRouterService::new(config, 10, 3).unwrap()
    }
}
