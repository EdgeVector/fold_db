//! HashRange schema executor for transform execution.
//!
//! This module handles the execution of HashRange schema types, including
//! validation, key configuration extraction, and coordination.

use crate::schema::types::SchemaError;
use crate::transform::validation::ValidationTimings;
use crate::transform::coordination::execute_multi_chain_coordination_with_monitoring;
use log::info;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::Instant;

/// Timing information for execution phase
#[derive(Debug)]
pub struct ExecutionTiming {
    pub execution_duration: std::time::Duration,
    pub result: JsonValue,
}

/// Executes a HashRange schema type declarative transform.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
/// * `input_values` - The input values for the transform
///
/// # Returns
///
/// The result of the HashRange schema execution
pub fn execute_hashrange_schema(
    schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
    input_values: HashMap<String, JsonValue>,
) -> Result<JsonValue, SchemaError> {
    let start_time = Instant::now();
    info!("🔧 Executing HashRange schema: {}", schema.name);
    
    // Validate schema structure and field alignment
    let validation_timings = crate::transform::validation::validate_hashrange_schema(schema)?;
    
    // Extract key configuration
    let key_config = extract_hashrange_key_config(schema)?;
    
    // Execute multi-chain coordination
    let execution_timing = execute_hashrange_coordination(schema, &input_values, key_config)?;
    
    // Log performance summary
    log_hashrange_performance_summary(start_time, validation_timings, &execution_timing);
    
    Ok(execution_timing.result)
}

/// Extracts and validates key configuration for HashRange schema.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
///
/// # Returns
///
/// The validated key configuration
pub fn extract_hashrange_key_config(
    schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
) -> Result<&crate::schema::types::json_schema::KeyConfig, SchemaError> {
    let key_config = schema.key.as_ref().ok_or_else(|| {
        SchemaError::InvalidTransform(format!(
            "HashRange schema '{}' must have key configuration with hash_field and range_field", 
            schema.name
        ))
    })?;
    
    info!("📊 HashRange key config - hash_field: {}, range_field: {}", 
          key_config.hash_field, key_config.range_field);
    
    Ok(key_config)
}

/// Executes multi-chain coordination for HashRange schema.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
/// * `input_values` - The input values for the transform
/// * `key_config` - The key configuration
///
/// # Returns
///
/// Execution timing and result
pub fn execute_hashrange_coordination(
    schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
    input_values: &HashMap<String, JsonValue>,
    key_config: &crate::schema::types::json_schema::KeyConfig,
) -> Result<ExecutionTiming, SchemaError> {
    let execution_start = Instant::now();
    let result = execute_multi_chain_coordination_with_monitoring(schema, input_values, key_config)?;
    let execution_duration = execution_start.elapsed();
    
    Ok(ExecutionTiming {
        execution_duration,
        result,
    })
}

/// Logs comprehensive performance summary for HashRange execution.
///
/// # Arguments
///
/// * `start_time` - Overall execution start time
/// * `validation_timings` - Validation phase timings
/// * `execution_timing` - Execution phase timing
pub fn log_hashrange_performance_summary(
    start_time: Instant,
    validation_timings: ValidationTimings,
    execution_timing: &ExecutionTiming,
) {
    let total_duration = start_time.elapsed();
    info!("⏱️ HashRange execution completed in {:?} (execution: {:?}, validation: {:?}, alignment: {:?})", 
          total_duration, 
          execution_timing.execution_duration, 
          validation_timings.validation_duration, 
          validation_timings.alignment_duration);
}
