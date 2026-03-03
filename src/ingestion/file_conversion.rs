//! File conversion utilities — CSV, Twitter JS, code metadata extraction, and unified file reading.

use crate::ingestion::error::IngestionError;
use crate::ingestion::IngestionResult;
use regex::Regex;
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

/// Code file extensions handled by `extract_code_metadata`.
const CODE_EXTS: &[&str] = &[
    "js", "jsx", "ts", "tsx", "py", "rs", "go", "java", "kt", "rb",
    "c", "cpp", "h", "hpp", "cs", "swift", "scala", "lua", "r", "pl",
    "sh", "bash", "zsh",
];

/// Config file extensions wrapped as text content.
const CONFIG_EXTS: &[&str] = &["yaml", "yml", "toml", "xml"];

/// Returns true if `ext` (lowercase, no dot) is a code file extension.
fn is_code_ext(ext: &str) -> bool {
    CODE_EXTS.contains(&ext)
}

/// Returns true if `ext` (lowercase, no dot) is a config file extension.
fn is_config_ext(ext: &str) -> bool {
    CONFIG_EXTS.contains(&ext)
}

/// Extract structural metadata from a source code file using regex.
///
/// Returns a JSON string with function/method declarations, class/struct
/// declarations, and comments found in the source.
pub fn extract_code_metadata(content: &str, file_name: &str, ext: &str) -> IngestionResult<String> {
    let fn_re = Regex::new(
        r"(?m)^\s*(?:pub\s+)?(?:async\s+)?(?:fn|def|function|func|sub)\s+\w+",
    )
    .unwrap();
    let class_re = Regex::new(
        r"(?m)^\s*(?:pub\s+)?(?:class|struct|trait|enum|interface)\s+\w+",
    )
    .unwrap();
    let comment_re = Regex::new(
        r"(?m)^\s*(?://[/!]?.*|#[^!].*|#$)",
    )
    .unwrap();

    let functions: Vec<String> = fn_re
        .find_iter(content)
        .map(|m| m.as_str().trim().to_string())
        .collect();

    let classes: Vec<String> = class_re
        .find_iter(content)
        .map(|m| m.as_str().trim().to_string())
        .collect();

    let comments: Vec<String> = comment_re
        .find_iter(content)
        .map(|m| m.as_str().trim().to_string())
        .collect();

    serde_json::to_string(&serde_json::json!({
        "source_file": file_name,
        "file_type": ext,
        "functions": functions,
        "classes": classes,
        "comments": comments,
    }))
    .map_err(|e| IngestionError::InvalidInput(format!("Failed to serialize code metadata: {}", e)))
}

/// Wrap plain-text content (`.txt`, `.md`, config files) as JSON.
fn wrap_text_content(content: &str, file_name: &str, ext: &str) -> IngestionResult<String> {
    serde_json::to_string(&serde_json::json!({
        "content": content,
        "source_file": file_name,
        "file_type": ext
    }))
    .map_err(|e| IngestionError::InvalidInput(format!("Failed to wrap text content: {}", e)))
}

/// Convert a `.js` file to JSON — tries Twitter export format first,
/// falls back to code metadata extraction.
fn js_to_json(content: &str, file_name: &str) -> IngestionResult<String> {
    match twitter_js_to_json(content) {
        Ok(json) => Ok(json),
        Err(_) => extract_code_metadata(content, file_name, "js"),
    }
}

