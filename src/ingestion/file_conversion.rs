//! File conversion utilities — CSV, Twitter JS, and unified file reading.

use crate::ingestion::error::IngestionError;
use crate::ingestion::IngestionResult;
use serde_json::Value;
use std::path::Path;

/// Convert CSV content to JSON array
pub fn csv_to_json(csv_content: &str) -> IngestionResult<String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_content.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| IngestionError::InvalidInput(format!("Failed to read CSV headers: {}", e)))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut records: Vec<Value> = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| {
            IngestionError::InvalidInput(format!("Failed to read CSV record: {}", e))
        })?;
        let mut obj = serde_json::Map::new();

        for (i, field) in record.iter().enumerate() {
            if let Some(header) = headers.get(i) {
                let value = if let Ok(n) = field.parse::<f64>() {
                    Value::Number(
                        serde_json::Number::from_f64(n)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    )
                } else if field == "true" {
                    Value::Bool(true)
                } else if field == "false" {
                    Value::Bool(false)
                } else {
                    Value::String(field.to_string())
                };
                obj.insert(header.clone(), value);
            }
        }

        records.push(Value::Object(obj));
    }

    serde_json::to_string(&records)
        .map_err(|e| IngestionError::InvalidInput(format!("Failed to serialize JSON: {}", e)))
}

/// Convert a Twitter data export `.js` file to JSON.
///
/// Twitter data exports use files like `window.YTD.tweet.part0 = [...]`.
/// This strips the variable assignment prefix and returns the pure JSON.
pub fn twitter_js_to_json(content: &str) -> IngestionResult<String> {
    if let Some(eq_pos) = content.find('=') {
        let json_part = content[eq_pos + 1..].trim();
        // Validate it parses as JSON
        serde_json::from_str::<Value>(json_part).map_err(|e| {
            IngestionError::InvalidInput(format!("Invalid JSON in .js file: {}", e))
        })?;
        Ok(json_part.to_string())
    } else {
        Err(IngestionError::InvalidInput(
            "Not a Twitter data export .js file (no '=' found)".to_string(),
        ))
    }
}

/// Read a file and convert it to a JSON Value regardless of format.
///
/// Supported extensions: `.json`, `.js` (Twitter export), `.csv`, `.txt`, `.md`
pub fn read_file_as_json(file_path: &Path) -> IngestionResult<Value> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| IngestionError::InvalidInput(format!("Failed to read file: {}", e)))?;

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let json_string = match ext.as_str() {
        "json" => content,
        "js" => twitter_js_to_json(&content)?,
        "csv" => csv_to_json(&content)?,
        "txt" | "md" => {
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            serde_json::to_string(&serde_json::json!({
                "content": content,
                "source_file": file_name,
                "file_type": ext
            }))
            .map_err(|e| {
                IngestionError::InvalidInput(format!("Failed to wrap text content: {}", e))
            })?
        }
        _ => {
            return Err(IngestionError::InvalidInput(format!(
                "Unsupported file type: {}",
                ext
            )))
        }
    };

    serde_json::from_str(&json_string)
        .map_err(|e| IngestionError::InvalidInput(format!("Failed to parse JSON: {}", e)))
}

