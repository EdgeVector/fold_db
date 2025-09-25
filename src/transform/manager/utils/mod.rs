//! Unified transform manager utilities eliminating ALL duplication
//!
//! AGGRESSIVE CLEANUP: This module consolidates:
//! - conversion_helper.rs: JsonValue conversion utilities
//! - serialization_helper.rs: Mapping serialization utilities  
//! - event_publisher.rs: Event publishing utilities
//! - field_resolver.rs: Field value resolution utilities
//! - default_value_helper.rs: Default value generation utilities
//! - lock_helper.rs: Lock acquisition utilities
//! - logging_helper.rs: Debug logging utilities
#![allow(unused_imports)]
//! - validation_helper.rs: Validation utilities
//! - Plus multiple duplicate logging/helper patterns found throughout

use crate::fold_db_core::infrastructure::message_bus::{
    schema_events::TransformExecuted, MessageBus,
};
use crate::schema::types::field::common::Field;
use crate::schema::types::field::variant::FieldVariant;
use crate::schema::types::{Schema, SchemaError};
use log::{error, info, warn};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// Re-export commonly used types to avoid import duplication
pub use serde_json::Value;

/// Single unified utility combining ALL transform manager utilities
pub struct TransformUtils;

pub mod conversion;
pub mod default_values;
pub mod locking;
pub mod serialization;

pub use conversion::*;
pub use default_values::*;
pub use locking::*;
pub use serialization::*;

impl TransformUtils {
    // ========== EVENT PUBLISHING UTILITIES ==========

