use datafold::db_operations::DbOperations;
use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
use datafold::schema::{SchemaCore, SchemaState};
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_discover_available_schemas_json_only() {
    let temp = TempDir::new().unwrap();
    let db = sled::open(temp.path().join("db")).unwrap();
    let db_ops = Arc::new(DbOperations::new(db).unwrap());
    let message_bus = Arc::new(MessageBus::new());
    let schema_core = SchemaCore::new(temp.path().to_str().unwrap(), Arc::clone(&db_ops), Arc::clone(&message_bus)).unwrap();

    let schemas = schema_core.discover_available_schemas().unwrap();
    let names: Vec<String> = schemas.iter().map(|s| s.name.clone()).collect();
    assert!(!names.is_empty());
    assert!(!names.iter().any(|n| n.contains("README")));
}

#[test]
fn test_available_schemas_not_loaded_by_default() {
    let temp = TempDir::new().unwrap();
    let db = sled::open(temp.path().join("db")).unwrap();
    let db_ops = Arc::new(DbOperations::new(db).unwrap());
    let message_bus = Arc::new(MessageBus::new());
    let schema_core = SchemaCore::new(temp.path().to_str().unwrap(), Arc::clone(&db_ops), Arc::clone(&message_bus)).unwrap();

    schema_core.load_schemas_from_disk().unwrap();

    // Available schemas should be discovered but not loaded
    let available = schema_core.list_available_schemas().unwrap();
    assert!(!available.is_empty());

    let loaded = schema_core.list_loaded_schemas().unwrap();
    assert!(loaded.is_empty());

    // States for available schemas should default to Available
    assert_eq!(schema_core.get_schema_state(&available[0]).unwrap(), SchemaState::Available);
}