/// Read a file, compute its SHA256 hash, and convert to JSON.
/// Returns `(json_value, sha256_hex_hash)`.
pub fn read_file_with_hash(file_path: &Path) -> IngestionResult<(Value, String, Vec<u8>)> {
    use sha2::{Digest, Sha256};

    let raw_bytes = std::fs::read(file_path)
        .map_err(|e| IngestionError::InvalidInput(format!("Failed to read file: {}", e)))?;

    let hash_hex = format!("{:x}", Sha256::digest(&raw_bytes));

    let content = std::str::from_utf8(&raw_bytes)
        .map_err(|e| IngestionError::InvalidInput(format!("File is not valid UTF-8: {}", e)))?;

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let json_string: std::borrow::Cow<'_, str> = match ext.as_str() {
        "json" => std::borrow::Cow::Borrowed(content),
        "js" => std::borrow::Cow::Owned(twitter_js_to_json(content)?),
        "csv" => std::borrow::Cow::Owned(csv_to_json(content)?),
        "txt" | "md" => {
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            std::borrow::Cow::Owned(
                serde_json::to_string(&serde_json::json!({
                    "content": content,
                    "source_file": file_name,
                    "file_type": ext
                }))
                .map_err(|e| {
                    IngestionError::InvalidInput(format!("Failed to wrap text content: {}", e))
                })?,
            )
        }
        _ => {
            return Err(IngestionError::InvalidInput(format!(
                "Unsupported file type: {}",
                ext
            )))
        }
    };

    let value = serde_json::from_str(&json_string)
        .map_err(|e| IngestionError::InvalidInput(format!("Failed to parse JSON: {}", e)))?;

    Ok((value, hash_hex, raw_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::Write;

    #[test]
    fn test_read_file_with_hash_json() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".json")
            .tempfile()
            .unwrap();
        let json_content = r#"{"name": "Alice", "age": 30}"#;
        write!(tmp, "{}", json_content).unwrap();

        let (value, hash, raw) = read_file_with_hash(tmp.path()).unwrap();
        assert_eq!(value["name"], "Alice");
        assert_eq!(value["age"], 30);

        let expected_hash = format!("{:x}", Sha256::digest(json_content.as_bytes()));
        assert_eq!(hash, expected_hash);
        assert_eq!(raw, json_content.as_bytes());
    }

    #[test]
    fn test_read_file_with_hash_twitter_js() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".js")
            .tempfile()
            .unwrap();
        let content = r#"window.YTD.tweet.part0 = [{"id": "123", "text": "hello"}]"#;
        write!(tmp, "{}", content).unwrap();

        let (value, hash, _raw) = read_file_with_hash(tmp.path()).unwrap();
        let arr = value.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "123");

        let expected_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        assert_eq!(hash, expected_hash);
    }

    #[test]
    fn test_read_file_with_hash_csv() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".csv")
            .tempfile()
            .unwrap();
        let content = "name,age\nAlice,30\nBob,25\n";
        write!(tmp, "{}", content).unwrap();

        let (value, hash, _raw) = read_file_with_hash(tmp.path()).unwrap();
        let arr = value.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["name"], "Alice");

        let expected_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        assert_eq!(hash, expected_hash);
    }

    #[test]
    fn test_read_file_with_hash_txt() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".txt")
            .tempfile()
            .unwrap();
        let content = "Hello, this is a text file.";
        write!(tmp, "{}", content).unwrap();

        let (value, hash, _raw) = read_file_with_hash(tmp.path()).unwrap();
        assert_eq!(value["content"], content);
        assert_eq!(value["file_type"], "txt");
        assert!(value["source_file"].as_str().unwrap().ends_with(".txt"));

        let expected_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        assert_eq!(hash, expected_hash);
    }

    #[test]
    fn test_read_file_with_hash_md() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".md")
            .tempfile()
            .unwrap();
        let content = "# Heading\n\nSome markdown content.";
        write!(tmp, "{}", content).unwrap();

        let (value, hash, _raw) = read_file_with_hash(tmp.path()).unwrap();
        assert_eq!(value["content"], content);
        assert_eq!(value["file_type"], "md");
        assert!(value["source_file"].as_str().unwrap().ends_with(".md"));

        let expected_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        assert_eq!(hash, expected_hash);
    }

    #[test]
    fn test_read_file_with_hash_unsupported_extension() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".xyz")
            .tempfile()
            .unwrap();
        write!(tmp, "some content").unwrap();

        let result = read_file_with_hash(tmp.path());
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Unsupported file type"));
    }

    #[test]
    fn test_read_file_with_hash_nonexistent_file() {
        let result = read_file_with_hash(Path::new("/tmp/nonexistent_file_abc123.json"));
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Failed to read file"));
    }
}
