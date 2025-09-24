//! Validation logic for mutations and field values.
//!
//! This module contains validation functions for field values and schema mutations,
//! ensuring data integrity and proper format compliance.

use crate::schema::types::{Schema, SchemaError, Mutation};
use crate::logging::features::{log_feature, LogFeature};
use crate::fold_db_core::infrastructure::factory::InfrastructureLogger;
use crate::validation::{
    validate_range_schema_mutation_unified
};

/// Range schema mutation validation using universal key configuration
///
/// This function validates Range schema mutations by checking for the presence and validity
/// of the range key field, using universal key configuration when available or falling back
/// to legacy range_key patterns.
pub fn validate_range_schema_mutation_format(
    schema: &Schema,
    mutation: &Mutation,
) -> Result<(), SchemaError> {
    log_feature!(
        LogFeature::Mutation,
        info,
        "🔍 Validating Range schema mutation format for schema: {}",
        schema.name
    );

    // Use the unified validation function
    validate_range_schema_mutation_unified(schema, mutation)?;

    InfrastructureLogger::log_operation_success(
        "MutationService",
        "Range schema mutation format validation passed",
        &format!("schema: {}", schema.name),
    );

    Ok(())
}
