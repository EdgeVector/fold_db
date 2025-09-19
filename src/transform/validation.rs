//! Validation utilities for transform execution.
//!
//! This module provides validation functions for schema structure, field alignment,
//! and other validation concerns in the transform execution framework.

use crate::transform::iterator_stack::field_alignment::{FieldAlignmentValidator, AlignmentValidationResult};
use crate::transform::shared_utilities::{
    parse_atom_uuid_expression, 
    collect_expressions_from_schema,
    create_parsing_error,
    format_alignment_validation_errors,
};
use crate::schema::types::SchemaError;
use log::{info, error};

/// Timing information for validation phases
#[derive(Debug)]
pub struct ValidationTimings {
    pub validation_duration: std::time::Duration,
    pub alignment_duration: std::time::Duration,
}

/// Validates HashRange schema structure and field alignment.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
///
/// # Returns
///
/// Validation timings and any validation errors
pub fn validate_hashrange_schema(
    schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
) -> Result<ValidationTimings, SchemaError> {
    use std::time::Instant;
    
    // Validate schema structure
    let validation_start = Instant::now();
    schema.validate()?;
    let validation_duration = validation_start.elapsed();
    info!("⏱️ HashRange schema validation took: {:?}", validation_duration);
    
    // Validate field alignment
    let alignment_start = Instant::now();
    validate_field_alignment(schema)?;
    let alignment_duration = alignment_start.elapsed();
    info!("⏱️ HashRange field alignment validation took: {:?}", alignment_duration);
    
    Ok(ValidationTimings {
        validation_duration,
        alignment_duration,
    })
}

/// Validates field alignment for declarative transforms.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
///
/// # Returns
///
/// Validation result or error
pub fn validate_field_alignment(
    schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
) -> Result<AlignmentValidationResult, SchemaError> {
    validate_field_alignment_unified(Some(schema), None)
}

/// Unified field alignment validation function that consolidates all validation logic.
///
/// This function eliminates duplication across executor modules by providing a single
/// source of truth for field alignment validation.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition (optional)
/// * `parsed_chains` - Pre-parsed chains (optional, if provided, skips parsing)
///
/// # Returns
///
/// Validation result or error
pub fn validate_field_alignment_unified(
    schema: Option<&crate::schema::types::json_schema::DeclarativeSchemaDefinition>,
    parsed_chains: Option<&[crate::transform::iterator_stack::chain_parser::ParsedChain]>,
) -> Result<AlignmentValidationResult, SchemaError> {
    // Handle case where parsed chains are provided directly (from executor modules)
    if let Some(chains) = parsed_chains {
        info!("🔍 Validating field alignment with {} pre-parsed chains", chains.len());
        return validate_alignment_with_chains(chains);
    }
    
    // Handle case where schema is provided (original behavior)
    if let Some(schema) = schema {
        info!("🔍 Validating field alignment for schema: {}", schema.name);
        return validate_alignment_from_schema(schema);
    }
    
    // Neither schema nor chains provided - return empty result
    info!("⚠️ No schema or parsed chains provided for alignment validation");
    Ok(AlignmentValidationResult {
        valid: true,
        max_depth: 0,
        field_alignments: std::collections::HashMap::new(),
        errors: Vec::new(),
        warnings: Vec::new(),
    })
}

/// Validates field alignment using pre-parsed chains (for executor modules).
///
/// # Arguments
///
/// * `parsed_chains` - Pre-parsed chains from executor modules
///
/// # Returns
///
/// Validation result or error
fn validate_alignment_with_chains(
    parsed_chains: &[crate::transform::iterator_stack::chain_parser::ParsedChain],
) -> Result<AlignmentValidationResult, SchemaError> {
    if parsed_chains.is_empty() {
        info!("⚠️ No parsed chains provided for alignment validation");
        return Ok(AlignmentValidationResult {
            valid: true,
            max_depth: 0,
            field_alignments: std::collections::HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        });
    }
    
    info!("📊 Validating alignment for {} pre-parsed chains", parsed_chains.len());
    
    // Perform alignment validation using the unified logic
    let validator = FieldAlignmentValidator::new();
    let alignment_result = validator.validate_alignment(parsed_chains)
        .map_err(|err| SchemaError::InvalidField(format!("Alignment validation failed: {}", err)))?;
    
    // Process and return the result
    process_unified_alignment_result(&alignment_result)
}

