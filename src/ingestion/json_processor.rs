//! JSON conversion and processing for file uploads

use file_to_json::{Converter, FallbackStrategy, OpenRouterConfig};
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::NamedTempFile;

use crate::ingestion::config::AIProvider;
use crate::ingestion::IngestionError;
use crate::log_feature;
use crate::logging::features::LogFeature;

/// Convert a file to JSON using file_to_json library (core implementation)
async fn convert_file_to_json_core(file_path: &PathBuf) -> Result<Value, IngestionError> {
    log_feature!(
        LogFeature::Ingestion,
        info,
        "Converting file to JSON: {:?}",
        file_path
    );

    // Load fold_db ingestion config
    let ingestion_config = crate::ingestion::IngestionConfig::from_env()?;

    // Only OpenRouter is supported for file_to_json conversion
    if ingestion_config.provider != AIProvider::OpenRouter {
        return Err(IngestionError::configuration_error(
            "File conversion requires OpenRouter provider. Ollama is not supported for this feature."
        ));
    }

    // Build file_to_json OpenRouterConfig from fold_db config
    let file_to_json_config = OpenRouterConfig {
        api_key: ingestion_config.openrouter.api_key.clone(),
        model: ingestion_config.openrouter.model.clone(),
        timeout: Duration::from_secs(ingestion_config.timeout_seconds),
        fallback_strategy: FallbackStrategy::Chunked,
        vision_model: Some(ingestion_config.openrouter.model.clone()),
        max_image_bytes: 5 * 1024 * 1024, // 5MB default
    };

    let file_path_str = file_path.to_string_lossy().to_string();

    // Run conversion in blocking task
    tokio::task::spawn_blocking(move || {
        let converter = Converter::new(file_to_json_config)
            .map_err(|_| IngestionError::FileConversionFailed)?;
        converter.convert_path(&file_path_str).map_err(|e| {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "Failed to convert file to JSON: {}",
                e
            );
            IngestionError::FileConversionFailed
        })
    })
    .await
    .map_err(|e| {
        log_feature!(
            LogFeature::Ingestion,
            error,
            "Failed to spawn blocking task: {}",
            e
        );
        IngestionError::FileConversionFailed
    })?
}

/// Convert a file to JSON using file_to_json library (public API for ingestion)
pub async fn convert_file_to_json(file_path: &PathBuf) -> Result<Value, IngestionError> {
    convert_file_to_json_core(file_path).await
}

/// Convert a file to JSON using file_to_json library (actix-web wrapper)
pub async fn convert_file_to_json_http(
    file_path: &PathBuf,
) -> Result<Value, actix_web::HttpResponse> {
    use actix_web::HttpResponse;

    match convert_file_to_json_core(file_path).await {
        Ok(value) => Ok(value),
        Err(e) => {
            log_feature!(
                LogFeature::Ingestion,
                error,
                "File conversion failed: {}",
                e
            );
            Err(HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to convert file to JSON: {}", e)
            })))
        }
    }
}

/// Flatten JSON structures with unnecessary root layers
/// Handles patterns:
/// 1. root -> array: {"key": [...]} => [...]
/// 2. root -> root -> array: {"key1": {"key2": [...]}} => [...]
/// 3. array elements with single-field wrappers: [{"wrapper": {...}}] => [{...}]
/// 4. direct arrays with single-field wrappers: [...] => [...]
pub fn flatten_root_layers(json: Value) -> Value {
    // Check if it's already an array - flatten its elements
    if json.is_array() {
        log_feature!(
            LogFeature::Ingestion,
            info,
            "Flattening array elements with single-field wrappers"
        );
        return flatten_array_elements(json);
    }

    // Check for root -> array pattern
    if let Value::Object(ref map) = json {
        // If object has exactly one field
        if map.len() == 1 {
            let (key, value) = map.iter().next().unwrap();

            // If that field is an array, flatten the array and its elements
            if value.is_array() {
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "Flattening root->array pattern: removing '{}' wrapper",
                    key
                );
                return flatten_array_elements(value.clone());
            }

            // Check for root -> root -> array pattern
            if let Value::Object(ref inner_map) = value {
                if inner_map.len() == 1 {
                    let (inner_key, inner_value) = inner_map.iter().next().unwrap();
                    if inner_value.is_array() {
                        log_feature!(
                            LogFeature::Ingestion,
                            info,
                            "Flattening root->root->array pattern: removing '{}'->'{}' wrappers",
                            key,
                            inner_key
                        );
                        return flatten_array_elements(inner_value.clone());
                    }
                }
            }
        }
    }

    // No flattening needed
    json
}

