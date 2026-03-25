use super::error::{StorageError, StorageResult};
use super::exemem_api_store::{ExememApiStore, ExememAuth};
use super::traits::{KvStore, NamespacedStore};
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;

/// NamespacedStore implementation for the Exemem Storage API.
///
/// `open_namespace` returns an `ExememApiStore` bound to that namespace.
/// No server call is needed because the namespace is just a field in each
/// request body — the Storage API Lambda resolves it to the correct
/// DynamoDB table on the server side.
pub struct ExememNamespacedStore {
    client: Arc<Client>,
    base_url: String,
    auth: ExememAuth,
}

impl ExememNamespacedStore {
    pub fn new(base_url: String, auth: ExememAuth) -> Self {
        Self {
            client: Arc::new(Client::new()),
            base_url,
            auth,
        }
    }
}

#[async_trait]
impl NamespacedStore for ExememNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let store = ExememApiStore::new(
            self.client.clone(),
            self.base_url.clone(),
            name.to_string(),
            self.auth.clone(),
        );
        Ok(Arc::new(store))
    }

    async fn list_namespaces(&self) -> StorageResult<Vec<String>> {
        Err(StorageError::InvalidOperation(
            "list_namespaces not supported via Exemem Storage API".to_string(),
        ))
    }

    async fn delete_namespace(&self, _name: &str) -> StorageResult<bool> {
        Err(StorageError::InvalidOperation(
            "delete_namespace not supported via Exemem Storage API".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_open_namespace_returns_store() {
        let store = ExememNamespacedStore::new(
            "https://api.example.com".to_string(),
            ExememAuth::UserHash("test_user".to_string()),
        );

        let ns = store.open_namespace("main").await.unwrap();
        assert_eq!(ns.backend_name(), "exemem-api");
    }

    #[tokio::test]
    async fn test_list_namespaces_unsupported() {
        let store = ExememNamespacedStore::new(
            "https://api.example.com".to_string(),
            ExememAuth::UserHash("test_user".to_string()),
        );

        let result = store.list_namespaces().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_namespace_unsupported() {
        let store = ExememNamespacedStore::new(
            "https://api.example.com".to_string(),
            ExememAuth::UserHash("test_user".to_string()),
        );

        let result = store.delete_namespace("main").await;
        assert!(result.is_err());
    }
}
