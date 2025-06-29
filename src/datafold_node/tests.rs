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

#[tokio::test]
async fn test_tcp_protocol_roundtrip() {
    use crate::datafold_node::tcp_protocol::{read_request, send_response};
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let req = read_request(&mut socket).await.unwrap().unwrap();
        assert_eq!(req["foo"], "bar");
        send_response(&mut socket, &json!({"ok": true}))
            .await
            .unwrap();
    });

    let mut client = TcpStream::connect(addr).await.unwrap();
    let request = json!({"foo": "bar"});
    let bytes = serde_json::to_vec(&request).unwrap();
    client.write_u32(bytes.len() as u32).await.unwrap();
    client.write_all(&bytes).await.unwrap();

    let resp_len = client.read_u32().await.unwrap();
    let mut resp_bytes = vec![0u8; resp_len as usize];
    client.read_exact(&mut resp_bytes).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&resp_bytes).unwrap();
    assert_eq!(resp, json!({"ok": true}));

    server.await.unwrap();
}

#[tokio::test]
async fn test_network_error_when_uninitialized() {
    let config = create_test_config();
    let node = DataFoldNode::new(config).unwrap();

    assert!(node.start_network().await.is_err());
    assert!(node.get_network_mut().await.is_err());
}