/// Validates field alignment from schema definition (original behavior).
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
///
/// # Returns
///
/// Validation result or error
fn validate_alignment_from_schema(
    schema: &crate::schema::types::json_schema::DeclarativeSchemaDefinition,
) -> Result<AlignmentValidationResult, SchemaError> {
    // Collect all expressions for alignment validation using unified function
    let mut all_expressions = collect_expressions_from_schema(schema);
    
    // Add HashRange special field expressions if this is a HashRange schema
    if let Some(key_config) = &schema.key {
        info!("🔑 Adding key expressions to alignment validation when present");
        if !key_config.hash_field.trim().is_empty() {
            all_expressions.push(("_hash_field".to_string(), key_config.hash_field.clone()));
        }
        if !key_config.range_field.trim().is_empty() {
            all_expressions.push(("_range_field".to_string(), key_config.range_field.clone()));
        }
    }
    
    if all_expressions.is_empty() {
        info!("⚠️ No expressions found for alignment validation");
        return Ok(AlignmentValidationResult {
            valid: true,
            max_depth: 0,
            field_alignments: std::collections::HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        });
    }
    
    info!("📊 Validating alignment for {} expressions", all_expressions.len());
    
    // Parse expressions for validation
    let mut parsed_chains = Vec::new();
    let mut parsing_errors = Vec::new();
    
    for (field_name, expression) in &all_expressions {
        match parse_atom_uuid_expression(expression) {
            Ok(parsed_chain) => {
                parsed_chains.push(parsed_chain);
            }
            Err(parse_error) => {
                parsing_errors.push((field_name.clone(), expression.clone(), parse_error));
            }
        }
    }
    
    if !parsing_errors.is_empty() {
        error!("🚨 Field alignment validation failed due to parsing errors: {}", 
               format_alignment_validation_errors(&parsing_errors.iter()
                   .map(|(field, expr, err)| format!("Field '{}' expression '{}': {}", field, expr, err))
                   .collect::<Vec<_>>()));
        return Err(create_parsing_error(&parsing_errors, "Field alignment validation"));
    }
    
    if parsed_chains.is_empty() {
        info!("⚠️ No valid expressions found for alignment validation");
        return Ok(AlignmentValidationResult {
            valid: true,
            max_depth: 0,
            field_alignments: std::collections::HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        });
    }
    
    // Perform alignment validation using the unified logic
    let validator = FieldAlignmentValidator::new();
    let alignment_result = validator.validate_alignment(&parsed_chains)
        .map_err(|err| SchemaError::InvalidField(format!("Alignment validation failed: {}", err)))?;
    
    // Process and return the result
    process_unified_alignment_result(&alignment_result)
}

/// Processes alignment validation results with unified error handling and logging.
///
/// # Arguments
///
/// * `alignment_result` - The result from field alignment validation
///
/// # Returns
///
/// Processed validation result
fn process_unified_alignment_result(
    alignment_result: &AlignmentValidationResult,
) -> Result<AlignmentValidationResult, SchemaError> {
    // Debug: Log all field alignments generated
    info!("🔍 Generated {} field alignments:", alignment_result.field_alignments.len());
    for (expression, alignment_info) in &alignment_result.field_alignments {
        info!("  📝 Expression: '{}' -> alignment: {:?}, depth: {}, requires_reducer: {}", 
              expression, alignment_info.alignment, alignment_info.depth, alignment_info.requires_reducer);
    }
    
    if !alignment_result.valid {
        let error_messages: Vec<String> = alignment_result.errors.iter()
            .map(|err| format!("{:?}: {}", err.error_type, err.message))
            .collect();
        error!("🚨 Field alignment validation failed: {}", format_alignment_validation_errors(&error_messages));
        
        return Err(SchemaError::InvalidField(format_alignment_validation_errors(&error_messages)));
    }
    
    if !alignment_result.warnings.is_empty() {
        let warning_messages: Vec<String> = alignment_result.warnings.iter()
            .map(|warn| format!("{:?}: {}", warn.warning_type, warn.message))
            .collect();
        info!("⚠️ Field alignment validation warnings: {}", warning_messages.join("; "));
    }
    
    info!("✅ Field alignment validation passed");
    Ok(alignment_result.clone())
}

