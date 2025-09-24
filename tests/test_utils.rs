//! Consolidated test utilities eliminating all duplicates from common.rs and test_utils.rs
//!
//! AGGRESSIVE CLEANUP: This module consolidates:
//! - 26+ duplicate tempfile setup patterns
//! - 18+ duplicate Arc::new(MessageBus::new()) patterns  
//! - 7+ duplicate sled::Config patterns
//! - 7+ duplicate NodeConfig patterns
//! - Multiple duplicate registration/transform creation patterns
//!
//! UPDATED: Now uses root test_db folder for consistent test database location

use datafold::datafold_node::config::NodeConfig;
use datafold::datafold_node::DataFoldNode;
use datafold::db_operations::DbOperations;
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::fees::SchemaPaymentConfig;
use datafold::fold_db_core::infrastructure::message_bus::{
    request_events::FieldValueSetRequest, MessageBus, NormalizedRequestParts,
};
use datafold::fold_db_core::managers::atom::AtomManager;
use datafold::fold_db_core::transform_manager::TransformManager;
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::schema::types::field::{FieldVariant, RangeField, SingleField};
use datafold::schema::types::key_config::KeyConfig;
use datafold::schema::types::{Schema, SchemaError, SchemaType, Transform, TransformRegistration, DeclarativeSchemaDefinition};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

/// Default wait duration for asynchronous test operations
pub const TEST_WAIT_MS: u64 = 100;

/// Path to the root test database directory
pub const TEST_DB_PATH: &str = "test_db";

/// Extract the nested normalized fields map from universal key snapshots.
#[allow(dead_code)]
pub fn normalized_fields(
    fields: &serde_json::Map<String, serde_json::Value>,
) -> &serde_json::Map<String, serde_json::Value> {
    fields
        .get("fields")
        .and_then(|value| value.as_object())
        .unwrap_or(fields)
}

/// Build a normalized FieldValueSetRequest for tests without hash/range keys.
#[allow(dead_code)]
pub fn normalized_field_value_request(
    correlation_id: impl Into<String>,
    schema_name: impl Into<String>,
    field_name: impl Into<String>,
    field_value: JsonValue,
    source_pub_key: impl Into<String>,
) -> FieldValueSetRequest {
    normalized_field_value_request_with_keys(
        correlation_id,
        schema_name,
        field_name,
        field_value,
        None,
        None,
        source_pub_key,
    )
}

/// Build a normalized FieldValueSetRequest for tests with optional hash/range keys.
#[allow(dead_code)]
pub fn normalized_field_value_request_with_keys(
    correlation_id: impl Into<String>,
    schema_name: impl Into<String>,
    field_name: impl Into<String>,
    field_value: JsonValue,
    hash: Option<&str>,
    range: Option<&str>,
    source_pub_key: impl Into<String>,
) -> FieldValueSetRequest {
    let correlation_id = correlation_id.into();
    let schema_name = schema_name.into();
    let field_name = field_name.into();
    let source_pub_key = source_pub_key.into();

    let mut fields = serde_json::Map::new();
    fields.insert(field_name.clone(), field_value);

    FieldValueSetRequest::from_normalized_parts(NormalizedRequestParts {
        correlation_id,
        schema_name,
        field_name,
        fields,
        hash: hash.map(|value| value.to_string()),
        range: range.map(|value| value.to_string()),
        source_pub_key,
        mutation_hash: None,
    })
}

/// Build a Single schema with optional universal key configuration for tests.
#[allow(dead_code)]
pub fn single_schema_with_key(
    name: &str,
    field_name: &str,
    hash_field: Option<&str>,
    range_field: Option<&str>,
) -> Schema {
    let mut fields = HashMap::new();
    fields.insert(
        field_name.to_string(),
        FieldVariant::Single(SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        )),
    );

    let key = match (hash_field, range_field) {
        (None, None) => None,
        _ => Some(KeyConfig {
            hash_field: hash_field.unwrap_or("").to_string(),
            range_field: range_field.unwrap_or("").to_string(),
        }),
    };

    Schema {
        name: name.to_string(),
        schema_type: SchemaType::Single,
        key,
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    }
}

