use crate::schema::{types::DeclarativeSchemaDefinition, SchemaError};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Handles file-based operations for schemas
pub struct SchemaFileOperations;

impl SchemaFileOperations {
    /// Write schema to file with hash
    pub fn write_schema_to_file(
        declarative_schema: &DeclarativeSchemaDefinition,
        final_name: &str,
        directory: &str,
    ) -> Result<(), SchemaError> {
        let available_schemas_dir = PathBuf::from(directory);
        let target_path = available_schemas_dir.join(format!("{}.json", final_name));

        // Ensure the directory exists
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to create directory: {}", e))
            })?;
        }

        // Add hash to the schema before writing
        let mut schema_with_hash = serde_json::to_value(declarative_schema)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to serialize schema: {}", e)))?;

        let _hash = super::hasher::SchemaHasher::add_hash_to_schema(&mut schema_with_hash)
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to add hash to schema: {}", e))
            })?;

        // Write the schema file with hash and proper formatting
        let formatted_json = serde_json::to_string_pretty(&schema_with_hash)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to format JSON: {}", e)))?;

        std::fs::write(&target_path, formatted_json)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to write schema file: {}", e)))?;
        Ok(())
    }

    /// Read and parse schema file
    pub fn read_schema_file(file_path: &Path) -> Result<Value, SchemaError> {
        let content = std::fs::read_to_string(file_path).map_err(|e| {
            SchemaError::InvalidData(format!(
                "Failed to read file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        serde_json::from_str(&content).map_err(|e| {
            SchemaError::InvalidData(format!(
                "Failed to parse JSON from {}: {}",
                file_path.display(),
                e
            ))
        })
    }

    /// Check if schema file exists
    pub fn schema_file_exists(schema_name: &str, directory: &str) -> bool {
        let target_path = PathBuf::from(directory).join(format!("{}.json", schema_name));
        target_path.exists()
    }

    /// Get schema file path
    pub fn get_schema_file_path(schema_name: &str, directory: &str) -> PathBuf {
        PathBuf::from(directory).join(format!("{}.json", schema_name))
    }
}
