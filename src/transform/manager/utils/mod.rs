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
