//! Validation utilities for transform execution.
//!
//! This module provides validation functions for schema structure, field alignment,
//! and other validation concerns in the transform execution framework.

use crate::transform::iterator_stack::chain_parser::{ChainParser, ParsedChain};
use crate::transform::iterator_stack::field_alignment::{FieldAlignmentValidator, AlignmentValidationResult};
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
    info!("🔍 Validating field alignment for schema: {}", schema.name);
    
    // Collect all expressions for alignment validation
    let mut all_expressions = Vec::new();
    
    // Add expressions from schema fields
    for (field_name, field_def) in &schema.fields {
        if let Some(atom_uuid_expr) = &field_def.atom_uuid {
            all_expressions.push((field_name.clone(), atom_uuid_expr.clone()));
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
                parsed_chains.push((field_name.clone(), parsed_chain));
            }
            Err(parse_error) => {
                parsing_errors.push((field_name.clone(), expression.clone(), parse_error));
            }
        }
    }
    
    if !parsing_errors.is_empty() {
        let error_messages: Vec<String> = parsing_errors.iter()
            .map(|(field, expr, err)| format!("Field '{}' expression '{}': {}", field, expr, err))
            .collect();
        error!("🚨 Field alignment validation failed due to parsing errors: {}", error_messages.join("; "));
        return Err(SchemaError::InvalidField(format!(
            "Field alignment validation failed due to parsing errors: {}", 
            error_messages.join("; ")
        )));
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
    
    // Perform alignment validation
    let chains_only: Vec<ParsedChain> = parsed_chains.iter().map(|(_, chain)| chain.clone()).collect();
    let validator = FieldAlignmentValidator::new();
    let alignment_result = validator.validate_alignment(&chains_only)
        .map_err(|err| SchemaError::InvalidField(format!("Alignment validation failed: {}", err)))?;
    
    process_alignment_validation_result(&alignment_result, &parsed_chains)
}

/// Processes alignment validation results and provides detailed feedback.
///
/// # Arguments
///
/// * `alignment_result` - The result from field alignment validation
/// * `parsed_chains` - The parsed chains for context
///
/// # Returns
///
/// Processed validation result
fn process_alignment_validation_result(
    alignment_result: &AlignmentValidationResult,
    parsed_chains: &[(String, ParsedChain)],
) -> Result<AlignmentValidationResult, SchemaError> {
    if !alignment_result.valid {
        let error_messages: Vec<String> = alignment_result.errors.iter()
            .map(|err| format!("{:?}: {}", err.error_type, err.message))
            .collect();
        error!("🚨 Field alignment validation failed: {}", error_messages.join("; "));
        
        // Log detailed information about the chains that failed
        for (field_name, parsed_chain) in parsed_chains {
            info!("🔍 Failed chain for field '{}': {:?}", field_name, parsed_chain);
        }
        
        return Err(SchemaError::InvalidField(format!(
            "Field alignment validation failed: {}", 
            error_messages.join("; ")
        )));
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

/// Parses atom UUID expressions for validation.
///
/// # Arguments
///
/// * `expression` - The expression to parse
///
/// # Returns
///
/// Parsed chain or error
fn parse_atom_uuid_expression(expression: &str) -> Result<ParsedChain, SchemaError> {
    let parser = ChainParser::new();
    parser.parse(expression).map_err(|err| {
        SchemaError::InvalidField(format!("Failed to parse expression '{}': {}", expression, err))
    })
}
