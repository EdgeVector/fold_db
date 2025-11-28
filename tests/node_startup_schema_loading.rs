use datafold::datafold_node::DataFoldNode;
use datafold::datafold_node::config::NodeConfig;
use tempfile::TempDir;

/// Test to verify that DataFoldNode FAILS to start when no schema service is configured
#[tokio::test]
async fn test_node_loads_schemas_for_testing_on_startup() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().join("test_db");
    
    // Create node configuration WITHOUT schema service URL
    let config = NodeConfig {
        storage_path: test_db_path.to_path_buf(),
        default_trust_distance: 1,
        network_listen_address: "/ip4/127.0.0.1/tcp/9002".to_string(),
        security_config: Default::default(),
        schema_service_url: None,
    };
    
    // Attempt to create the node - should fail without schema service URL
    let result = DataFoldNode::new(config).await;
    
    // Verify that node creation fails
    assert!(
        result.is_err(),
        "Node creation should fail when schema_service_url is None"
    );
    
    // Verify the error message mentions schema service
    if let Err(error) = result {
        let error_message = error.to_string();
        assert!(
            error_message.contains("Schema service") || error_message.contains("schema_service_url"),
            "Error message should mention schema service requirement: {}",
            error_message
        );
    }
    
    println!("✅ Node correctly fails to start when schema service is not configured!");
}

/// Test to verify that DataFoldNode can start with a mock schema service for testing
#[tokio::test]
async fn test_node_new_loads_schemas_for_testing() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().join("test_db");
    
    // Create node configuration with mock schema service URL
    let config = NodeConfig::new(test_db_path.to_path_buf())
        .with_network_listen_address("/ip4/127.0.0.1/tcp/9003")
        .with_schema_service_url("test://mock");
    
    // Create a new node using DataFoldNode::new() with mock service
    let node = DataFoldNode::new(config)
        .await
        .expect("Failed to create DataFoldNode with mock schema service");
    
    // Get the fold_db to verify it was created successfully
    let fold_db = node.get_fold_db().expect("Failed to get FoldDB");
    let schema_manager = fold_db.schema_manager();
    
    // Verify that NO schemas were auto-loaded (mock service doesn't load schemas)
    let schemas = schema_manager.get_schemas().expect("Failed to get schemas");
    
    // Verify that no schemas are loaded initially with mock service
    assert_eq!(
        schemas.len(), 
        0, 
        "No schemas should be auto-loaded with mock schema service"
    );
    
    println!("✅ DataFoldNode correctly starts with mock schema service for testing!");
}

