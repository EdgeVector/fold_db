#[cfg(test)]
mod storage_abstraction_tests {
    use crate::storage::traits::*;
    use crate::storage::*;

    #[tokio::test]
    async fn test_sled_backend_basic_operations() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SledNamespacedStore::open(temp_dir.path()).unwrap();

        let kv = store.open_namespace("test").await.unwrap();

        // Test put and get
        kv.put(b"key1", b"value1".to_vec()).await.unwrap();
        let value = kv.get(b"key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));

        // Test exists
        assert!(kv.exists(b"key1").await.unwrap());
        assert!(!kv.exists(b"key2").await.unwrap());

        // Test delete
        assert!(kv.delete(b"key1").await.unwrap());
        assert!(!kv.exists(b"key1").await.unwrap());
    }

    #[tokio::test]
    async fn test_sled_backend_scan_prefix() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SledNamespacedStore::open(temp_dir.path()).unwrap();
        let kv = store.open_namespace("test").await.unwrap();

        kv.put(b"prefix:key1", b"value1".to_vec()).await.unwrap();
        kv.put(b"prefix:key2", b"value2".to_vec()).await.unwrap();
        kv.put(b"other:key3", b"value3".to_vec()).await.unwrap();

        let results = kv.scan_prefix(b"prefix:").await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_sled_backend_batch_operations() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SledNamespacedStore::open(temp_dir.path()).unwrap();
        let kv = store.open_namespace("test").await.unwrap();

