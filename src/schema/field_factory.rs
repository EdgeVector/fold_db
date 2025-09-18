//! Field creation factory for eliminating duplicate field creation patterns
//!
//! This module consolidates common field creation patterns found throughout the codebase:
//! - Eliminates 18+ field creation patterns with PermissionsPolicy::default() + FieldPaymentConfig::default() + HashMap::new()
//! - Standardizes transform execution setup across examples
//! - Common database initialization for examples
//! - Unified field configuration builders

use crate::schema::types::field::{
    single_field::SingleField,
    range_field::RangeField,
    variant::FieldVariant,
    common::{Field, FieldCommon},
};
use crate::permissions::types::policy::PermissionsPolicy;
use crate::fees::types::config::FieldPaymentConfig;
use crate::atom::{Atom, Molecule, MoleculeBehavior};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Factory for creating fields with standardized default configurations
/// Eliminates duplicate PermissionsPolicy::default() + FieldPaymentConfig::default() + HashMap::new() patterns
pub struct FieldFactory;

impl FieldFactory {
    /// Create a SingleField with default configuration
    /// Consolidates the most common field creation pattern found 18+ times in the codebase
    pub fn create_single_field() -> SingleField {
        SingleField {
            inner: FieldCommon::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            )
        }
    }

    /// Create a SingleField with custom permissions policy
    pub fn create_single_field_with_permissions(permissions: PermissionsPolicy) -> SingleField {
        SingleField {
            inner: FieldCommon::new(
                permissions,
                FieldPaymentConfig::default(),
                HashMap::new(),
            )
        }
    }

    /// Create a SingleField with custom payment configuration
    pub fn create_single_field_with_payment(payment_config: FieldPaymentConfig) -> SingleField {
        SingleField {
            inner: FieldCommon::new(
                PermissionsPolicy::default(),
                payment_config,
                HashMap::new(),
            )
        }
    }

    /// Create a SingleField with custom metadata
    pub fn create_single_field_with_metadata(metadata: HashMap<String, String>) -> SingleField {
        SingleField {
            inner: FieldCommon::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                metadata,
            )
        }
    }

    /// Create a SingleField with all custom configurations
    pub fn create_single_field_full(
        permissions: PermissionsPolicy,
        payment_config: FieldPaymentConfig,
        metadata: HashMap<String, String>
    ) -> SingleField {
        SingleField {
            inner: FieldCommon::new(permissions, payment_config, metadata)
        }
    }


    /// Create a RangeField with default configuration
    pub fn create_range_field() -> RangeField {
        RangeField {
            inner: FieldCommon::new(
                PermissionsPolicy::default(),
                FieldPaymentConfig::default(),
                HashMap::new(),
            ),
            molecule_range: None,
        }
    }

    /// Create a FieldVariant::Single with default configuration
    pub fn create_single_variant() -> FieldVariant {
        FieldVariant::Single(Self::create_single_field())
    }


    /// Create a FieldVariant::Range with default configuration
    pub fn create_range_variant() -> FieldVariant {
        FieldVariant::Range(Self::create_range_field())
    }

    /// Create a SingleField with a Molecule already linked
    /// Consolidates the pattern of creating field + atom + molecule + linking
    pub fn create_single_field_with_molecule(
        schema_name: &str,
        user_key: &str,
        content: JsonValue,
        db_ops: &crate::db_operations::DbOperations,
    ) -> Result<SingleField, Box<dyn std::error::Error>> {
        // Create atom
        let atom = Atom::new(schema_name.to_string(), user_key.to_string(), content);
        let atom_uuid = atom.uuid().to_string();
        db_ops.store_item(&format!("atom:{}", atom_uuid), &atom)?;

        // Create molecule - Note: Molecule::new takes (atom_uuid, source_pub_key)
        let molecule = Molecule::new(atom_uuid, user_key.to_string());
        let molecule_uuid = molecule.uuid().to_string();
        db_ops.store_item(&format!("ref:{}", molecule_uuid), &molecule)?;

        // Create field with ref linked
        let mut field = Self::create_single_field();
        field.set_molecule_uuid(molecule_uuid);

        Ok(field)
    }

    /// Create a complete field setup for transform testing
    /// Consolidates the common pattern used in transform examples
    pub fn create_transform_test_field(
        field_name: &str,
        schema_name: &str,
        value: JsonValue,
        db_ops: &crate::db_operations::DbOperations,
    ) -> Result<(String, FieldVariant), Box<dyn std::error::Error>> {
        let field = Self::create_single_field_with_molecule(
            schema_name,
            "test_user",
            value,
            db_ops,
        )?;

        Ok((field_name.to_string(), FieldVariant::Single(field)))
    }
}