/// Build a Range schema configured for either legacy or universal key scenarios.
#[allow(dead_code)]
pub fn range_schema_with_key(
    name: &str,
    field_name: &str,
    legacy_range_key: &str,
    hash_field: Option<&str>,
    range_field: Option<&str>,
) -> Schema {
    let mut fields = HashMap::new();
    fields.insert(
        field_name.to_string(),
        FieldVariant::Range(RangeField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        )),
    );

    let key = match (hash_field, range_field) {
        (None, None) => None,
        _ => Some(KeyConfig {
            hash_field: hash_field.unwrap_or("").to_string(),
            range_field: range_field.unwrap_or(legacy_range_key).to_string(),
        }),
    };

    Schema {
        name: name.to_string(),
        schema_type: SchemaType::Range {
            range_key: legacy_range_key.to_string(),
        },
        key,
        fields,
        payment_config: SchemaPaymentConfig::default(),
        hash: None,
    }
}

/// Single unified test fixture eliminating all duplication
#[allow(dead_code)]
pub struct TestFixture {
    pub transform_manager: Arc<TransformManager>,
    pub message_bus: Arc<MessageBus>,
    pub db_ops: Arc<DbOperations>,
    pub atom_manager: datafold::fold_db_core::managers::atom::AtomManager,
    pub _temp_dir: TempDir,
}

/// Test fixture that uses the root test_db folder for consistent database location
#[allow(dead_code)]
pub struct TestDbFixture {
    pub transform_manager: Arc<TransformManager>,
    pub message_bus: Arc<MessageBus>,
    pub db_ops: Arc<DbOperations>,
    pub atom_manager: datafold::fold_db_core::managers::atom::AtomManager,
}

/// Extended fixture for full integration testing
#[allow(dead_code)]
pub struct CommonTestFixture {
    pub common: TestFixture,
    pub node: DataFoldNode,
    pub _temp_dir: TempDir,
}

/// Specialized fixture for orchestrator testing
#[allow(dead_code)]
pub struct DirectEventTestFixture {
    pub transform_manager: Arc<TransformManager>,
    pub transform_orchestrator:
        datafold::fold_db_core::orchestration::transform_orchestrator::TransformOrchestrator,
    pub message_bus: Arc<MessageBus>,
    pub db_ops: Arc<DbOperations>,
    pub _temp_dir: TempDir,
}

#[allow(dead_code)]
impl TestDbFixture {
    /// Create a test fixture using the root test_db folder
    ///
    /// This fixture uses a consistent database location for all tests,
    /// making it easier to debug and inspect test data.
    pub fn new() -> Result<Self, SchemaError> {
        // Ensure the test_db directory exists
        let test_db_path = Path::new(TEST_DB_PATH);
        if !test_db_path.exists() {
            std::fs::create_dir_all(test_db_path).map_err(|e| {
                SchemaError::InvalidData(format!("Failed to create test_db directory: {}", e))
            })?;
        }

        // Open database using the test_db folder
        let db = sled::Config::new().path(test_db_path).open().map_err(|e| {
            SchemaError::InvalidData(format!("Failed to open test database: {}", e))
        })?;

        let db_ops = Arc::new(DbOperations::new(db).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to create DbOperations: {}", e))
        })?);

        // Unified MessageBus creation
        let message_bus = Arc::new(MessageBus::new());

        let transform_manager =
            TransformManager::new(Arc::clone(&db_ops), Arc::clone(&message_bus))?;

        // Create AtomManager to handle FieldValueSetRequest events
        let atom_manager = AtomManager::new((*db_ops).clone(), Arc::clone(&message_bus));

        Ok(Self {
            transform_manager: Arc::new(transform_manager),
            message_bus,
            db_ops,
            atom_manager,
        })
    }

    /// Clean up the test database by clearing all data
    ///
    /// This is useful for tests that need a clean state.
    pub fn cleanup(&self) -> Result<(), SchemaError> {
        // Clear all trees in the database
        let db = self.db_ops.db();
        for tree_name in db.tree_names() {
            if let Ok(tree) = db.open_tree(&tree_name) {
                tree.clear().map_err(|e| {
                    SchemaError::InvalidData(format!(
                        "Failed to clear tree {}: {}",
                        String::from_utf8_lossy(&tree_name),
                        e
                    ))
                })?;
            }
        }
        Ok(())
    }

    /// Get the path to the test database
    pub fn db_path(&self) -> &Path {
        Path::new(TEST_DB_PATH)
    }
}

