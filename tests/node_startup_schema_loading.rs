use datafold::datafold_node::DataFoldNode;
use datafold::datafold_node::config::NodeConfig;
use std::fs;
use tempfile::TempDir;

/// Test to verify that DataFoldNode loads available schemas from the 'available_schemas' folder on startup
#[tokio::test]
async fn test_node_loads_available_schemas_on_startup() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().join("test_db");
    
    // Verify that the available_schemas directory exists and contains the expected schemas
    let available_schemas_dir = std::env::current_dir().expect("Failed to get current directory").join("available_schemas");
    assert!(available_schemas_dir.exists(), "available_schemas directory should exist");
    
    // Debug: Check what files are in the available_schemas directory
    println!("📁 Available schemas directory: {:?}", available_schemas_dir);
    let files: Vec<_> = fs::read_dir(&available_schemas_dir).unwrap()
        .collect::<Result<Vec<_>, _>>().unwrap()
        .iter()
        .map(|e| e.file_name())
        .collect();
    println!("📁 Files in directory: {:?}", files);
    
    // Discover schema files in the available_schemas directory
    let expected_schema_names: Vec<String> = fs::read_dir(&available_schemas_dir).unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_str()?;
            if file_name_str.ends_with(".json") {
                Some(file_name_str[..file_name_str.len() - 5].to_string()) // Remove .json extension
            } else {
                None
            }
        })
        .collect();
    
    println!("🔍 Expected schema names: {:?}", expected_schema_names);
    assert!(!expected_schema_names.is_empty(), "Should have at least one schema file in available_schemas directory");
    
    // Create node configuration
    let config = NodeConfig {
        storage_path: test_db_path.to_path_buf(),
        default_trust_distance: 1,
        network_listen_address: "/ip4/127.0.0.1/tcp/9002".to_string(),
        security_config: Default::default(),
    };
    
    // Create the node
    let node = DataFoldNode::new(config).expect("Failed to create DataFoldNode");
    
    // Get the fold_db to check loaded schemas
    let fold_db = node.get_fold_db().expect("Failed to get FoldDB");
    let schema_manager = fold_db.schema_manager();
    
    // Verify that all three schemas were loaded
    let schemas = schema_manager.get_schemas().expect("Failed to get schemas");
    
    // Debug: Print what schemas were actually loaded
    println!("📋 Schemas loaded: {:?}", schemas.keys().collect::<Vec<_>>());
    println!("📋 Total schemas loaded: {}", schemas.len());
    
    // Check that all expected schemas from the available_schemas directory are loaded
    for expected_schema_name in &expected_schema_names {
        assert!(
            schemas.contains_key(expected_schema_name), 
            "Schema '{}' should be loaded from available_schemas directory", 
            expected_schema_name
        );
    }
    
    // Verify that the number of loaded schemas matches the number of schema files
    assert_eq!(
        schemas.len(), 
        expected_schema_names.len(), 
        "Number of loaded schemas should match number of schema files in available_schemas directory"
    );
    
    // Verify that each loaded schema has the correct structure
    for expected_schema_name in &expected_schema_names {
        let schema = schemas.get(expected_schema_name).expect(&format!("{} should exist", expected_schema_name));
        assert_eq!(schema.name, *expected_schema_name, "Schema name should match expected name");
        
        // Verify that the schema has a valid schema type
        match schema.schema_type {
            datafold::schema::types::SchemaType::Single => {},
            datafold::schema::types::SchemaType::Range { .. } => {},
            datafold::schema::types::SchemaType::HashRange { .. } => {},
        }
        
        println!("✅ Schema '{}' loaded with type: {:?}", expected_schema_name, schema.schema_type);
    }
    
    // Verify that all schemas are in "Available" state by default
    let schema_states = schema_manager.get_schema_states().expect("Failed to get schema states");
    
    for expected_schema_name in &expected_schema_names {
        assert_eq!(
            schema_states.get(expected_schema_name).copied().unwrap_or_default(),
            datafold::schema::SchemaState::Available,
            "Schema '{}' should be in Available state",
            expected_schema_name
        );
    }
    
    // Test that schemas are persisted to the database
    let schemas_with_states = schema_manager.get_schemas_with_states().expect("Failed to get schemas with states");
    assert_eq!(
        schemas_with_states.len(), 
        expected_schema_names.len(), 
        "Number of schemas with states should match number of schema files"
    );
    
    // Verify that we can find each schema by name
    for expected_schema_name in &expected_schema_names {
        let found_schema = schema_manager.get_schema(expected_schema_name).expect(&format!("Failed to get {}", expected_schema_name));
        assert!(found_schema.is_some(), "Should be able to retrieve {} by name", expected_schema_name);
        println!("✅ Schema '{}' can be retrieved by name: {:?}", expected_schema_name, found_schema.unwrap().schema_type);
    }
    
    println!("✅ All {} schemas loaded successfully on node startup!", expected_schema_names.len());
}

/// Test to verify that DataFoldNode::new() loads available schemas (used by database reset)
#[tokio::test]
async fn test_node_new_loads_available_schemas() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().join("test_db");
    
    // Verify that the available_schemas directory exists and contains the expected schemas
    let available_schemas_dir = std::env::current_dir().expect("Failed to get current directory").join("available_schemas");
    assert!(available_schemas_dir.exists(), "available_schemas directory should exist");
    
    // Discover schema files in the available_schemas directory
    let expected_schema_names: Vec<String> = fs::read_dir(&available_schemas_dir).unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_str()?;
            if file_name_str.ends_with(".json") {
                Some(file_name_str[..file_name_str.len() - 5].to_string()) // Remove .json extension
            } else {
                None
            }
        })
        .collect();
    
    assert!(!expected_schema_names.is_empty(), "Should have at least one schema file in available_schemas directory");
    
    // Create node configuration
    let config = NodeConfig {
        storage_path: test_db_path.to_path_buf(),
        default_trust_distance: 1,
        network_listen_address: "/ip4/127.0.0.1/tcp/9003".to_string(),
        security_config: Default::default(),
    };
    
    // Create a new node using DataFoldNode::new() (this is what database reset uses)
    let node = DataFoldNode::new(config).expect("Failed to create DataFoldNode with new()");
    
    // Get the fold_db to check loaded schemas
    let fold_db = node.get_fold_db().expect("Failed to get FoldDB");
    let schema_manager = fold_db.schema_manager();
    
    // Verify that all expected schemas were loaded
    let schemas = schema_manager.get_schemas().expect("Failed to get schemas");
    
    // Check that all expected schemas from the available_schemas directory are loaded
    for expected_schema_name in &expected_schema_names {
        assert!(
            schemas.contains_key(expected_schema_name), 
            "Schema '{}' should be loaded from available_schemas directory when using DataFoldNode::new()", 
            expected_schema_name
        );
    }
    
    // Verify that the number of loaded schemas matches the number of schema files
    assert_eq!(
        schemas.len(), 
        expected_schema_names.len(), 
        "Number of loaded schemas should match number of schema files in available_schemas directory"
    );
    
    // Verify that all schemas are in "Available" state by default
    let schema_states = schema_manager.get_schema_states().expect("Failed to get schema states");
    
    for expected_schema_name in &expected_schema_names {
        assert_eq!(
            schema_states.get(expected_schema_name).copied().unwrap_or_default(),
            datafold::schema::SchemaState::Available,
            "Schema '{}' should be in Available state when using DataFoldNode::new()",
            expected_schema_name
        );
    }
    
    println!("✅ All {} schemas loaded successfully with DataFoldNode::new() (database reset behavior)!", expected_schema_names.len());
}