/// Flatten array elements that have unnecessary single-field wrapper objects
fn flatten_array_elements(value: Value) -> Value {
    if let Value::Array(arr) = value {
        let flattened_elements: Vec<Value> = arr
            .into_iter()
            .map(|element| {
                // If element is an object with exactly one field
                if let Value::Object(ref map) = element {
                    if map.len() == 1 {
                        let (key, inner_value) = map.iter().next().unwrap();

                        // If that field contains an object (not an array or primitive),
                        // flatten by returning the inner object
                        if inner_value.is_object() {
                            log_feature!(
                                LogFeature::Ingestion,
                                debug,
                                "Flattening array element: removing '{}' wrapper from object",
                                key
                            );
                            return inner_value.clone();
                        }
                    }
                }
                element
            })
            .collect();

        Value::Array(flattened_elements)
    } else {
        value
    }
}

/// Add file_location metadata to JSON value
pub fn add_file_location(json: Value, file_path: &std::path::Path) -> Value {
    match json {
        Value::Object(mut map) => {
            // Add file_location directly to the object
            map.insert(
                "file_location".to_string(),
                Value::String(file_path.to_string_lossy().to_string()),
            );
            Value::Object(map)
        }
        Value::Array(arr) => {
            // Add file_location to each element in the array
            let modified_array: Vec<Value> = arr
                .into_iter()
                .map(|mut item| {
                    if let Value::Object(ref mut obj) = item {
                        obj.insert(
                            "file_location".to_string(),
                            Value::String(file_path.to_string_lossy().to_string()),
                        );
                    }
                    item
                })
                .collect();
            Value::Array(modified_array)
        }
        other => {
            // For primitives, wrap in a minimal object with file_location
            json!({
                "file_location": file_path.to_string_lossy().to_string(),
                "value": other
            })
        }
    }
}