#[allow(dead_code)]
impl TestFixture {
    /// Unified test fixture creation - eliminates 26+ tempfile duplicate patterns
    #[allow(dead_code)]
    pub fn new() -> Result<Self, SchemaError> {
        let temp_dir = tempfile::tempdir().map_err(|e| {
            SchemaError::InvalidData(format!("Failed to create temp directory: {}", e))
        })?;

        // Unified database setup - consolidates 7+ sled::Config patterns
        let db = sled::Config::new()
            .path(temp_dir.path())
            .temporary(true)
            .open()
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to open temporary database: {}", e))
            })?;

        let db_ops = Arc::new(DbOperations::new(db).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to create DbOperations: {}", e))
        })?);

        // Unified MessageBus creation - consolidates 18+ duplicate patterns
        let message_bus = Arc::new(MessageBus::new());

        let transform_manager =
            TransformManager::new(Arc::clone(&db_ops), Arc::clone(&message_bus))?;

        // Create AtomManager to handle FieldValueSetRequest events
        let atom_manager = AtomManager::new((*db_ops).clone(), Arc::clone(&message_bus));

        Ok(Self {
            transform_manager: Arc::new(transform_manager),
            message_bus,
            db_ops,
            atom_manager,
            _temp_dir: temp_dir,
        })
    }

    /// Unified transform creation - consolidates transform creation patterns
    pub fn create_sample_transform() -> Transform {

        let schema = DeclarativeSchemaDefinition::new(
            "test_schema".to_string(),
            SchemaType::Single,
            None,
            HashMap::new(),
        );

        Transform::from_declarative_schema(
            schema,
            vec!["test.input1".to_string()],
            "test.output".to_string(),
        )
    }

    /// Unified registration creation - consolidates registration patterns
    pub fn create_sample_registration() -> TransformRegistration {
        TransformRegistration {
            transform_id: "test_transform".to_string(),
            transform: Self::create_sample_transform(),
            input_molecules: vec!["molecule1".to_string()],
            input_names: vec!["input1".to_string()],
            trigger_fields: vec!["test.field1".to_string()],
            output_molecule: "output_molecule".to_string(),
            schema_name: "test".to_string(),
            field_name: "output".to_string(),
        }
    }

    /// Unified named transform creation
    pub fn create_named_transform(transform_id: &str) -> Transform {
        use datafold::schema::types::schema::SchemaType;
        use std::collections::HashMap;

        let schema = DeclarativeSchemaDefinition::new(
            "test_schema".to_string(),
            SchemaType::Single,
            None,
            HashMap::new(),
        );

        Transform::from_declarative_schema(
            schema,
            vec!["test.input1".to_string()],
            format!("test.{}", transform_id),
        )
    }

    /// Unified named registration creation
    pub fn create_named_registration(transform_id: &str) -> TransformRegistration {
        TransformRegistration {
            transform_id: transform_id.to_string(),
            transform: Self::create_named_transform(transform_id),
            input_molecules: vec![format!("{}_molecule1", transform_id)],
            input_names: vec!["input1".to_string()],
            trigger_fields: vec![format!("test.{}_field", transform_id)],
            output_molecule: format!("{}_output_molecule", transform_id),
            schema_name: "test".to_string(),
            field_name: transform_id.to_string(),
        }
    }

    /// Unified orchestrator fixture creation
    pub fn new_with_orchestrator() -> Result<DirectEventTestFixture, Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;

        let db = sled::Config::new()
            .path(temp_dir.path())
            .temporary(true)
            .open()?;

        let db_ops = Arc::new(DbOperations::new(db)?);
        let message_bus = Arc::new(MessageBus::new());

        let transform_manager = Arc::new(TransformManager::new(
            Arc::clone(&db_ops),
            Arc::clone(&message_bus),
        )?);

        let orchestrator_tree = {
            let orchestrator_db = sled::Config::new()
                .path(temp_dir.path().join("orchestrator"))
                .temporary(true)
                .open()?;
            orchestrator_db.open_tree("transform_orchestrator")?
        };

        let transform_orchestrator = datafold::fold_db_core::orchestration::transform_orchestrator::TransformOrchestrator::new(
            Arc::clone(&transform_manager) as Arc<dyn datafold::fold_db_core::transform_manager::types::TransformRunner>,
            orchestrator_tree,
            Arc::clone(&message_bus),
            Arc::clone(&db_ops),
        );

        Ok(DirectEventTestFixture {
            transform_manager,
            transform_orchestrator,
            message_bus,
            db_ops,
            _temp_dir: temp_dir,
        })
    }

    /// Unified wait function - consolidates sleep patterns
    pub async fn wait_for_async_operation() {
        tokio::time::sleep(std::time::Duration::from_millis(TEST_WAIT_MS)).await;
    }

    /// Unified correlation ID generation
    pub fn generate_correlation_id(prefix: &str) -> String {
        format!("{}_{}", prefix, Uuid::new_v4())
    }
}

