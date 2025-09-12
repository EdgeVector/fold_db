//! Configuration utilities for eliminating duplicate initialization patterns
//!
//! This module consolidates common configuration patterns found throughout the codebase:
//! - Factory functions for HashMap::new() initialization (82 occurrences)
//! - Default field configuration builders
//! - Standard initialization patterns
//! - Unified configuration management

use std::collections::HashMap;
use serde_json::{Value as JsonValue, json};

/// Factory for creating standardized configuration objects
/// Eliminates 82+ HashMap::new() initialization patterns
pub struct ConfigFactory;

impl ConfigFactory {
    /// Create empty metadata HashMap - consolidates HashMap::new() pattern
    pub fn empty_metadata() -> HashMap<String, String> {
        HashMap::new()
    }

    /// Create empty fields HashMap - consolidates HashMap::new() pattern for schema fields
    pub fn empty_fields() -> HashMap<String, crate::schema::types::field::variant::FieldVariant> {
        HashMap::new()
    }

    /// Create empty transforms HashMap - consolidates HashMap::new() pattern for transforms
    pub fn empty_transforms() -> HashMap<String, crate::schema::types::Transform> {
        HashMap::new()
    }

    /// Create empty mutations HashMap - consolidates HashMap::new() pattern for mutations
    pub fn empty_mutations() -> HashMap<String, JsonValue> {
        HashMap::new()
    }

    /// Create empty variables HashMap - consolidates HashMap::new() pattern for variables
    pub fn empty_variables() -> HashMap<String, crate::transform::Value> {
        HashMap::new()
    }

    /// Create empty string to string HashMap - most common pattern
    pub fn empty_string_map() -> HashMap<String, String> {
        HashMap::new()
    }

    /// Create empty string to JsonValue HashMap - common in mutations/queries
    pub fn empty_json_map() -> HashMap<String, JsonValue> {
        HashMap::new()
    }

    /// Create metadata with single entry - common pattern
    pub fn single_metadata_entry(key: &str, value: &str) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert(key.to_string(), value.to_string());
        metadata
    }

    /// Create standard test metadata - consolidates test metadata patterns
    pub fn test_metadata() -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("test".to_string(), "true".to_string());
        metadata.insert("source".to_string(), "automated_test".to_string());
        metadata
    }

    /// Create transform input fields - consolidates transform field creation patterns
    pub fn transform_input_fields(field_names: Vec<&str>) -> HashMap<String, JsonValue> {
        let mut fields = HashMap::new();
        for (i, name) in field_names.iter().enumerate() {
            fields.insert(name.to_string(), json!(i * 10)); // Default test values
        }
        fields
    }

    /// Create standard mutation fields - consolidates mutation creation patterns
    pub fn standard_mutation_fields() -> HashMap<String, JsonValue> {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), json!("Test User"));
        fields.insert("email".to_string(), json!("test@example.com"));
        fields.insert("created_at".to_string(), json!(chrono::Utc::now().to_rfc3339()));
        fields
    }
}

/// Builder for complex configuration scenarios
pub struct ConfigBuilder<T> {
    map: HashMap<String, T>,
}

impl<T> ConfigBuilder<T> {
    /// Create new config builder
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Add entry to configuration
    pub fn with_entry(mut self, key: String, value: T) -> Self {
        self.map.insert(key, value);
        self
    }

    /// Add entry with string key
    pub fn with_str_key(mut self, key: &str, value: T) -> Self {
        self.map.insert(key.to_string(), value);
        self
    }

    /// Build the final HashMap
    pub fn build(self) -> HashMap<String, T> {
        self.map
    }
}

impl<T> Default for ConfigBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Specialized builders for common configuration types
pub type MetadataBuilder = ConfigBuilder<String>;
pub type FieldsBuilder = ConfigBuilder<JsonValue>;
pub type VariablesBuilder = ConfigBuilder<crate::transform::Value>;

/// Default field configuration builders
pub struct DefaultFieldConfig;

impl DefaultFieldConfig {
    /// Create default permissions policy
    pub fn permissions_policy() -> crate::permissions::types::policy::PermissionsPolicy {
        crate::permissions::types::policy::PermissionsPolicy::default()
    }

    /// Create default payment configuration
    pub fn payment_config() -> crate::fees::types::config::FieldPaymentConfig {
        crate::fees::types::config::FieldPaymentConfig::default()
    }

    /// Create default field common with empty HashMap
    pub fn field_common() -> crate::schema::types::field::common::FieldCommon {
        crate::schema::types::field::common::FieldCommon::new(
            Self::permissions_policy(),
            Self::payment_config(),
            ConfigFactory::empty_metadata(),
        )
    }

