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
}
