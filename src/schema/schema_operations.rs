use super::{schema_lock_error};
use crate::fold_db_core::infrastructure::message_bus::MessageBus;
use crate::logging::features::{log_feature, LogFeature};
use crate::schema::constants::{
    DEFAULT_OUTPUT_FIELD_NAME, DEFAULT_TRANSFORM_ID_SUFFIX, KEY_CONFIG_HASH_FIELD,
    KEY_CONFIG_RANGE_FIELD, KEY_FIELD_NAME,
};
use crate::schema::types::{
    DeclarativeSchemaDefinition, Field, FieldVariant, Schema, SchemaError,
};
use crate::schema::{
    interpret_schema, map_fields, MoleculeVariant,
    SchemaState,
};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::core::SchemaCore;

impl SchemaCore {
    /// Add a new schema from JSON to the available_schemas directory with validation
    pub fn add_schema_to_available_directory(
        &self,
        json_content: &str,
        schema_name: Option<String>,
    ) -> Result<String, SchemaError> {
        info!("Adding new schema to available_schemas directory");

        // Parse and validate the JSON schema
        let json_schema = self.parse_and_validate_json_schema(json_content)?;
        let final_name = schema_name.unwrap_or_else(|| json_schema.name.clone());

        // Load schema into memory
        let schema = self.interpret_schema(json_schema)?;
        self.load_schema_internal(schema)?;

        Ok(final_name)
    }

    /// Parse and validate JSON schema content
    fn parse_and_validate_json_schema(
        &self,
        json_content: &str,
    ) -> Result<super::types::JsonSchemaDefinition, SchemaError> {
        let json_schema: super::types::JsonSchemaDefinition = serde_json::from_str(json_content)
            .map_err(|e| SchemaError::InvalidField(format!("Invalid JSON schema: {}", e)))?;

        Ok(json_schema)
    }