    /// Publish a TransformExecuted event with consistent error handling
    pub fn publish_transform_executed(
        message_bus: &Arc<MessageBus>,
        transform_id: &str,
        status: &str,
    ) -> Result<(), SchemaError> {
        info!(
            "� Publishing TransformExecuted {} event for: {}",
            status, transform_id
        );

        let event = TransformExecuted::new(transform_id, status);

        match message_bus.publish(event) {
            Ok(_) => {
                info!(
                    "✅ Published TransformExecuted {} event for transform: {}",
                    status, transform_id
                );
                Ok(())
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to publish TransformExecuted {} event for {}: {}",
                    status, transform_id, e
                );
                error!("❌ {}", error_msg);
                Err(SchemaError::InvalidData(error_msg))
            }
        }
    }

    /// Handle execution result and publish event
    pub fn handle_execution_result_and_publish(
        message_bus: &Arc<MessageBus>,
        transform_id: &str,
        execution_result: &Result<serde_json::Value, crate::schema::types::SchemaError>,
    ) {
        match execution_result {
            Ok(value) => {
                info!(
                    "✅ Transform {} execution completed successfully",
                    transform_id
                );
                info!("� Execution result details: {:?}", value);

                if let Err(e) =
                    Self::publish_transform_executed(message_bus, transform_id, "success")
                {
                    error!(
                        "❌ Event publishing failed after successful execution: {}",
                        e
                    );
                }
            }
            Err(e) => {
                error!("❌ Transform {} execution failed", transform_id);
                error!("❌ Failure details: {:?}", e);

                if let Err(publish_err) =
                    Self::publish_transform_executed(message_bus, transform_id, "failed")
                {
                    error!(
                        "❌ Event publishing failed after execution failure: {}",
                        publish_err
                    );
                }
            }
        }
    }

    // ========== FIELD RESOLUTION UTILITIES ==========

    /// Extract simplified value from range field atom content
    /// Converts {"range_key":"2","value":"2"} to "2"
    /// Converts {"range_key":"2","value":{"value":"b"}} to "b"
    fn extract_simplified_value(content: &JsonValue) -> Result<JsonValue, SchemaError> {
        // Try to extract the "value" field
        if let Some(value_field) = content.get("value") {
            // If the value field is itself an object with a nested "value", extract that
            if let Some(nested_value) = value_field.get("value") {
                return Ok(nested_value.clone());
            } else {
                return Ok(value_field.clone());
            }
        }

        // If no "value" field found, return the content as-is
        warn!("⚠️ No 'value' field found, returning content as-is");
        Ok(content.clone())
    }

    /// Unified field value resolution from schema using database operations
    pub fn resolve_field_value(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema: &mut Schema,
        field_name: &str,
        unified_filter: Option<crate::schema::types::field::HashRangeFilter>,
    ) -> Result<JsonValue, SchemaError> {
        info!(
            "🔍 FieldValueResolver: Looking up field '{}' in schema '{}'",
            field_name, schema.name
        );

        let field = schema.fields.get_mut(field_name).ok_or_else(|| {
            error!(
                "❌ Field '{}' not found in schema '{}'",
                field_name, schema.name
            );
            SchemaError::InvalidField(format!(
                "Field '{}' not found in schema '{}'",
                field_name, schema.name
            ))
        })?;

        info!(
            "✅ Field '{}' found in schema '{}'",
            field_name, schema.name
        );

        field.resolve_value(db_ops, unified_filter)
    }

    /// Extract molecule_uuid from field variant with consistent error handling
    fn extract_molecule_uuid(
        field: &FieldVariant,
        field_name: &str,
    ) -> Result<String, SchemaError> {
        let molecule_uuid = field
            .common()
            .molecule_uuid()
            .ok_or_else(|| {
                error!("❌ Field '{}' has no molecule_uuid", field_name);
                SchemaError::InvalidField(format!("Field '{}' has no molecule_uuid", field_name))
            })?
            .clone();
        Ok(molecule_uuid)
    }

    /// Load Molecule from database with consistent error handling
    fn load_molecule(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        molecule_uuid: &str,
    ) -> Result<crate::atom::Molecule, SchemaError> {
        match db_ops.get_item::<crate::atom::Molecule>(&format!("ref:{}", molecule_uuid)) {
            Ok(Some(molecule)) => Ok(molecule),
            Ok(None) => {
                error!("❌ Molecule '{}' not found", molecule_uuid);
                Err(SchemaError::InvalidField(format!(
                    "Molecule '{}' not found",
                    molecule_uuid
                )))
            }
            Err(e) => {
                error!("❌ Failed to load Molecule {}: {}", molecule_uuid, e);
                Err(SchemaError::InvalidField(format!(
                    "Failed to load Molecule: {}",
                    e
                )))
            }
        }
    }

    /// Load Atom from database with consistent error handling
    fn load_atom(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        atom_uuid: &str,
    ) -> Result<crate::atom::Atom, SchemaError> {
        db_ops
            .get_item(&format!("atom:{}", atom_uuid))?
            .ok_or_else(|| {
                error!("❌ Atom '{}' not found", atom_uuid);
                SchemaError::InvalidField(format!("Atom '{}' not found", atom_uuid))
            })
    }

    // ========== LOGGING UTILITIES ==========

    /// Standard logging for transform registration
    pub fn log_transform_registration(transform_id: &str, inputs: &[String], output: &str) {
        info!(
            "🔧 Registering transform '{}' with inputs: {:?}, output: {}",
            transform_id, inputs, output
        );
    }

    /// Standard logging for field mapping creation
    pub fn log_field_mapping_creation(field_key: &str, transform_id: &str) {
        info!(
            "🔗 Created field mapping: '{}' -> transform '{}'",
            field_key, transform_id
        );
    }

    /// Standard logging for verification results
    pub fn log_verification_result(_item_type: &str, _id: &str, _details: Option<&str>) {
        // Logging removed to reduce verbosity
    }

    /// Standard logging for atom ref operations
    pub fn log_molecule_operation(molecule_uuid: &str, atom_uuid: &str, operation: &str) {
        info!(
            "🔗 Molecule {} - ref:{} -> atom:{}",
            operation, molecule_uuid, atom_uuid
        );
    }

    /// Simple glob-style pattern matching (supports `*` and `?`)
    fn matches_pattern(text: &str, pattern: &str) -> bool {
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let text_chars: Vec<char> = text.chars().collect();

        Self::match_recursive(&text_chars, &pattern_chars, 0, 0)
    }

    fn match_recursive(text: &[char], pattern: &[char], text_idx: usize, pattern_idx: usize) -> bool {
        // If we've reached the end of both strings, it's a match
        if pattern_idx >= pattern.len() && text_idx >= text.len() {
            return true;
        }

        // If we've reached the end of pattern but not text, no match
        if pattern_idx >= pattern.len() {
            return false;
        }

        match pattern[pattern_idx] {
            '*' => {
                // Try matching zero characters
                if Self::match_recursive(text, pattern, text_idx, pattern_idx + 1) {
                    return true;
                }
                // Try matching one or more characters
                if text_idx < text.len() && Self::match_recursive(text, pattern, text_idx + 1, pattern_idx) {
                    return true;
                }
                false
            }
            '?' => {
                // Match any single character (but not end of string)
                if text_idx < text.len() && Self::match_recursive(text, pattern, text_idx + 1, pattern_idx + 1) {
                    return true;
                }
                false
            }
            ch => {
                // Match exact character
                if text_idx < text.len() && text[text_idx] == ch && Self::match_recursive(text, pattern, text_idx + 1, pattern_idx + 1) {
                    return true;
                }
                false
            }
        }
    }

    /// Standard logging for field mappings state
    pub fn log_field_mappings_state(mappings: &HashMap<String, HashSet<String>>, context: &str) {
        info!(
            "🔍 DEBUG {}: Current field mappings ({} entries):",
            context,
            mappings.len()
        );
        for (field_key, transforms) in mappings {
            info!("  📋 '{}' -> {:?}", field_key, transforms);
        }
        if mappings.is_empty() {
            warn!("⚠️ No field mappings found in {}", context);
        }
    }

    /// Log collection state with consistent formatting
    pub fn log_collection_state<T: std::fmt::Debug>(
        collection_name: &str,
        collection: &T,
        operation: &str,
    ) {
        info!(
            "🔍 DEBUG {}: {} collection state: {:?}",
            operation, collection_name, collection
        );
    }

    /// Read a persisted mapping or return a default value
    pub fn read_mapping<T>(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        key: &str,
        name: &str,
    ) -> Result<T, SchemaError>
    where
        T: serde::de::DeserializeOwned + Default,
    {
        if let Some(data) = db_ops.get_transform_mapping(key)? {
            Self::deserialize_mapping(&data, name)
        } else {
            Ok(T::default())
        }
    }

    /// Insert a value into a set mapping
    pub fn insert_mapping_set(map: &mut HashMap<String, HashSet<String>>, key: &str, value: &str) {
        map.entry(key.to_string())
            .or_default()
            .insert(value.to_string());
    }

    /// Wrap an error with context and log it
    pub fn handle_error<E: std::fmt::Display>(context: &str, err: E) -> SchemaError {
        error!("❌ {}: {}", context, err);
        SchemaError::InvalidData(format!("{}: {}", context, err))
    }
}

// Type aliases for backward compatibility and reduced import burden
pub type LoggingHelper = TransformUtils;
pub type FieldValueResolver = TransformUtils;
pub type EventPublisher = TransformUtils;
pub type ConversionHelper = TransformUtils;
pub type SerializationHelper = TransformUtils;
pub type LockHelper = TransformUtils;
pub type DefaultValueHelper = TransformUtils;
pub type ValidationHelper = TransformUtils;