#[allow(dead_code)]
impl CommonTestFixture {
    /// Create with schemas - consolidates NodeConfig patterns
    #[allow(dead_code)]
    pub async fn new_with_schemas() -> Result<CommonTestFixture, SchemaError> {
        let temp_dir = tempfile::tempdir().map_err(|e| {
            SchemaError::InvalidData(format!("Failed to create temp directory: {}", e))
        })?;

        // Unified NodeConfig setup - consolidates 7+ duplicate patterns
        let config = NodeConfig::new(temp_dir.path().to_path_buf());
        let mut node = DataFoldNode::load(config)
            .await
            .map_err(|e| SchemaError::InvalidData(format!("Failed to load DataFoldNode: {}", e)))?;

        // Explicitly load transform schemas from available_schemas
        node.load_schema_from_file("available_schemas/TransformBase.json")
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to load TransformBase schema: {}", e))
            })?;
        node.load_schema_from_file("available_schemas/TransformSchema.json")
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to load TransformSchema schema: {}", e))
            })?;

        let node_clone = node.clone();
        {
            let fold_db = node_clone.get_fold_db().map_err(|e| {
                SchemaError::InvalidData(format!("Failed to get FoldDB from node: {}", e))
            })?;

            fold_db
                .schema_manager()
                .approve_schema("TransformBase")
                .map_err(|e| {
                    SchemaError::InvalidData(format!(
                        "Failed to approve TransformBase schema: {}",
                        e
                    ))
                })?;
            fold_db
                .schema_manager()
                .approve_schema("TransformSchema")
                .map_err(|e| {
                    SchemaError::InvalidData(format!(
                        "Failed to approve TransformSchema schema: {}",
                        e
                    ))
                })?;

            fold_db
                .transform_manager()
                .reload_transforms()
                .map_err(|e| {
                    SchemaError::InvalidData(format!("Failed to reload transforms: {}", e))
                })?;
        }

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        Ok(Self::new_from_node(node, temp_dir))
    }

    /// Create basic fixture
    pub fn new() -> Result<Self, SchemaError> {
        let temp_dir = tempfile::tempdir().map_err(|e| {
            SchemaError::InvalidData(format!("Failed to create temp directory: {}", e))
        })?;

        let basic_fixture = TestFixture::new()?;

        let config = NodeConfig::new(temp_dir.path().to_path_buf());
        let node = DataFoldNode::new(config).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to create DataFoldNode: {}", e))
        })?;

        Ok(Self {
            common: basic_fixture,
            node,
            _temp_dir: temp_dir,
        })
    }

    /// Create from existing node
    fn new_from_node(node: DataFoldNode, temp_dir: TempDir) -> Self {
        let node_clone = node.clone();
        let fold_db = node_clone.get_fold_db().unwrap();

        let db_ops = fold_db.get_db_ops();
        let message_bus = fold_db.message_bus();
        let transform_manager = fold_db.transform_manager();
        let atom_manager = fold_db.atom_manager().clone();

        let common = TestFixture {
            transform_manager,
            message_bus,
            db_ops,
            atom_manager,
            _temp_dir: tempfile::tempdir().expect("Should create temp dir"),
        };

        Self {
            common,
            node,
            _temp_dir: temp_dir,
        }
    }

    /// Delegate to TestFixture methods to avoid duplication
    pub fn create_sample_registration() -> TransformRegistration {
        TestFixture::create_sample_registration()
    }
}

/// Schema loading utilities for tests
pub struct SchemaLoader;

impl SchemaLoader {
    /// Load a regular schema from the available_schemas directory
    pub fn load_schema_from_available_schemas(
        fold_db: &mut datafold::fold_db_core::FoldDB,
        schema_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let schema_path = format!("available_schemas/{}.json", schema_name);
        fold_db.load_schema_from_file(&schema_path)?;
        fold_db.approve_schema(schema_name)?;
        Ok(())
    }

    /// Load a declarative schema from the available_schemas directory
    pub fn load_declarative_schema_from_available_schemas(
        fold_db: &mut datafold::fold_db_core::FoldDB,
        schema_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let schema_path = format!("available_schemas/{}.json", schema_name);
        fold_db.load_schema_from_file(&schema_path)?;
        fold_db.approve_schema(schema_name)?;
        Ok(())
    }
}
