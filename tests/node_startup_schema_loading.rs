use datafold::datafold_node::config::NodeConfig;
use datafold::datafold_node::DataFoldNode;
use tempfile::TempDir;

/// Test to verify that DataFoldNode starts successfully when no schema service is configured
#[tokio::test(flavor = "multi_thread")]
async fn test_node_starts_without_schema_service() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().join("test_db");

    // Create node configuration WITHOUT schema service URL
    let keypair = datafold::security::Ed25519KeyPair::generate().unwrap();
    let config = NodeConfig::new(test_db_path.to_path_buf())
        .with_network_listen_address("/ip4/127.0.0.1/tcp/9002")
        .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());

    // Attempt to create the node - should succeed without schema service URL
    let result = DataFoldNode::new(config).await;

    // Verify that node creation succeeds
    assert!(
        result.is_ok(),
        "Node creation should succeed when schema_service_url is None"
    );

    println!("✅ Node correctly starts when schema service is not configured!");
}

/// Test to verify that DataFoldNode can start with a mock schema service for testing
#[tokio::test(flavor = "multi_thread")]
async fn test_node_new_loads_schemas_for_testing() {
    // Create a temporary directory for this test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_db_path = temp_dir.path().join("test_db");

    // Create node configuration with mock schema service URL
    let keypair = datafold::security::Ed25519KeyPair::generate().unwrap();
    let config = NodeConfig::new(test_db_path.to_path_buf())
        .with_network_listen_address("/ip4/127.0.0.1/tcp/9003")
        .with_schema_service_url("test://mock")
        .with_identity(&keypair.public_key_base64(), &keypair.secret_key_base64());

    // Create a new node using DataFoldNode::new() with mock service
    let node = DataFoldNode::new(config)
        .await
        .expect("Failed to create DataFoldNode with mock schema service");

    // Get the fold_db to verify it was created successfully
    let fold_db = node.get_fold_db().await.expect("Failed to get FoldDB");
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
