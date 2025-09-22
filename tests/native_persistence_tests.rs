use std::{collections::HashMap, sync::Arc};

use datafold::{
    db_operations::DbOperations,
    persistence::{
        KeyConfig, NativePersistence, NativeRecordKey, NativeSchemaProvider, PersistenceError,
        SchemaDescription,
    },
    transform::native::{FieldDefinition, FieldType, FieldValue},
};
use tempfile::tempdir;

struct InMemorySchemaProvider {
    schemas: HashMap<String, SchemaDescription>,
}

impl InMemorySchemaProvider {
    fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    fn with_schema(mut self, schema: SchemaDescription) -> Self {
        self.schemas.insert(schema.name.clone(), schema);
        self
    }
}

impl NativeSchemaProvider for InMemorySchemaProvider {
    fn schema_for(&self, schema_name: &str) -> Result<SchemaDescription, PersistenceError> {
        self.schemas
            .get(schema_name)
            .cloned()
            .ok_or_else(|| PersistenceError::SchemaNotFound {
                schema: schema_name.to_string(),
            })
    }
}

fn create_persistence(provider: InMemorySchemaProvider) -> (NativePersistence, tempfile::TempDir) {
    let temp_dir = tempdir().expect("failed to create temp dir");
    let db = sled::Config::new()
        .path(temp_dir.path())
        .temporary(true)
        .open()
        .expect("failed to open sled database");
    let db_ops = Arc::new(DbOperations::new(db).expect("failed to create db operations"));
    let provider: Arc<dyn NativeSchemaProvider> = Arc::new(provider);
    let persistence = NativePersistence::new(db_ops, provider);
    (persistence, temp_dir)
}

fn user_schema() -> SchemaDescription {
    let mut fields = HashMap::new();
    fields.insert(
        "id".to_string(),
        FieldDefinition::new("id", FieldType::String),
    );
    fields.insert(
        "name".to_string(),
        FieldDefinition::new("name", FieldType::String),
    );
    fields.insert(
        "visits".to_string(),
        FieldDefinition::new("visits", FieldType::Integer).with_required(false),
    );

    SchemaDescription {
        name: "users".to_string(),
        key: KeyConfig::Single {
            key_field: "id".to_string(),
        },
        fields,
    }
}

fn session_schema() -> SchemaDescription {
    let mut fields = HashMap::new();
    fields.insert(
        "user_id".to_string(),
        FieldDefinition::new("user_id", FieldType::String),
    );
    fields.insert(
        "timestamp".to_string(),
        FieldDefinition::new("timestamp", FieldType::String),
    );
    fields.insert(
        "duration".to_string(),
        FieldDefinition::new("duration", FieldType::Integer).with_required(false),
    );

    SchemaDescription {
        name: "sessions".to_string(),
        key: KeyConfig::HashRange {
            hash_field: "user_id".to_string(),
            range_field: "timestamp".to_string(),
        },
        fields,
    }
}

#[test]
fn store_and_load_single_key() {
    let provider = InMemorySchemaProvider::new().with_schema(user_schema());
    let (persistence, _temp_dir) = create_persistence(provider);

    let mut record = HashMap::new();
    record.insert("id".to_string(), FieldValue::String("user-1".to_string()));
    record.insert("name".to_string(), FieldValue::String("Ada".to_string()));

    let key = persistence
        .store_data("users", &record)
        .expect("store should succeed");
    assert!(matches!(key, NativeRecordKey::Single(_)));

    let loaded = persistence
        .load_data("users", &key)
        .expect("load should succeed");

    assert_eq!(
        loaded.get("id"),
        Some(&FieldValue::String("user-1".to_string()))
    );
    assert_eq!(
        loaded.get("name"),
        Some(&FieldValue::String("Ada".to_string()))
    );
}

#[test]
fn missing_required_field_errors() {
    let provider = InMemorySchemaProvider::new().with_schema(user_schema());
    let (persistence, _temp_dir) = create_persistence(provider);

    let mut record = HashMap::new();
    record.insert("id".to_string(), FieldValue::String("user-1".to_string()));

    let err = persistence.store_data("users", &record).unwrap_err();
    assert!(matches!(
        err,
        PersistenceError::MissingRequiredField { field, .. } if field == "name"
    ));
}

#[test]
fn type_mismatch_errors() {
    let provider = InMemorySchemaProvider::new().with_schema(user_schema());
    let (persistence, _temp_dir) = create_persistence(provider);

    let mut record = HashMap::new();
    record.insert("id".to_string(), FieldValue::Integer(7));
    record.insert("name".to_string(), FieldValue::String("Grace".to_string()));

    let err = persistence.store_data("users", &record).unwrap_err();
    assert!(matches!(
        err,
        PersistenceError::FieldTypeMismatch { field, .. } if field == "id"
    ));
}

#[test]
fn optional_field_defaults_are_applied() {
    let provider = InMemorySchemaProvider::new().with_schema(user_schema());
    let (persistence, _temp_dir) = create_persistence(provider);

    let mut record = HashMap::new();
    record.insert("id".to_string(), FieldValue::String("user-2".to_string()));
    record.insert("name".to_string(), FieldValue::String("Lin".to_string()));

    let key = persistence
        .store_data("users", &record)
        .expect("store should succeed");

    let loaded = persistence
        .load_data("users", &key)
        .expect("load should succeed");

    assert_eq!(loaded.get("visits"), Some(&FieldValue::Integer(0)));
}

#[test]
fn composite_key_round_trip() {
    let provider = InMemorySchemaProvider::new().with_schema(session_schema());
    let (persistence, _temp_dir) = create_persistence(provider);

    let mut record = HashMap::new();
    record.insert(
        "user_id".to_string(),
        FieldValue::String("user-9".to_string()),
    );
    record.insert(
        "timestamp".to_string(),
        FieldValue::String("2025-09-24T10:00:00Z".to_string()),
    );
    record.insert("duration".to_string(), FieldValue::Integer(42));

    let key = persistence
        .store_data("sessions", &record)
        .expect("store should succeed");

    match &key {
        NativeRecordKey::Composite { hash, range } => {
            assert_eq!(hash, "user-9");
            assert_eq!(range, "2025-09-24T10:00:00Z");
        }
        _ => panic!("expected composite key"),
    }

    let loaded = persistence
        .load_data("sessions", &key)
        .expect("load should succeed");

    assert_eq!(loaded.get("duration"), Some(&FieldValue::Integer(42)));
}
