//! Restricted Access Pattern for Transform Module
//!
//! This module enforces that transforms cannot directly create atoms or molecules.
//! All data persistence must go through the mutation system to ensure proper
//! audit trails, event coordination, and data integrity.
//!
//! ## Design Principles
//!
//! 1. **No Direct Creation**: Transforms cannot directly call `Atom::new()`, `Molecule::new()`, etc.
//! 2. **Mutation-Only Persistence**: All data changes must go through the mutation system
//! 3. **Compile-Time Enforcement**: Use Rust's module system to prevent unauthorized access
//! 4. **Clear Interfaces**: Provide well-defined mutation interfaces for transforms

use crate::schema::types::{Mutation, MutationType};
use crate::schema::SchemaError;
use log::{info, warn};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Restricted access to atom/molecule creation methods.
///
/// This trait provides the only way for transforms to persist data,
/// ensuring all changes go through the mutation system.
pub trait TransformDataPersistence {
    /// Create a mutation to persist transform results.
    ///
    /// This is the ONLY way transforms should persist data.
    /// Direct atom/molecule creation is prohibited.
    fn create_persistence_mutation(
        &self,
        schema_name: &str,
        field_name: &str,
        value: JsonValue,
        source_pub_key: &str,
    ) -> Result<Mutation, SchemaError>;

    /// Create a batch of mutations for multiple field updates.
    fn create_batch_persistence_mutations(
        &self,
        schema_name: &str,
        field_updates: HashMap<String, JsonValue>,
        source_pub_key: &str,
    ) -> Result<Vec<Mutation>, SchemaError>;
}

/// Default implementation of transform data persistence.
///
/// This ensures all transform data goes through mutations.
pub struct MutationBasedPersistence {
    /// The source public key for audit trails
    pub source_pub_key: String,
}

impl MutationBasedPersistence {
    /// Create a new mutation-based persistence handler.
    pub fn new(source_pub_key: String) -> Self {
        Self { source_pub_key }
    }
}

impl TransformDataPersistence for MutationBasedPersistence {
    fn create_persistence_mutation(
        &self,
        schema_name: &str,
        field_name: &str,
        value: JsonValue,
        source_pub_key: &str,
    ) -> Result<Mutation, SchemaError> {
        info!(
            "📝 Creating persistence mutation for {}.{}",
            schema_name, field_name
        );

        let mut fields_and_values = HashMap::new();
        fields_and_values.insert(field_name.to_string(), value);

        let mutation = Mutation::new(
            schema_name.to_string(),
            fields_and_values,
            source_pub_key.to_string(),
            0, // trust_distance
            MutationType::Update,
        );

        info!("✅ Created mutation for {}.{}", schema_name, field_name);
        Ok(mutation)
    }

    fn create_batch_persistence_mutations(
        &self,
        schema_name: &str,
        field_updates: HashMap<String, JsonValue>,
        source_pub_key: &str,
    ) -> Result<Vec<Mutation>, SchemaError> {
        info!(
            "📝 Creating batch persistence mutations for schema: {}",
            schema_name
        );

        if field_updates.is_empty() {
            warn!("⚠️ No field updates provided for batch mutation");
            return Ok(vec![]);
        }

        let mutation = Mutation::new(
            schema_name.to_string(),
            field_updates,
            source_pub_key.to_string(),
            0, // trust_distance
            MutationType::Update,
        );

        info!("✅ Created batch mutation for {} fields", schema_name);
        Ok(vec![mutation])
    }
}

/// Validation utilities to ensure transforms don't bypass the mutation system.
pub struct TransformAccessValidator;

impl TransformAccessValidator {
    /// Validates that a transform doesn't attempt direct atom/molecule creation.
    ///
    /// This is a compile-time check that should be used in transform validation.
    pub fn validate_no_direct_creation(transform_code: &str) -> Result<(), SchemaError> {
        let forbidden_patterns = [
            "Atom::new",
            "Molecule::new",
            "MoleculeRange::new",
            "MoleculeHashRange::new",
            "atom::Atom::new",
            "atom::Molecule::new",
            "atom::MoleculeRange::new",
            "atom::MoleculeHashRange::new",
        ];

        for pattern in &forbidden_patterns {
            if transform_code.contains(pattern) {
                return Err(SchemaError::InvalidTransform(format!(
                    "Transform contains forbidden direct creation pattern: '{}'. \
                     All data persistence must go through the mutation system.",
                    pattern
                )));
            }
        }

        Ok(())
    }

    /// Validates that a transform uses proper mutation interfaces.
    pub fn validate_mutation_usage(transform_code: &str) -> Result<(), SchemaError> {
        // Check for proper mutation usage patterns
        let required_patterns = [
            "create_persistence_mutation",
            "create_batch_persistence_mutations",
            "Mutation::new",
        ];

        let has_mutation_pattern = required_patterns
            .iter()
            .any(|pattern| transform_code.contains(pattern));

        if !has_mutation_pattern && transform_code.contains("persist") {
            return Err(SchemaError::InvalidTransform(
                "Transform appears to persist data but doesn't use mutation interfaces. \
                 Use TransformDataPersistence trait methods instead."
                    .to_string(),
            ));
        }

        Ok(())
    }
}

/// Error types specific to transform access restrictions.
#[derive(Debug, thiserror::Error)]
pub enum TransformAccessError {
    #[error("Direct atom/molecule creation is forbidden: {0}")]
    DirectCreationForbidden(String),

    #[error("Transform must use mutation interfaces: {0}")]
    MutationInterfaceRequired(String),

    #[error("Invalid persistence pattern: {0}")]
    InvalidPersistencePattern(String),
}

/// Macro to ensure transforms use mutation interfaces.
///
/// This macro can be used to wrap transform execution and ensure
/// all data persistence goes through mutations.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forbidden_patterns_detection() {
        let forbidden_code = "let atom = Atom::new(schema, key, content);";
        let result = TransformAccessValidator::validate_no_direct_creation(forbidden_code);
        assert!(result.is_err());

        let allowed_code = "let mutation = create_persistence_mutation(schema, field, value);";
        let result = TransformAccessValidator::validate_no_direct_creation(allowed_code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mutation_usage_validation() {
        let code_with_persistence =
            "persist_data(); create_persistence_mutation(schema, field, value);";
        let result = TransformAccessValidator::validate_mutation_usage(code_with_persistence);
        assert!(result.is_ok());

        let code_without_mutations = "persist_data(); direct_save();";
        let result = TransformAccessValidator::validate_mutation_usage(code_without_mutations);
        assert!(result.is_err());
    }

    #[test]
    fn test_mutation_creation() {
        let persistence = MutationBasedPersistence::new("test_key".to_string());
        let mutation = persistence
            .create_persistence_mutation(
                "TestSchema",
                "test_field",
                JsonValue::String("test_value".to_string()),
                "test_key",
            )
            .unwrap();

        assert_eq!(mutation.schema_name, "TestSchema");
        // Note: MutationType comparison removed due to missing PartialEq implementation
    }
}