    /// Create default field common with test metadata
    pub fn field_common_with_test_metadata() -> crate::schema::types::field::common::FieldCommon {
        crate::schema::types::field::common::FieldCommon::new(
            Self::permissions_policy(),
            Self::payment_config(),
            ConfigFactory::test_metadata(),
        )
    }
}

/// Standard initialization patterns for common objects
pub struct StandardInitializers;

impl StandardInitializers {
    /// Initialize empty schema with standard configuration
    pub fn empty_schema(name: &str) -> crate::schema::types::Schema {
        crate::schema::types::Schema::new(name.to_string())
    }

    /// Initialize schema with standard test fields
    pub fn test_schema(name: &str) -> crate::schema::types::Schema {
        let mut schema = Self::empty_schema(name);
        
        // Add standard test fields
        let field = crate::schema::field_factory::FieldFactory::create_single_field();
        schema.fields.insert("test_field".to_string(), crate::schema::types::field::variant::FieldVariant::Single(field));
        
        schema
    }

    /// Initialize transform with standard test configuration
    pub fn test_transform(input_expr: &str, output_expr: &str) -> crate::schema::types::Transform {
        crate::schema::types::Transform::new(input_expr.to_string(), output_expr.to_string())
    }

    /// Initialize mutation with standard test configuration
    pub fn test_mutation(schema_name: &str) -> crate::schema::types::Mutation {
        crate::schema::types::Mutation {
            schema_name: schema_name.to_string(),
            mutation_type: crate::schema::types::MutationType::Create,
            fields_and_values: ConfigFactory::standard_mutation_fields(),
            pub_key: "test_key".to_string(),
            trust_distance: 0,
            synchronous: None,
        }
    }

    /// Initialize query with standard test configuration
    pub fn test_query(schema_name: &str, fields: Vec<&str>) -> crate::schema::types::Query {
        crate::schema::types::Query {
            schema_name: schema_name.to_string(),
            fields: fields.iter().map(|&s| s.to_string()).collect(),
            filter: None,
            trust_distance: 0,
            pub_key: "test_key".to_string(),
        }
    }

    /// Initialize atom with standard test configuration
    pub fn test_atom(schema_name: &str, content: JsonValue) -> crate::atom::Atom {
        crate::atom::Atom::new(
            schema_name.to_string(),
            "test_user".to_string(),
            content,
        )
    }

    /// Initialize molecule with standard test configuration
    pub fn test_molecule() -> crate::atom::Molecule {
        crate::atom::Molecule::new(
            uuid::Uuid::new_v4().to_string(),
            uuid::Uuid::new_v4().to_string(),
        )
    }
}

/// Environment configuration utilities
pub struct EnvironmentConfig;

impl EnvironmentConfig {
    /// Create standard test environment configuration
    pub fn test_environment() -> EnvironmentConfiguration {
        EnvironmentConfiguration {
            database_path: None, // Use temporary
            log_level: "debug".to_string(),
            message_bus_capacity: 1000,
            enable_metrics: true,
            test_mode: true,
            metadata: ConfigFactory::test_metadata(),
        }
    }

    /// Create production environment configuration
    pub fn production_environment(db_path: &str) -> EnvironmentConfiguration {
        EnvironmentConfiguration {
            database_path: Some(db_path.to_string()),
            log_level: "info".to_string(),
            message_bus_capacity: 10000,
            enable_metrics: true,
            test_mode: false,
            metadata: ConfigFactory::empty_metadata(),
        }
    }

    /// Create development environment configuration
    pub fn development_environment(db_path: &str) -> EnvironmentConfiguration {
        EnvironmentConfiguration {
            database_path: Some(db_path.to_string()),
            log_level: "debug".to_string(),
            message_bus_capacity: 5000,
            enable_metrics: true,
            test_mode: false,
            metadata: ConfigFactory::single_metadata_entry("environment", "development"),
        }
    }
}

/// Configuration structure for environment setup
#[derive(Debug, Clone)]
pub struct EnvironmentConfiguration {
    pub database_path: Option<String>,
    pub log_level: String,
    pub message_bus_capacity: usize,
    pub enable_metrics: bool,
    pub test_mode: bool,
    pub metadata: HashMap<String, String>,
}