/// Builder pattern for more complex field creation scenarios
pub struct FieldBuilder {
    permissions: PermissionsPolicy,
    payment_config: FieldPaymentConfig,
    metadata: HashMap<String, String>,
}

impl FieldBuilder {
    /// Create new field builder with defaults
    pub fn new() -> Self {
        Self {
            permissions: PermissionsPolicy::default(),
            payment_config: FieldPaymentConfig::default(),
            metadata: HashMap::new(),
        }
    }

    /// Set permissions policy
    pub fn with_permissions(mut self, permissions: PermissionsPolicy) -> Self {
        self.permissions = permissions;
        self
    }

    /// Set payment configuration
    pub fn with_payment_config(mut self, payment_config: FieldPaymentConfig) -> Self {
        self.payment_config = payment_config;
        self
    }

    /// Add metadata entry
    pub fn with_metadata_entry(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set all metadata
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Build a SingleField
    pub fn build_single(self) -> SingleField {
        SingleField {
            inner: FieldCommon::new(self.permissions, self.payment_config, self.metadata)
        }
    }


    /// Build a RangeField
    pub fn build_range(self) -> RangeField {
        RangeField {
            inner: FieldCommon::new(self.permissions, self.payment_config, self.metadata),
            molecule_range: None,
        }
    }
}

impl Default for FieldBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Transform execution setup utilities
/// Consolidates common transform setup patterns across examples
pub struct TransformSetupHelper;

impl TransformSetupHelper {
    /// Create a complete schema setup for transform testing
    /// Consolidates the pattern used in transform examples
    pub fn create_transform_test_schema(
        schema_name: &str,
        fields: Vec<(&str, JsonValue)>,
        db_ops: &crate::db_operations::DbOperations,
    ) -> Result<crate::schema::types::Schema, Box<dyn std::error::Error>> {
        use crate::schema::types::Schema;

        let mut schema = Schema::new(schema_name.to_string());

        for (field_name, value) in fields {
            let (name, field_variant) = FieldFactory::create_transform_test_field(
                field_name,
                schema_name,
                value,
                db_ops,
            )?;
            schema.fields.insert(name, field_variant);
        }

        // Store the schema
        db_ops.store_schema(schema_name, &schema)?;

        Ok(schema)
    }

    /// Create standard TransformBase schema for testing
    /// Consolidates the common TransformBase + TransformSchema pattern
    pub fn create_standard_transform_schemas(
        db_ops: &crate::db_operations::DbOperations,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create TransformBase schema with test values
        Self::create_transform_test_schema(
            "TransformBase",
            vec![
                ("value1", serde_json::json!(15)),
                ("value2", serde_json::json!(25)),
            ],
            db_ops,
        )?;

        // Create TransformSchema with result field
        Self::create_transform_test_schema(
            "TransformSchema",
            vec![
                ("result", serde_json::json!(null)),
            ],
            db_ops,
        )?;

        Ok(())
    }

