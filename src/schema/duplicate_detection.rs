use crate::schema::{types::DeclarativeSchemaDefinition, SchemaError};

/// Handles duplicate detection and conflict resolution for schemas
pub struct SchemaDuplicateDetector;

impl SchemaDuplicateDetector {
    /// Check for schema conflicts and duplicates
    pub fn check_schema_conflicts(
        declarative_schema: &DeclarativeSchemaDefinition,
        final_name: &str,
        directory: &str,
        find_hash_fn: impl Fn(&str, &str) -> Result<Option<String>, SchemaError>,
    ) -> Result<(), SchemaError> {
        // Check for name conflicts
        if super::file_operations::SchemaFileOperations::schema_file_exists(final_name, directory) {
            Self::handle_existing_schema_conflict(declarative_schema, final_name, directory)?;
        }

        // Check for content-based duplicates across all schemas
        Self::check_content_duplicates(declarative_schema, final_name, find_hash_fn)?;

        Ok(())
    }

    /// Handle conflicts with existing schema files
    fn handle_existing_schema_conflict(
        declarative_schema: &DeclarativeSchemaDefinition,
        final_name: &str,
        directory: &str,
    ) -> Result<(), SchemaError> {
        let target_path = super::file_operations::SchemaFileOperations::get_schema_file_path(
            final_name, directory,
        );
        let existing_schema_json =
            super::file_operations::SchemaFileOperations::read_schema_file(&target_path)?;

        let new_schema_json = serde_json::to_value(declarative_schema).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to serialize new schema: {}", e))
        })?;

        let new_hash =
            super::hasher::SchemaHasher::calculate_hash(&new_schema_json).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to calculate new schema hash: {}", e))
            })?;

        // Compare hashes
        if let Some(existing_hash) = existing_schema_json.get("hash").and_then(|h| h.as_str()) {
            Self::compare_schema_hashes(existing_hash, &new_hash, final_name)
        } else {
            let existing_hash = super::hasher::SchemaHasher::calculate_hash(&existing_schema_json)
                .map_err(|e| {
                    SchemaError::InvalidData(format!(
                        "Failed to calculate existing schema hash: {}",
                        e
                    ))
                })?;
            Self::compare_schema_hashes(&existing_hash, &new_hash, final_name)
        }
    }

    /// Compare schema hashes and handle conflicts
    fn compare_schema_hashes(
        existing_hash: &str,
        new_hash: &str,
        final_name: &str,
    ) -> Result<(), SchemaError> {
        if new_hash == existing_hash {
            Ok(()) // Allow identical schemas
        } else {
            Err(SchemaError::InvalidData(format!(
                "Schema '{}' already exists with different content. Existing hash: {}, new hash: {}. Use a different name or remove the existing schema first.",
                final_name, existing_hash, new_hash
            )))
        }
    }

    /// Check for content-based duplicates across all schemas
    fn check_content_duplicates(
        declarative_schema: &DeclarativeSchemaDefinition,
        final_name: &str,
        find_hash_fn: impl Fn(&str, &str) -> Result<Option<String>, SchemaError>,
    ) -> Result<(), SchemaError> {
        let new_schema_json = serde_json::to_value(declarative_schema).map_err(|e| {
            SchemaError::InvalidData(format!(
                "Failed to serialize schema for duplicate check: {}",
                e
            ))
        })?;
        let new_hash =
            super::hasher::SchemaHasher::calculate_hash(&new_schema_json).map_err(|e| {
                SchemaError::InvalidData(format!(
                    "Failed to calculate schema hash for duplicate check: {}",
                    e
                ))
            })?;

        if let Some(duplicate_name) = find_hash_fn(&new_hash, final_name)? {
            return Err(SchemaError::InvalidData(format!(
                "Schema content already exists as '{}' (hash: {}). Schemas must have unique content.",
                duplicate_name, new_hash
            )));
        }

        Ok(())
    }
}