/// Read a file and convert it to a JSON Value regardless of format.
///
/// Supported extensions: `.json`, `.js` (Twitter export or code), `.csv`,
/// `.txt`, `.md`, code files, and config files (`.yaml`, `.yml`, `.toml`, `.xml`).
pub fn read_file_as_json(file_path: &Path) -> IngestionResult<Value> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| IngestionError::InvalidInput(format!("Failed to read file: {}", e)))?;

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let json_string = match ext.as_str() {
        "json" => content,
        "js" => js_to_json(&content, file_name)?,
        "csv" => csv_to_json(&content)?,
        "txt" | "md" => wrap_text_content(&content, file_name, &ext)?,
        e if is_code_ext(e) => extract_code_metadata(&content, file_name, e)?,
        e if is_config_ext(e) => wrap_text_content(&content, file_name, e)?,
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

    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let json_string: std::borrow::Cow<'_, str> = match ext.as_str() {
        "json" => std::borrow::Cow::Borrowed(content),
        "js" => std::borrow::Cow::Owned(js_to_json(content, file_name)?),
        "csv" => std::borrow::Cow::Owned(csv_to_json(content)?),
        "txt" | "md" => std::borrow::Cow::Owned(wrap_text_content(content, file_name, &ext)?),
        e if is_code_ext(e) => {
            std::borrow::Cow::Owned(extract_code_metadata(content, file_name, e)?)
        }
        e if is_config_ext(e) => {
            std::borrow::Cow::Owned(wrap_text_content(content, file_name, e)?)
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

    // ---- extract_code_metadata tests ----

    #[test]
    fn test_extract_code_metadata_python() {
        let content = r#"# Helper utilities
class MyClass:
    def __init__(self):
        pass

def foo(x, y):
    # compute sum
    return x + y

async def bar():
    pass
"#;
        let json = extract_code_metadata(content, "example.py", "py").unwrap();
        let val: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["source_file"], "example.py");
        assert_eq!(val["file_type"], "py");

        let functions: Vec<&str> = val["functions"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert!(functions.iter().any(|f| f.contains("def foo")));
        assert!(functions.iter().any(|f| f.contains("def bar")));
        assert!(functions.iter().any(|f| f.contains("def __init__")));

        let classes: Vec<&str> = val["classes"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert!(classes.iter().any(|c| c.contains("class MyClass")));

        let comments: Vec<&str> = val["comments"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert!(comments.iter().any(|c| c.contains("Helper utilities")));
        assert!(comments.iter().any(|c| c.contains("compute sum")));
    }

    #[test]
    fn test_extract_code_metadata_rust() {
        let content = r#"/// A greeter struct
pub struct Greeter {
    name: String,
}

// private helper
fn helper() -> bool {
    true
}

pub async fn greet(name: &str) {
    println!("Hello, {}", name);
}

enum Color {
    Red,
    Blue,
}
"#;
        let json = extract_code_metadata(content, "lib.rs", "rs").unwrap();
        let val: Value = serde_json::from_str(&json).unwrap();

        let functions: Vec<&str> = val["functions"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert!(functions.iter().any(|f| f.contains("fn helper")));
        assert!(functions.iter().any(|f| f.contains("pub async fn greet")));

        let classes: Vec<&str> = val["classes"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert!(classes.iter().any(|c| c.contains("pub struct Greeter")));
        assert!(classes.iter().any(|c| c.contains("enum Color")));

        let comments: Vec<&str> = val["comments"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert!(comments.iter().any(|c| c.contains("A greeter struct")));
        assert!(comments.iter().any(|c| c.contains("private helper")));
    }

    #[test]
    fn test_extract_code_metadata_javascript() {
        let content = r#"// Main entry point
function greet(name) {
    console.log("Hello " + name);
}

class App {
    constructor() {}
}
"#;
        let json = extract_code_metadata(content, "app.js", "js").unwrap();
        let val: Value = serde_json::from_str(&json).unwrap();

        let functions: Vec<&str> = val["functions"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert!(functions.iter().any(|f| f.contains("function greet")));

        let classes: Vec<&str> = val["classes"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert!(classes.iter().any(|c| c.contains("class App")));
    }

    #[test]
    fn test_extract_code_metadata_go() {
        let content = r#"// Package main
package main

type Server struct {
    Port int
}

func main() {
    fmt.Println("hello")
}

func (s *Server) Start() {
}
"#;
        let json = extract_code_metadata(content, "main.go", "go").unwrap();
        let val: Value = serde_json::from_str(&json).unwrap();

        let functions: Vec<&str> = val["functions"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert!(functions.iter().any(|f| f.contains("func main")));

        let comments: Vec<&str> = val["comments"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert!(comments.iter().any(|c| c.contains("Package main")));
    }

    #[test]
    fn test_js_twitter_format_takes_priority() {
        let content = r#"window.YTD.tweet.part0 = [{"id": "1"}]"#;
        let json = js_to_json(content, "tweets.js").unwrap();
        let val: Value = serde_json::from_str(&json).unwrap();
        // Twitter format returns an array
        assert!(val.is_array());
        assert_eq!(val[0]["id"], "1");
    }

    #[test]
    fn test_js_non_twitter_falls_back_to_code() {
        let content = "function hello() { return 42; }\n";
        let json = js_to_json(content, "app.js").unwrap();
        let val: Value = serde_json::from_str(&json).unwrap();
        // Falls back to code metadata
        assert_eq!(val["source_file"], "app.js");
        assert_eq!(val["file_type"], "js");
        let functions = val["functions"].as_array().unwrap();
        assert!(functions.iter().any(|f| f.as_str().unwrap().contains("function hello")));
    }

    #[test]
    fn test_read_file_with_hash_code_file() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".py")
            .tempfile()
            .unwrap();
        let content = "def greet():\n    print('hi')\n";
        write!(tmp, "{}", content).unwrap();

        let (value, _hash, _raw) = read_file_with_hash(tmp.path()).unwrap();
        assert_eq!(value["file_type"], "py");
        let functions = value["functions"].as_array().unwrap();
        assert!(functions.iter().any(|f| f.as_str().unwrap().contains("def greet")));
    }

    #[test]
    fn test_read_file_with_hash_config_file() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".yaml")
            .tempfile()
            .unwrap();
        let content = "name: test\nversion: 1.0\n";
        write!(tmp, "{}", content).unwrap();

        let (value, _hash, _raw) = read_file_with_hash(tmp.path()).unwrap();
        assert_eq!(value["content"], content);
        assert_eq!(value["file_type"], "yaml");
    }

    #[test]
    fn test_read_file_as_json_code_file() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".rs")
            .tempfile()
            .unwrap();
        let content = "pub fn main() {}\n";
        write!(tmp, "{}", content).unwrap();

        let value = read_file_as_json(tmp.path()).unwrap();
        assert_eq!(value["file_type"], "rs");
        let functions = value["functions"].as_array().unwrap();
        assert!(functions.iter().any(|f| f.as_str().unwrap().contains("pub fn main")));
    }

    #[test]
    fn test_read_file_as_json_config_file() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".toml")
            .tempfile()
            .unwrap();
        let content = "[package]\nname = \"test\"\n";
        write!(tmp, "{}", content).unwrap();

        let value = read_file_as_json(tmp.path()).unwrap();
        assert_eq!(value["content"], content);
        assert_eq!(value["file_type"], "toml");
    }

    #[test]
    fn test_extract_code_metadata_empty_file() {
        let json = extract_code_metadata("", "empty.py", "py").unwrap();
        let val: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["source_file"], "empty.py");
        assert_eq!(val["file_type"], "py");
        assert!(val["functions"].as_array().unwrap().is_empty());
        assert!(val["classes"].as_array().unwrap().is_empty());
        assert!(val["comments"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_extract_code_metadata_shell_comments() {
        let content = "#!/bin/bash\n# Setup env\necho hello\n# Done\n";
        let json = extract_code_metadata(content, "setup.sh", "sh").unwrap();
        let val: Value = serde_json::from_str(&json).unwrap();
        let comments: Vec<&str> = val["comments"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert!(comments.iter().any(|c| c.contains("Setup env")));
        assert!(comments.iter().any(|c| c.contains("Done")));
        // Shebangs start with #! which is excluded by the #[^!] pattern
        assert!(!comments.iter().any(|c| c.contains("#!/bin/bash")));
    }

    #[test]
    fn test_read_file_with_hash_js_non_twitter() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".js")
            .tempfile()
            .unwrap();
        let content = "function setup() { return true; }\n";
        write!(tmp, "{}", content).unwrap();

        let (value, _hash, _raw) = read_file_with_hash(tmp.path()).unwrap();
        // Should fall back to code metadata since it's not Twitter format
        assert_eq!(value["file_type"], "js");
        let functions = value["functions"].as_array().unwrap();
        assert!(functions.iter().any(|f| f.as_str().unwrap().contains("function setup")));
    }
}
