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

/// Ensure the JSON object produced by the vision model contains `image_type` and
/// `created_at` fields.  Existing values are preserved so the model's own output
/// is respected when present.
pub fn enrich_image_json(json: &mut Value, file_path: &std::path::PathBuf, source_file_name: Option<&str>) {
    if let Value::Object(map) = json {
        // image_type — keep if already set
        if !map.contains_key("image_type") {
            let image_type = classify_image_type(source_file_name.unwrap_or(""));
            map.insert("image_type".to_string(), Value::String(image_type));
        }
        // created_at — keep if already set
        if !map.contains_key("created_at") {
            let created_at = get_file_creation_date(file_path);
            map.insert("created_at".to_string(), Value::String(created_at));
        }
    }
}

/// Heuristic classification of an image based on the source filename.
///
/// - "screenshot" if the filename contains "screenshot"
/// - "diagram" for SVG files or filenames containing "chart" or "diagram"
/// - "photo" otherwise (default)
pub fn classify_image_type(source_file_name: &str) -> String {
    let lower = source_file_name.to_lowercase();
    if lower.contains("screenshot") {
        "screenshot".to_string()
    } else if lower.ends_with(".svg") || lower.contains("chart") || lower.contains("diagram") {
        "diagram".to_string()
    } else {
        "photo".to_string()
    }
}

/// Read the file's modified (or created) time and return it as an ISO 8601 string.
/// Falls back to `Utc::now()` if the metadata cannot be read.
pub fn get_file_creation_date(file_path: &std::path::PathBuf) -> String {
    std::fs::metadata(file_path)
        .ok()
        .and_then(|meta| meta.modified().ok().or_else(|| meta.created().ok()))
        .map(|time| {
            let dt: chrono::DateTime<chrono::Utc> = time.into();
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
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

    #[test]
    fn test_classify_image_type_photo() {
        assert_eq!(classify_image_type("vacation.jpg"), "photo");
        assert_eq!(classify_image_type("IMG_1234.PNG"), "photo");
        assert_eq!(classify_image_type(""), "photo");
    }

    #[test]
    fn test_classify_image_type_screenshot() {
        assert_eq!(classify_image_type("Screenshot_2024-01-01.png"), "screenshot");
        assert_eq!(classify_image_type("my_screenshot.jpg"), "screenshot");
    }

    #[test]
    fn test_classify_image_type_diagram() {
        assert_eq!(classify_image_type("architecture.svg"), "diagram");
        assert_eq!(classify_image_type("sales_chart.png"), "diagram");
        assert_eq!(classify_image_type("system_diagram.jpg"), "diagram");
    }

    #[test]
    fn test_enrich_image_json_adds_fields() {
        let mut json = json!({"description": "A sunset"});
        let path = std::path::PathBuf::from("/tmp/test.jpg");
        enrich_image_json(&mut json, &path, Some("test.jpg"));

        assert_eq!(json["image_type"], "photo");
        assert!(json.get("created_at").is_some());
    }

    #[test]
    fn test_enrich_image_json_preserves_existing() {
        let mut json = json!({
            "description": "A sunset",
            "image_type": "landscape",
            "created_at": "2024-06-15 10:00:00"
        });
        let path = std::path::PathBuf::from("/tmp/test.jpg");
        enrich_image_json(&mut json, &path, Some("test.jpg"));

        // Should NOT overwrite existing values
        assert_eq!(json["image_type"], "landscape");
        assert_eq!(json["created_at"], "2024-06-15 10:00:00");
    }

    #[test]
    fn test_enrich_image_json_noop_for_non_object() {
        let mut json = json!([1, 2, 3]);
        let path = std::path::PathBuf::from("/tmp/test.jpg");
        enrich_image_json(&mut json, &path, Some("test.jpg"));
        // Should remain unchanged
        assert!(json.is_array());
    }
}
