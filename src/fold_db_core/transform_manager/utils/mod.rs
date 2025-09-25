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
        info!("🎯 Extracting simplified value from: {}", content);

        // Try to extract the "value" field
        if let Some(value_field) = content.get("value") {
            // If the value field is itself an object with a nested "value", extract that
            if let Some(nested_value) = value_field.get("value") {
                info!("✅ Extracted nested value: {}", nested_value);
                return Ok(nested_value.clone());
            } else {
                info!("✅ Extracted direct value: {}", value_field);
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
        schema: &Schema,
        field_name: &str,
        unified_filter: Option<crate::schema::types::field::HashRangeFilter>,
    ) -> Result<JsonValue, SchemaError> {
        info!(
            "🔍 FieldValueResolver: Looking up field '{}' in schema '{}'",
            field_name, schema.name
        );

        let field = schema.fields.get(field_name).ok_or_else(|| {
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

        // Check if this is a range field first
        match field {
            FieldVariant::Range(_) => {
                info!("🔍 Detected range field, using MoleculeRange resolution");

                // FIXED: Search for all MoleculeRanges that match the pattern {schema}_{field}_range_*
                let range_prefix = format!("ref:{}_{}_range_", schema.name, field_name);
                info!(
                    "🔍 Looking for MoleculeRanges with prefix: {}",
                    range_prefix
                );

                // Get all keys that match the pattern
                let matching_keys = db_ops.list_items_with_prefix(&range_prefix).map_err(|e| {
                    SchemaError::InvalidField(format!("Failed to search for MoleculeRanges: {}", e))
                })?;

                info!(
                    "🔍 Found {} MoleculeRange keys matching pattern",
                    matching_keys.len()
                );

                if matching_keys.is_empty() {
                    info!(
                        "⚠️ No MoleculeRanges found for field {}.{}",
                        schema.name, field_name
                    );
                    return Ok(JsonValue::Object(serde_json::Map::new()));
                }

                let mut combined_data = serde_json::Map::new();

                // Process each MoleculeRange
                for key in matching_keys {
                    info!("🔍 Processing MoleculeRange key: {}", key);

                    match db_ops.get_item::<crate::atom::MoleculeRange>(&key) {
                        Ok(Some(range_molecule)) => {
                            info!(
                                "✅ Found MoleculeRange with {} entries",
                                range_molecule.atom_uuids.len()
                            );

                            // Extract range key from the MoleculeRange UUID
                            // Pattern: ref:{schema}_{field}_range_{range_key}
                            let range_key = key.strip_prefix(&range_prefix).ok_or_else(|| {
                                SchemaError::InvalidData(format!(
                                    "Invalid MoleculeRange key format: {}",
                                    key
                                ))
                            })?;

                            info!("🔍 Extracted range key: '{}' from key: {}", range_key, key);

                            // Apply unified filter if provided
                            let should_process = if let Some(unified_filter) = &unified_filter {
                                info!("🎯 Processing unified filter: {:?}", unified_filter);

                                // Apply the filter to this specific range key
                                match unified_filter {
                                    crate::schema::types::field::HashRangeFilter::RangePrefix(prefix) => {
                                        info!("🎯 Applying RangePrefix filter for: {}", prefix);
                                        range_key.starts_with(prefix)
                                    }
                                    crate::schema::types::field::HashRangeFilter::RangeRange { start, end } => {
                                        info!("🎯 Applying RangeRange filter from {} to {}", start, end);
                                        range_key >= start.as_str() && range_key < end.as_str()
                                    }
                                    crate::schema::types::field::HashRangeFilter::RangePattern(pattern) => {
                                        info!("🎯 Applying RangePattern filter for: {}", pattern);
                                        Self::matches_pattern(range_key, pattern)
                                    }
                                    crate::schema::types::field::HashRangeFilter::Value(value) => {
                                        info!("🎯 Applying Value filter for: {}", value);
                                        // For value filters, we need to check the actual atom content
                                        // This is more complex and would require loading the atom first
                                        true // For now, include all entries when using Value filter
                                    }
                                    _ => {
                                        // For other HashRangeFilter variants that don't apply to range keys
                                        info!("🎯 Filter variant doesn't apply to range keys, processing all");
                                        true
                                    }
                                }
                            } else {
                                info!("📋 Processing all range keys");
                                true
                            };

                            if should_process {
                                // Process all atoms in this MoleculeRange
                                for (atom_key, atom_uuid) in &range_molecule.atom_uuids {
                                    info!(
                                        "🔗 Processing range key '{}' -> atom: {}",
                                        atom_key, atom_uuid
                                    );

                                    match Self::load_atom(db_ops, atom_uuid) {
                                        Ok(atom) => {
                                            let content = atom.content();
                                            info!(
                                                "📦 Range entry '{}' content: {}",
                                                atom_key, content
                                            );

                                            // Extract simplified value instead of full structure
                                            let simplified_value =
                                                Self::extract_simplified_value(content)?;
                                            info!(
                                                "🎯 Simplified value for key '{}': {}",
                                                atom_key, simplified_value
                                            );

                                            combined_data
                                                .insert(atom_key.clone(), simplified_value);
                                        }
                                        Err(e) => {
                                            error!(
                                                "❌ Failed to load atom {} for range key '{}': {}",
                                                atom_uuid, atom_key, e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            error!("❌ MoleculeRange '{}' not found", key);
                        }
                        Err(e) => {
                            error!("❌ Error loading MoleculeRange '{}': {}", key, e);
                        }
                    }
                }

                let result = JsonValue::Object(combined_data);
                info!(
                    "✅ Range field resolution complete - combined result: {}",
                    result
                );
                return Ok(result);
            }
            FieldVariant::Single(_) => {
                info!("🔄 Detected single field, using Molecule resolution");
            }
            FieldVariant::HashRange(_) => {
                info!("🔑 Detected HashRange field, using HashRange resolution");

                // For HashRange schemas, we need to query using the hash key filter
                if let Some(hash_filter) = &unified_filter {
                    info!("🎯 Processing HashRange unified filter: {:?}", hash_filter);

                    // Extract the hash key from the filter
                    let hash_key = match hash_filter {
                        crate::schema::types::field::HashRangeFilter::HashKey(key) => key.clone(),
                        crate::schema::types::field::HashRangeFilter::HashRangeKey { hash, .. } => hash.clone(),
                        crate::schema::types::field::HashRangeFilter::HashRangePrefix { hash, .. } => hash.clone(),
                        crate::schema::types::field::HashRangeFilter::HashRangeRange { hash, .. } => hash.clone(),
                        crate::schema::types::field::HashRangeFilter::HashRangePattern { hash, .. } => hash.clone(),
                        _ => {
                            // For filters that don't specify a hash key, we can't query specific hash ranges
                            info!("🔍 HashRange filter doesn't specify hash key, using general query");
                            return Ok(serde_json::Value::Null);
                        }
                    };

                    info!("🔍 HashRange query for hash key: '{}'", hash_key);

                    // Query HashRange data using the hash key
                    // Look for atoms with the pattern: {schema_name}_{hash_key}
                    let hashrange_atom_uuid = format!("{}_{}", schema.name, hash_key);
                    println!("🔍 Looking for HashRange atom: {}", hashrange_atom_uuid);

                    // Debug: List all atoms in the database to see what's actually stored
                    info!("🔍 DEBUG: Listing all atoms in database...");
                    // This is a debug approach - in production we'd want a more efficient way

                    // Debug: Try to list what's actually in the database
                    println!(
                        "🔍 DEBUG: Attempting to retrieve atom with key: atom:{}",
                        hashrange_atom_uuid
                    );

                    match db_ops
                        .get_item::<crate::atom::Atom>(&format!("atom:{}", hashrange_atom_uuid))
                    {
                        Ok(Some(atom)) => {
                            println!("✅ Found HashRange atom for key '{}'", hash_key);
                            let content = atom.content().clone();
                            println!("🔍 DEBUG: Atom content: {}", content);

                            // Extract the specific field value from the compound result
                            if let Some(content_obj) = content.as_object() {
                                if let Some(field_value) = content_obj.get(field_name) {
                                    info!(
                                        "✅ Extracted field '{}' value: {}",
                                        field_name, field_value
                                    );
                                    return Ok(field_value.clone());
                                } else {
                                    info!(
                                        "⚠️ Field '{}' not found in HashRange atom content",
                                        field_name
                                    );
                                    info!(
                                        "🔍 DEBUG: Available fields in content: {:?}",
                                        content_obj.keys().collect::<Vec<_>>()
                                    );
                                    return Ok(JsonValue::Null);
                                }
                            } else {
                                info!("⚠️ HashRange atom content is not an object: {}", content);
                                return Ok(JsonValue::Null);
                            }
                        }
                        Ok(None) => {
                            println!(
                                "⚠️ HashRange atom not found for key '{}' (UUID: {})",
                                hash_key, hashrange_atom_uuid
                            );
                            return Ok(JsonValue::Null);
                        }
                        Err(e) => {
                            error!(
                                "❌ Error loading HashRange atom '{}': {}",
                                hashrange_atom_uuid, e
                            );
                            return Err(SchemaError::InvalidField(format!(
                                "Error loading HashRange atom '{}': {}",
                                hashrange_atom_uuid, e
                            )));
                        }
                    }
                } else {
                    info!("🔍 No hash_key_filter provided - returning list of available words");

                    // When no hash_key_filter is provided, return a list of available words
                    // This is more appropriate for HashRange schemas than aggregating all data
                    let atom_prefix = format!("atom:{}_", schema.name);
                    info!(
                        "🔍 Scanning for all HashRange atoms with prefix: {}",
                        atom_prefix
                    );

                    // Get all atom keys with the schema prefix
                    let atom_keys = db_ops.list_items_with_prefix(&atom_prefix)?;
                    info!(
                        "🔍 Found {} HashRange atoms for schema '{}'",
                        atom_keys.len(),
                        schema.name
                    );

                    let mut available_words = Vec::new();

                    // Extract word names from atom UUIDs
                    for atom_key in atom_keys {
                        // Remove the "atom:" prefix and schema name to get the word
                        if let Some(word_part) =
                            atom_key.strip_prefix(&format!("atom:{}_", schema.name))
                        {
                            available_words.push(JsonValue::String(word_part.to_string()));
                        }
                    }

                    info!(
                        "✅ Found {} unique words: {:?}",
                        available_words.len(),
                        available_words.iter().take(10).collect::<Vec<_>>()
                    );

                    // Return a simple list of available words
                    return Ok(JsonValue::Array(available_words));
                }
            }
        }

        // BRIDGE FIX: Primary dynamic molecule lookup with static fallback
        let dynamic_molecule_uuid = format!("{}_{}_single", schema.name, field_name);
        info!(
            "🔍 BRIDGE FIX: Checking dynamic Molecule UUID first: {}",
            dynamic_molecule_uuid
        );

        // Try dynamic molecule system first (primary path)
        match db_ops.get_item::<crate::atom::Molecule>(&format!("ref:{}", dynamic_molecule_uuid)) {
            Ok(Some(dynamic_molecule)) => {
                let dynamic_atom_uuid = dynamic_molecule.get_atom_uuid();
                info!(
                    "✅ BRIDGE FIX: Found dynamic molecule pointing to atom: {}",
                    dynamic_atom_uuid
                );

                let atom = Self::load_atom(db_ops, dynamic_atom_uuid)?;
                let content = atom.content().clone();
                info!(
                    "✅ Query resolved using dynamic molecule system - content: {}",
                    content
                );
                return Ok(content);
            }
            Ok(None) => {
                info!("🔍 BRIDGE FIX: Dynamic molecule not found, trying static schema fallback");
            }
            Err(e) => {
                error!("🔍 BRIDGE FIX: Error checking dynamic molecule: {}", e);
            }
        }

        match Self::extract_molecule_uuid(field, field_name) {
            Ok(molecule_uuid) => {
                info!(
                    "🔗 BRIDGE FIX: Using static schema molecule_uuid: {}",
                    molecule_uuid
                );

                match Self::load_molecule(db_ops, &molecule_uuid) {
                    Ok(molecule) => {
                        let atom_uuid = molecule.get_atom_uuid();
                        info!("🔗 Static molecule points to atom: {}", atom_uuid);

                        let atom = Self::load_atom(db_ops, atom_uuid)?;
                        let content = atom.content().clone();
                        info!(
                            "✅ Query resolved using static schema fallback - content: {}",
                            content
                        );
                        Ok(content)
                    }
                    Err(e) => {
                        error!(
                            "❌ Failed to load static molecule '{}': {}",
                            molecule_uuid, e
                        );
                        Err(e)
                    }
                }
            }
            Err(e) => {
                error!(
                    "❌ BRIDGE FIX: Both dynamic and static molecule lookups failed for field '{}'",
                    field_name
                );
                Err(e)
            }
        }
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
        info!("🔍 Loading Molecule from database...");

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
        info!("🔍 Loading Atom from database...");
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
    pub fn log_verification_result(item_type: &str, id: &str, details: Option<&str>) {
        match details {
            Some(detail) => info!("✅ Verified {}: {} - {}", item_type, id, detail),
            None => info!("✅ Verified {}: {}", item_type, id),
        }
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
            info!("⚠️ No field mappings found in {}", context);
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