        let items = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
        ];

        kv.batch_put(items).await.unwrap();

        assert!(kv.exists(b"key1").await.unwrap());
        assert!(kv.exists(b"key2").await.unwrap());

        kv.batch_delete(vec![b"key1".to_vec(), b"key2".to_vec()])
            .await
            .unwrap();

        assert!(!kv.exists(b"key1").await.unwrap());
        assert!(!kv.exists(b"key2").await.unwrap());
    }

    #[tokio::test]
    async fn test_typed_store_operations() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct TestData {
            name: String,
            value: i32,
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let store = SledNamespacedStore::open(temp_dir.path()).unwrap();
        let kv = store.open_namespace("test").await.unwrap();
        let typed = TypedKvStore::new(kv);

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        // Test put and get
        typed.put_item("key1", &data).await.unwrap();
        let retrieved: Option<TestData> = typed.get_item("key1").await.unwrap();
        assert_eq!(retrieved, Some(data.clone()));

        // Test list keys
        typed.put_item("key2", &data).await.unwrap();
        let keys = typed.list_keys_with_prefix("key").await.unwrap();
        assert_eq!(keys.len(), 2);

        // Test scan items
        let items: Vec<(String, TestData)> = typed.scan_items_with_prefix("key").await.unwrap();
        assert_eq!(items.len(), 2);

        // Test delete
        assert!(typed.delete_item("key1").await.unwrap());
        assert!(!typed.exists_item("key1").await.unwrap());
    }

    #[tokio::test]
    async fn test_namespaced_store_isolation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = SledNamespacedStore::open(temp_dir.path()).unwrap();

        let ns1 = store.open_namespace("namespace1").await.unwrap();
        let ns2 = store.open_namespace("namespace2").await.unwrap();

        ns1.put(b"key", b"value1".to_vec()).await.unwrap();
        ns2.put(b"key", b"value2".to_vec()).await.unwrap();

        let val1 = ns1.get(b"key").await.unwrap();
        let val2 = ns2.get(b"key").await.unwrap();

        assert_eq!(val1, Some(b"value1".to_vec()));
        assert_eq!(val2, Some(b"value2".to_vec()));
    }

    #[tokio::test]
    async fn test_inmemory_backend_operations() {
        let store = InMemoryNamespacedStore::new();
        let kv = store.open_namespace("test").await.unwrap();

        // Test basic operations
        kv.put(b"key1", b"value1".to_vec()).await.unwrap();
        let value = kv.get(b"key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));

        // Test scan
        kv.put(b"prefix:a", b"a".to_vec()).await.unwrap();
        kv.put(b"prefix:b", b"b".to_vec()).await.unwrap();
        let results = kv.scan_prefix(b"prefix:").await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_backend_name() {
        let sled_dir = tempfile::tempdir().unwrap();
        let sled_store = SledNamespacedStore::open(sled_dir.path()).unwrap();
        let sled_kv = sled_store.open_namespace("test").await.unwrap();
        assert_eq!(sled_kv.backend_name(), "sled");

        let mem_store = InMemoryNamespacedStore::new();
        let mem_kv = mem_store.open_namespace("test").await.unwrap();
        assert_eq!(mem_kv.backend_name(), "in-memory");
    }

    #[tokio::test]
    async fn test_execution_model_metadata() {
        use crate::storage::traits::{ExecutionModel, FlushBehavior};

        // Test Sled backend
        let sled_dir = tempfile::tempdir().unwrap();
        let sled_store = SledNamespacedStore::open(sled_dir.path()).unwrap();
        let sled_kv = sled_store.open_namespace("test").await.unwrap();
        assert_eq!(sled_kv.execution_model(), ExecutionModel::SyncWrapped);
        assert_eq!(sled_kv.flush_behavior(), FlushBehavior::Persists);

        // Test InMemory backend
        let mem_store = InMemoryNamespacedStore::new();
        let mem_kv = mem_store.open_namespace("test").await.unwrap();
        assert_eq!(mem_kv.execution_model(), ExecutionModel::SyncWrapped);
        assert_eq!(mem_kv.flush_behavior(), FlushBehavior::NoOp);

        // Test DynamoDB backend (if available)
        #[cfg(feature = "aws-backend")]
        {
            use crate::storage::dynamodb_backend::DynamoDbNamespacedStore;
            use aws_sdk_dynamodb::Client;

            let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .load()
                .await;
            let client = Client::new(&config);
            let dynamodb_store =
                DynamoDbNamespacedStore::new_with_prefix(client, "test-table".to_string());

            match dynamodb_store.open_namespace("test").await {
                Ok(dynamodb_kv) => {
                    assert_eq!(dynamodb_kv.execution_model(), ExecutionModel::Async);
                    assert_eq!(dynamodb_kv.flush_behavior(), FlushBehavior::NoOp);
                }
                Err(_) => {
                    // AWS not available, skip DynamoDB test
                }
            }
        }
    }

    #[tokio::test]
    async fn test_batch_put_items_typed() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct Item {
            id: u32,
        }

        let store = InMemoryNamespacedStore::new();
        let kv = store.open_namespace("test").await.unwrap();
        let typed = TypedKvStore::new(kv);

        let items = vec![
            ("item1".to_string(), Item { id: 1 }),
            ("item2".to_string(), Item { id: 2 }),
            ("item3".to_string(), Item { id: 3 }),
        ];

        typed.batch_put_items(items).await.unwrap();

        let retrieved: Vec<(String, Item)> = typed.scan_items_with_prefix("item").await.unwrap();
        assert_eq!(retrieved.len(), 3);
    }

    #[tokio::test]
    #[cfg(feature = "aws-backend")]
    async fn test_dynamodb_partition_key_logic() {
        use crate::storage::dynamodb_backend::DynamoDbKvStore;
        use aws_sdk_dynamodb::Client;
        use std::sync::Arc;

        // Create a mock client (we won't actually use it, just testing the key logic)
        // In a real test, you'd use LocalStack or a mock
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .load()
            .await;
        let client = Arc::new(Client::new(&config));

        // Test with user_id - partition key should be user_id
        let store_with_user = DynamoDbKvStore::new(
            client.clone(),
            "test-table".to_string(),
            Some("user_123".to_string()),
        );

        // Test partition key
        let pk = store_with_user.get_partition_key();
        assert_eq!(pk, "user_123");

        // Test sort key (should not have user_id prefix)
        let test_key = b"atom:abc123";
        let sort_key = store_with_user.make_sort_key(test_key);
        assert_eq!(sort_key, "atom:abc123");

        // Test without user_id - partition key should be "default"
        let store_without_user =
            DynamoDbKvStore::new(client.clone(), "test-table".to_string(), None);

        let pk_default = store_without_user.get_partition_key();
        assert_eq!(pk_default, "default");

        let sort_key_no_user = store_without_user.make_sort_key(test_key);
        assert_eq!(sort_key_no_user, "atom:abc123");
    }

    #[tokio::test]
    #[cfg(feature = "aws-backend")]
    async fn test_dynamodb_namespaced_store_user_isolation() {
        use crate::storage::dynamodb_backend::DynamoDbNamespacedStore;
        use aws_sdk_dynamodb::Client;

        // Create a mock client
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .load()
            .await;
        let client = Client::new(&config);

        // Test table name generation
        let store = DynamoDbNamespacedStore::new_with_prefix(client, "DataFoldStorage".to_string());
        let table_name = store.get_table_name_for_namespace("main");
        assert_eq!(table_name, "DataFoldStorage-main");

        // Test with user_id
        let store_with_user = DynamoDbNamespacedStore::new_with_prefix(
            Client::new(&config),
            "DataFoldStorage".to_string(),
        )
        .with_user_id("user_456".to_string());

        // Verify user_id is stored
        // (We can't directly access private fields, but we can test through open_namespace)
        // Note: This will fail if AWS credentials are not configured, which is expected in CI/test environments
        match store_with_user.open_namespace("test").await {
            Ok(kv) => {
                // The kv store should have user_id set, which will be used as the partition key
                assert_eq!(kv.backend_name(), "dynamodb");
            }
            Err(e) => {
                // If AWS is not available, that's ok - we're just testing the structure
                // The error should be about table creation/access, not about the store structure
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("DynamoDbError")
                        || error_msg.contains("dispatch failure")
                        || error_msg.contains("credentials")
                        || error_msg.contains("table"),
                    "Unexpected error: {}",
                    error_msg
                );
            }
        }
    }
}