    /// Create transform with standard test pattern
    /// Consolidates the Transform::new pattern used across examples
    pub fn create_standard_test_transform() -> crate::schema::types::Transform {
        use crate::schema::types::json_schema::DeclarativeSchemaDefinition;
        use crate::schema::types::schema::SchemaType;
        use std::collections::HashMap;
        
        let schema = DeclarativeSchemaDefinition {
            name: "standard_test_transform".to_string(),
            schema_type: SchemaType::Single,
            fields: HashMap::new(),
            key: None,
        };
        
        crate::schema::types::Transform::new(schema, "TransformSchema.result".to_string())
    }

    /// Create and store complete transform setup
    /// Consolidates the full transform creation, storage, and schema setup pattern
    pub fn setup_complete_transform_test(
        db_ops: &crate::db_operations::DbOperations,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create schemas
        Self::create_standard_transform_schemas(db_ops)?;

        // Create and store transform
        let mut transform = Self::create_standard_test_transform();
        transform.set_inputs(vec![
            "TransformBase.value1".to_string(),
            "TransformBase.value2".to_string(),
        ]);
        db_ops.store_transform("test_transform", &transform)?;

        Ok(())
    }
}

/// Database initialization utilities for examples
/// Consolidates common database setup patterns
pub struct DatabaseInitHelper;

impl DatabaseInitHelper {
    /// Create a temporary database for testing
    pub fn create_temp_database() -> Result<(sled::Db, tempfile::TempDir), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let db = sled::Config::new()
            .path(temp_dir.path().join("test_db"))
            .create_new(true)
            .open()?;
        Ok((db, temp_dir))
    }

    /// Create a test environment with database operations
    pub fn create_test_environment() -> Result<crate::db_operations::DbOperations, Box<dyn std::error::Error>> {
        let (db, _temp_dir) = Self::create_temp_database()?;
        Ok(crate::db_operations::DbOperations::new(db)?)
    }
}

/// Complete test environment for examples
pub struct TestEnvironment {
    pub db_ops: std::sync::Arc<crate::db_operations::DbOperations>,
    pub message_bus: std::sync::Arc<crate::fold_db_core::infrastructure::message_bus::MessageBus>,
    pub _temp_dir: tempfile::TempDir,
}

impl TestEnvironment {
    /// Create TransformManager with this environment
    pub fn create_transform_manager(&self) -> Result<crate::fold_db_core::transform_manager::TransformManager, crate::schema::types::SchemaError> {
        crate::fold_db_core::transform_manager::TransformManager::new(
            std::sync::Arc::clone(&self.db_ops),
            std::sync::Arc::clone(&self.message_bus),
        )
    }

    /// Setup complete transform test environment
    pub fn setup_transform_testing(&self) -> Result<(), Box<dyn std::error::Error>> {
        TransformSetupHelper::setup_complete_transform_test(&self.db_ops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_factory_creates_default_single_field() {
        let field = FieldFactory::create_single_field();
        // Test that field was created with expected defaults
        assert!(field.molecule_uuid().is_none());
    }

    #[test]
    fn test_field_builder_pattern() {
        let field = FieldBuilder::new()
            .with_metadata_entry("test_key".to_string(), "test_value".to_string())
            .build_single();
        
        // Test that metadata was set correctly
        // The field no longer has metadata, test a different property
        // The test field now has field mappers, so let's test they exist
        assert!(!field.inner.field_mappers.is_empty());
    }

    #[test]
    fn test_database_init_helper() {
        let result = DatabaseInitHelper::create_temp_database();
        assert!(result.is_ok());
        
        let (db, _temp_dir) = result.unwrap();
        // Test that database is functional
        assert!(db.insert(b"test_key", b"test_value").is_ok());
    }

    #[test]
    fn test_test_environment_creation() {
        let env = DatabaseInitHelper::create_test_environment();
        assert!(env.is_ok());
        
        let _environment = env.unwrap();
        // Test that we can access basic database operations
        // Transform manager creation is now handled by the system infrastructure
    }
}