/// Save JSON to a temporary file that persists for testing
/// Returns the path to the temporary file
pub fn save_json_to_temp_file(json: &Value) -> std::io::Result<String> {
    // Create temp directory in system temp location (works in Lambda and locally)
    let temp_dir = std::env::temp_dir().join("folddb_debug");
    std::fs::create_dir_all(&temp_dir)?;

    // Create a named temporary file with .json extension
    let temp_file = NamedTempFile::new_in(&temp_dir)?;

    // Write the JSON with pretty formatting
    let json_string = serde_json::to_string_pretty(json)?;

    // Get a mutable handle to write
    let mut file = temp_file.as_file();
    file.write_all(json_string.as_bytes())?;
    file.sync_all()?;

    // Persist the temp file so it doesn't get deleted when dropped
    let (_file, path) = temp_file.keep()?;

    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatten_root_to_array() {
        let input = json!({
            "data": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ]
        });

        let result = flatten_root_layers(input);

        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["id"], 1);
    }

    #[test]
    fn test_flatten_root_root_to_array() {
        let input = json!({
            "response": {
                "items": [
                    {"id": 1, "name": "Alice"},
                    {"id": 2, "name": "Bob"}
                ]
            }
        });

        let result = flatten_root_layers(input);

        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["name"], "Alice");
    }

    #[test]
    fn test_no_flatten_multiple_fields() {
        let input = json!({
            "data": [{"id": 1}],
            "metadata": {"count": 1}
        });

        let result = flatten_root_layers(input.clone());

        // Should remain unchanged
        assert_eq!(result, input);
    }

    #[test]
    fn test_no_flatten_nested_object() {
        let input = json!({
            "user": {
                "id": 1,
                "name": "Alice"
            }
        });

        let result = flatten_root_layers(input.clone());

        // Should remain unchanged
        assert_eq!(result, input);
    }

    #[test]
    fn test_no_flatten_direct_array() {
        let input = json!([
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]);

        let result = flatten_root_layers(input.clone());

        // Should remain unchanged
        assert_eq!(result, input);
    }

    #[test]
    fn test_no_flatten_deep_nesting() {
        let input = json!({
            "level1": {
                "level2": {
                    "level3": [{"id": 1}]
                }
            }
        });

        let result = flatten_root_layers(input.clone());

        // Should remain unchanged (we only flatten up to 2 levels)
        assert_eq!(result, input);
    }

    #[test]
    fn test_flatten_with_array_keeps_array_structure() {
        let input = json!({
            "data": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ]
        });

        let result = flatten_root_layers(input);

        // Verify it's an array, not wrapped in an object
        assert!(result.is_array(), "Result should be an array");
        assert!(
            !result.is_object(),
            "Result should not be wrapped in an object"
        );

        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_add_file_location_to_object() {
        let input = json!({"id": 1, "name": "Alice"});
        let path = PathBuf::from("/test/file.csv");

        let result = add_file_location(input, &path);

        assert!(result.is_object());
        let obj = result.as_object().unwrap();
        assert_eq!(obj["file_location"], "/test/file.csv");
        assert_eq!(obj["id"], 1);
    }

    #[test]
    fn test_add_file_location_to_array() {
        let input = json!([
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]);
        let path = PathBuf::from("/test/file.csv");

        let result = add_file_location(input, &path);

        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["file_location"], "/test/file.csv");
        assert_eq!(arr[1]["file_location"], "/test/file.csv");
    }

    #[test]
    fn test_flatten_array_elements_with_single_field_wrappers() {
        let input = json!({
            "data": [
                {"item": {"id": 1, "name": "Alice"}},
                {"item": {"id": 2, "name": "Bob"}}
            ]
        });

        let result = flatten_root_layers(input);

        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);

        // Each array element should be flattened (no "item" wrapper)
        assert_eq!(arr[0]["id"], 1);
        assert_eq!(arr[0]["name"], "Alice");
        assert!(arr[0].get("item").is_none());

        assert_eq!(arr[1]["id"], 2);
        assert_eq!(arr[1]["name"], "Bob");
        assert!(arr[1].get("item").is_none());
    }

    #[test]
    fn test_flatten_array_elements_preserves_multi_field_objects() {
        let input = json!({
            "data": [
                {
                    "id": 1,
                    "wrapper": {"name": "Alice"}
                },
                {
                    "id": 2,
                    "wrapper": {"name": "Bob"}
                }
            ]
        });

        let result = flatten_root_layers(input.clone());

        // Should flatten root but NOT array elements (they have multiple fields)
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["id"], 1);
        assert!(arr[0].get("wrapper").is_some());
    }

    #[test]
    fn test_flatten_array_elements_preserves_primitives() {
        let input = json!({
            "data": [
                {"value": "Alice"},
                {"value": 42},
                {"value": true}
            ]
        });

        let result = flatten_root_layers(input);

        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 3);

        // Should NOT flatten when the inner value is a primitive
        assert_eq!(arr[0]["value"], "Alice");
        assert_eq!(arr[1]["value"], 42);
        assert_eq!(arr[2]["value"], true);
    }

    #[test]
    fn test_flatten_complex_nested_structure() {
        let input = json!({
            "response": {
                "items": [
                    {"record": {"id": 1, "name": "Alice", "email": "alice@example.com"}},
                    {"record": {"id": 2, "name": "Bob", "email": "bob@example.com"}}
                ]
            }
        });

        let result = flatten_root_layers(input);

        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);

        // Should flatten both root layers AND array element wrappers
        assert_eq!(arr[0]["id"], 1);
        assert_eq!(arr[0]["name"], "Alice");
        assert!(arr[0].get("record").is_none());

        assert_eq!(arr[1]["id"], 2);
        assert_eq!(arr[1]["name"], "Bob");
        assert!(arr[1].get("record").is_none());
    }

    #[test]
    fn test_flatten_direct_array_with_single_field_wrappers() {
        // Test case for arrays returned directly by file_to_json
        let input = json!([
            {"tweet": {"id": 1, "text": "Hello", "user": "alice"}},
            {"tweet": {"id": 2, "text": "World", "user": "bob"}}
        ]);

        let result = flatten_root_layers(input);

        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);

        // Should flatten the "tweet" wrapper from each element
        assert_eq!(arr[0]["id"], 1);
        assert_eq!(arr[0]["text"], "Hello");
        assert_eq!(arr[0]["user"], "alice");
        assert!(arr[0].get("tweet").is_none());

        assert_eq!(arr[1]["id"], 2);
        assert_eq!(arr[1]["text"], "World");
        assert_eq!(arr[1]["user"], "bob");
        assert!(arr[1].get("tweet").is_none());
    }
}
