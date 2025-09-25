#![allow(dead_code)]
use super::*;

fn create_test_config() -> NodeConfig {
    crate::testing_utils::TestDatabaseFactory::create_test_node_config()
}

#[test]
fn test_node_creation() {
    let config = create_test_config();
    let node = DataFoldNode::new(config);
    assert!(node.is_ok());
}

#[test]
fn test_add_trusted_node() {
    let config = create_test_config();
    let mut node = DataFoldNode::new(config).unwrap();

    assert!(node.add_trusted_node("test_node").is_ok());
    assert!(node.get_trusted_nodes().contains_key("test_node"));
    assert!(node.remove_trusted_node("test_node").is_ok());
    assert!(!node.get_trusted_nodes().contains_key("test_node"));
}

#[test]
fn test_node_config_default() {
    let config = NodeConfig::default();
    assert_eq!(config.storage_path, std::path::PathBuf::from("data"));
    assert_eq!(config.default_trust_distance, 1);
    assert_eq!(
        config.network_listen_address,
        "/ip4/0.0.0.0/tcp/0".to_string()
    );
}