impl EnvironmentConfiguration {
    /// Apply this configuration to create a test environment
    pub fn create_test_environment(&self) -> Result<crate::schema::field_factory::TestEnvironment, Box<dyn std::error::Error>> {
        if self.test_mode {
            // Use the new consolidated testing utilities
            let (db_ops, message_bus) = crate::testing_utils::TestDatabaseFactory::create_test_environment()?;
            let temp_dir = tempfile::tempdir()?;
            Ok(crate::schema::field_factory::TestEnvironment {
                db_ops,
                message_bus,
                _temp_dir: temp_dir,
            })
        } else {
            Err("Cannot create test environment from non-test configuration".into())
        }
    }
}

/// Pending operations initialization patterns
pub struct PendingOperationsInit;

impl PendingOperationsInit {
    /// Create empty pending operations map - consolidates pending operations patterns
    pub fn empty_pending_operations<T>() -> HashMap<String, T> {
        HashMap::new()
    }

    /// Create pending operations with correlation tracking
    pub fn pending_with_correlation() -> HashMap<String, PendingOperation> {
        HashMap::new()
    }

    /// Create standard pending operation entry
    pub fn create_pending_operation(operation_type: &str, correlation_id: String) -> PendingOperation {
        PendingOperation {
            operation_type: operation_type.to_string(),
            correlation_id,
            created_at: std::time::Instant::now(),
            metadata: ConfigFactory::empty_metadata(),
        }
    }
}

/// Represents a pending operation with correlation tracking
#[derive(Debug, Clone)]
pub struct PendingOperation {
    pub operation_type: String,
    pub correlation_id: String,
    pub created_at: std::time::Instant,
    pub metadata: HashMap<String, String>,
}

impl PendingOperation {
    /// Check if operation has timed out
    pub fn is_timed_out(&self, timeout: std::time::Duration) -> bool {
        self.created_at.elapsed() > timeout
    }

    /// Add metadata to pending operation
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Macro for creating HashMaps with initial values
#[macro_export]
macro_rules! hashmap {
    ($($key:expr => $value:expr),* $(,)?) => {
        {
            let mut map = std::collections::HashMap::new();
            $(
                map.insert($key, $value);
            )*
            map
        }
    };
}

/// Macro for creating field maps with default configurations
#[macro_export]
macro_rules! field_map {
    ($($field_name:expr => $field_value:expr),* $(,)?) => {
        {
            let mut fields = $crate::config_utils::ConfigFactory::empty_json_map();
            $(
                fields.insert($field_name.to_string(), $field_value);
            )*
            fields
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_factory_empty_methods() {
        let metadata = ConfigFactory::empty_metadata();
        assert!(metadata.is_empty());

        let fields = ConfigFactory::empty_json_map();
        assert!(fields.is_empty());
    }

    #[test]
    fn test_config_builder() {
        let config = MetadataBuilder::new()
            .with_str_key("key1", "value1".to_string())
            .with_str_key("key2", "value2".to_string())
            .build();

        assert_eq!(config.len(), 2);
        assert_eq!(config.get("key1"), Some(&"value1".to_string()));
        assert_eq!(config.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_standard_initializers() {
        let schema = StandardInitializers::test_schema("test_schema");
        assert_eq!(schema.name, "test_schema");
        assert!(!schema.fields.is_empty());

        let transform = StandardInitializers::test_transform("input", "output");
        assert_eq!(transform.inputs, Vec::<String>::new());
        assert_eq!(transform.output, "output");
    }

    #[test]
    fn test_environment_config() {
        let test_env = EnvironmentConfig::test_environment();
        assert!(test_env.test_mode);
        assert_eq!(test_env.log_level, "debug");

        let prod_env = EnvironmentConfig::production_environment("/tmp/prod_db");
        assert!(!prod_env.test_mode);
        assert_eq!(prod_env.log_level, "info");
        assert_eq!(prod_env.database_path, Some("/tmp/prod_db".to_string()));
    }

    #[test]
    fn test_pending_operations() {
        let pending = PendingOperationsInit::create_pending_operation("test_op", "correlation_123".to_string());
        assert_eq!(pending.operation_type, "test_op");
        assert_eq!(pending.correlation_id, "correlation_123");
        assert!(!pending.is_timed_out(std::time::Duration::from_secs(1)));
    }

    #[test]
    fn test_hashmap_macro() {
        let map = hashmap! {
            "key1".to_string() => "value1".to_string(),
            "key2".to_string() => "value2".to_string(),
        };

        assert_eq!(map.len(), 2);
        assert_eq!(map.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_field_map_macro() {
        let fields = field_map! {
            "name" => json!("test"),
            "age" => json!(25),
        };

        assert_eq!(fields.len(), 2);
        assert_eq!(fields.get("name"), Some(&json!("test")));
        assert_eq!(fields.get("age"), Some(&json!(25)));
    }
}