    /// Find a schema with the same hash (for duplicate detection) in the specified directory
    /// Find a schema with the same hash (for duplicate detection)
    fn find_schema_by_hash(
        &self,
        target_hash: &str,
        exclude_name: &str,
    ) -> Result<Option<String>, SchemaError> {
        let available_schemas_dir = std::path::PathBuf::from("available_schemas");

        if let Ok(entries) = std::fs::read_dir(&available_schemas_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    // Skip the file we're trying to create
                    if let Some(file_stem) = path.file_stem() {
                        if file_stem == exclude_name {
                            continue;
                        }
                    }

                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(schema_json) = serde_json::from_str::<serde_json::Value>(&content)
                        {
                            // Check if schema has a hash field
                            if let Some(existing_hash) =
                                schema_json.get("hash").and_then(|h| h.as_str())
                            {
                                if existing_hash == target_hash {
                                    if let Some(name) =
                                        schema_json.get("name").and_then(|n| n.as_str())
                                    {
                                        return Ok(Some(name.to_string()));
                                    }
                                }
                            } else {
                                // Calculate hash for schemas without hash field
                                if let Ok(calculated_hash) =
                                    super::hasher::SchemaHasher::calculate_hash(&schema_json)
                                {
                                    if calculated_hash == target_hash {
                                        if let Some(name) =
                                            schema_json.get("name").and_then(|n| n.as_str())
                                        {
                                            return Ok(Some(name.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Maps fields between schemas based on their defined relationships.
    /// Returns a list of Molecules that need to be persisted in FoldDB.
    pub fn map_fields(&self, schema_name: &str) -> Result<Vec<MoleculeVariant>, SchemaError> {
        info!("🔧 Starting field mapping for schema '{}'", schema_name);

        let schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;

        // First collect all the source field molecule_uuids we need
        let mut field_mappings = Vec::new();
        if let Some(schema) = schemas.get(schema_name) {
            for (field_name, field) in &schema.fields {
                for (source_schema_name, source_field_name) in field.field_mappers() {
                    if let Some(source_schema) = schemas.get(source_schema_name) {
                        if let Some(source_field) = source_schema.fields.get(source_field_name) {
                            if let Some(molecule_uuid) = source_field.molecule_uuid() {
                                field_mappings.push((field_name.clone(), molecule_uuid.clone()));
                            }
                        }
                    }
                }
            }
        }
        drop(schemas); // Release the immutable lock

        // Now get a mutable lock to update the fields
        let mut schemas = self.schemas.lock().map_err(|_| schema_lock_error())?;

        let schema = schemas
            .get_mut(schema_name)
            .ok_or_else(|| SchemaError::InvalidData(format!("Schema {schema_name} not found")))?;

        // Apply the collected mappings
        for (field_name, molecule_uuid) in field_mappings {
            if let Some(field) = schema.fields.get_mut(&field_name) {
                field.set_molecule_uuid(molecule_uuid);
            }
        }

        // Use the field mapping module to create molecules for unmapped fields
        let molecules = map_fields(&self.db_ops, schema)?;

        // Persist the updated schema
        self.persist_schema(schema)?;

        // Also update the available HashMap to keep it in sync
        let updated_schema = schema.clone();
        drop(schemas); // Release the schemas lock

        let mut available = self.available.lock().map_err(|_| schema_lock_error())?;

        if let Some((_, state)) = available.get(schema_name) {
            let state = *state;
            available.insert(schema_name.to_string(), (updated_schema, state));
        }

        Ok(molecules)
    }

    /// Interprets a JSON schema definition and converts it to a Schema.
    pub fn interpret_schema(
        &self,
        json_schema: crate::schema::types::JsonSchemaDefinition,
    ) -> Result<Schema, SchemaError> {
        interpret_schema(&SchemaValidator::new(self), json_schema)
    }

    /// Interprets a JSON schema from a string and loads it as Available.
    pub fn load_schema_from_json(&self, json_str: &str) -> Result<(), SchemaError> {
        let schema = load_schema_from_json(&SchemaValidator::new(self), json_str)?;
        self.load_schema_internal(schema)
    }

    /// Interprets a JSON schema from a file and loads it as Available.
    pub fn load_schema_from_file(&self, path: &str) -> Result<(), SchemaError> {
        let schema = load_schema_from_file(&SchemaValidator::new(self), path)?;
        self.load_schema_internal(schema)
    }
}

/// Unified key extraction helper for all schema types.
///
/// This function provides a single entry point to extract hash and range values
/// from any schema type, supporting both the new universal KeyConfig and legacy
/// range_key patterns.
///
/// # Arguments
///
/// * `schema` - The schema to extract keys from
/// * `data` - The data object containing the actual values
///
/// # Returns
///
/// A tuple of (hash_value, range_value) where both are Option<String>.
/// For Single schemas, both will be None unless key is provided.
/// For Range schemas, hash_value may be None, range_value will be extracted.
/// For HashRange schemas, both will be extracted from key configuration.
pub trait SchemaKeyContext {
    fn schema_name(&self) -> &str;
    fn schema_type(&self) -> &crate::schema::types::schema::SchemaType;
    fn key_config(&self) -> Option<&crate::schema::types::key_config::KeyConfig>;
}

impl SchemaKeyContext for Schema {
    fn schema_name(&self) -> &str {
        &self.name
    }

    fn schema_type(&self) -> &crate::schema::types::schema::SchemaType {
        &self.schema_type
    }

    fn key_config(&self) -> Option<&crate::schema::types::key_config::KeyConfig> {
        self.key.as_ref()
    }
}

impl SchemaKeyContext for DeclarativeSchemaDefinition {
    fn schema_name(&self) -> &str {
        &self.name
    }

    fn schema_type(&self) -> &crate::schema::types::schema::SchemaType {
        &self.schema_type
    }

    fn key_config(&self) -> Option<&crate::schema::types::key_config::KeyConfig> {
        self.key.as_ref()
    }
}

pub fn extract_unified_keys<S>(
    schema: &S,
    data: &serde_json::Value,
) -> Result<(Option<String>, Option<String>), SchemaError>
where
    S: SchemaKeyContext,
{
    match schema.schema_type() {
        crate::schema::types::schema::SchemaType::Single => {
            // For Single schemas, keys are optional and used for indexing hints
            // Check if schema has a key configuration
            if let Some(key_config) = schema.key_config() {
                let hash_value = if !key_config.hash_field.trim().is_empty() {
                    extract_field_value(data, &key_config.hash_field)?
                } else {
                    None
                };

                let range_value = if !key_config.range_field.trim().is_empty() {
                    extract_field_value(data, &key_config.range_field)?
                } else {
                    None
                };

                Ok((hash_value, range_value))
            } else {
                // No key configuration, return None for both
                Ok((None, None))
            }
        }
        crate::schema::types::schema::SchemaType::Range { range_key } => {
            // For Range schemas, use universal key configuration if available, otherwise fall back to legacy range_key
            let range_value = if let Some(key_config) = schema.key_config() {
                // Universal key configuration takes precedence
                let trimmed_field = key_config.range_field.trim();
                if trimmed_field.is_empty() {
                    return Err(SchemaError::InvalidData(
                        "Range schema with key configuration must have range_field".to_string(),
                    ));
                }

                match extract_field_value(data, trimmed_field)? {
                    Some(value) => Some(value),
                    None => {
                        if let Some(value) = extract_field_value(data, "range_key")? {
                            Some(value)
                        } else if let Some(value) = extract_field_value(data, "range")? {
                            Some(value)
                        } else {
                            return Err(SchemaError::InvalidData(format!(
                                "Range schema '{}' requires key.range_field '{}' in payload or normalized range value",
                                schema.schema_name(), trimmed_field
                            )));
                        }
                    }
                }
            } else {
                // Legacy range_key support - this maintains backward compatibility
                // First try to extract using the schema's range_key field name
                let trimmed_range_key = range_key.trim();
                if trimmed_range_key.is_empty() {
                    return Err(SchemaError::InvalidData(format!(
                        "Range schema '{}' is missing range_key configuration",
                        schema.schema_name()
                    )));
                }

                if let Some(value) = extract_field_value(data, trimmed_range_key)? {
                    Some(value)
                } else if let Some(value) = extract_field_value(data, "range_key")? {
                    Some(value)
                } else if let Some(value) = extract_field_value(data, "range")? {
                    Some(value)
                } else {
                    return Err(SchemaError::InvalidData(format!(
                        "Range schema '{}' requires range key field '{}' or normalized range value in payload",
                        schema.schema_name(), trimmed_range_key
                    )));
                }
            };

            let hash_value = if let Some(key_config) = schema.key_config() {
                if !key_config.hash_field.trim().is_empty() {
                    extract_field_value(data, &key_config.hash_field)?
                } else {
                    None
                }
            } else {
                None
            };

            Ok((hash_value, range_value))
        }
        crate::schema::types::schema::SchemaType::HashRange => {
            // For HashRange schemas, both hash and range are required
            let key_config = schema.key_config().ok_or_else(|| {
                SchemaError::InvalidData("HashRange schema requires key configuration".to_string())
            })?;

            if key_config.hash_field.trim().is_empty() {
                return Err(SchemaError::InvalidData(
                    "HashRange schema requires key.hash_field".to_string(),
                ));
            }
            if key_config.range_field.trim().is_empty() {
                return Err(SchemaError::InvalidData(
                    "HashRange schema requires key.range_field".to_string(),
                ));
            }
            let hash_field: &str = &key_config.hash_field;
            let range_field: &str = &key_config.range_field;

            // Prefer direct hash_key/range_key values provided by the caller when available.
            // Declarative HashRange transforms emit these fields explicitly, so we can avoid
            // re-evaluating complex key expressions (e.g., map().split_by_word()).
            let direct_hash = data
                .get("hash_key")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            let direct_range = data
                .get("range_key")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());

            let hash_value = if let Some(hash) = direct_hash {
                hash
            } else {
                extract_field_value(data, hash_field)?.ok_or_else(|| {
                    SchemaError::InvalidData(format!(
                        "HashRange hash_field '{}' not found in data",
                        hash_field
                    ))
                })?
            };

            let range_value = if let Some(range) = direct_range {
                range
            } else {
                extract_field_value(data, range_field)?.ok_or_else(|| {
                    SchemaError::InvalidData(format!(
                        "HashRange range_field '{}' not found in data",
                        range_field
                    ))
                })?
            };

            Ok((Some(hash_value), Some(range_value)))
        }
    }
}

/// Helper function to extract field value from data using field expression.
///
/// Supports both direct field access and dotted path expressions.
fn extract_field_value(
    data: &serde_json::Value,
    field_expression: &str,
) -> Result<Option<String>, SchemaError> {
    let trimmed_expr = field_expression.trim();
    if trimmed_expr.is_empty() {
        return Ok(None);
    }

    // Handle dotted path expressions (e.g., "data.timestamp", "input.map().id")
    if trimmed_expr.contains('.') {
        // For now, support simple dotted paths like "data.field" or "field.subfield"
        let parts: Vec<&str> = trimmed_expr.split('.').collect();
        let mut current = data;

        for part in parts {
            if let Some(obj) = current.as_object() {
                if let Some(value) = obj.get(part) {
                    current = value;
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        }

        // Convert the final value to string
        Ok(Some(current.to_string().trim_matches('"').to_string()))
    } else {
        // Direct field access
        if let Some(obj) = data.as_object() {
            if let Some(value) = obj.get(trimmed_expr) {
                Ok(Some(value.to_string().trim_matches('"').to_string()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

#[inline]
fn last_segment(expression: &str) -> &str {
    expression.rsplit('.').next().unwrap_or("")
}

/// Standardized result shaping helper for all schema types.
///
/// Shapes query/mutation results into a consistent { hash, range, fields } object.
pub fn shape_unified_result<S>(
    schema: &S,
    data: &serde_json::Value,
    hash_value: Option<String>,
    range_value: Option<String>,
) -> Result<serde_json::Value, SchemaError>
where
    S: SchemaKeyContext,
{
    let mut result = serde_json::Map::new();
    result.insert(
        "hash".to_string(),
        serde_json::Value::String(hash_value.unwrap_or_default()),
    );
    result.insert(
        "range".to_string(),
        serde_json::Value::String(range_value.unwrap_or_default()),
    );

    // Determine key field names (last segment of expressions)
    let mut key_field_names: Vec<String> = Vec::new();
    match schema.schema_type() {
        crate::schema::types::schema::SchemaType::Single => {
            if let Some(key) = schema.key_config() {
                if !key.hash_field.trim().is_empty() {
                    key_field_names.push(last_segment(&key.hash_field).to_string());
                }
                if !key.range_field.trim().is_empty() {
                    key_field_names.push(last_segment(&key.range_field).to_string());
                }
            }
        }
        crate::schema::types::schema::SchemaType::Range { range_key } => {
            key_field_names.push(range_key.clone());
            if let Some(key) = schema.key_config() {
                if !key.hash_field.trim().is_empty() {
                    key_field_names.push(last_segment(&key.hash_field).to_string());
                }
                if !key.range_field.trim().is_empty() {
                    key_field_names.push(last_segment(&key.range_field).to_string());
                }
            }
        }
        crate::schema::types::schema::SchemaType::HashRange => {
            if let Some(key) = schema.key_config() {
                if key.hash_field.trim().is_empty() || key.range_field.trim().is_empty() {
                    return Err(SchemaError::InvalidData(
                        "HashRange schema requires key.hash_field and key.range_field".to_string(),
                    ));
                }
                key_field_names.push(last_segment(&key.hash_field).to_string());
                key_field_names.push(last_segment(&key.range_field).to_string());
                // HashRange payloads frequently include explicit hash_key/range_key fields; exclude
                // them from the shaped field set so only derived values remain.
                key_field_names.push("hash_key".to_string());
                key_field_names.push("range_key".to_string());
            }
        }
    }

    // Build fields object excluding key field names
    let mut fields_obj = serde_json::Map::new();
    if let Some(obj) = data.as_object() {
        for (k, v) in obj {
            // CRITICAL FIX: For Range schemas, we need to include the range key field in the fields object
            // so that transforms can access it. The range key field must be available in atom content.
            let should_exclude = match schema.schema_type() {
                crate::schema::types::schema::SchemaType::Range { .. } => {
                    // For Range schemas, only exclude hash_key and range_key, but include the actual range field
                    k == "hash_key" || k == "range_key"
                }
                _ => {
                    // For other schema types, exclude all key field names
                    key_field_names.iter().any(|n| n == k)
                }
            };
            
            if !should_exclude {
                fields_obj.insert(k.clone(), v.clone());
            }
        }
    }
    result.insert("fields".to_string(), serde_json::Value::Object(fields_obj));

    Ok(serde_json::Value::Object(result))
}
