use super::error::{StorageError, StorageResult};
use super::traits::{ExecutionModel, FlushBehavior, KvStore};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::Arc;

/// Authentication method for the Exemem Storage API.
#[derive(Clone, Debug)]
pub enum ExememAuth {
    /// X-User-Hash header (dev/legacy)
    UserHash(String),
    /// X-API-Key header
    ApiKey(String),
    /// Authorization: Bearer <token>
    BearerToken(String),
}

/// KvStore implementation that routes operations through the Exemem Storage API.
///
/// Each instance is bound to a specific namespace. All keys and values are
/// base64-encoded in transit. The Storage API Lambda handles server-side routing,
/// user isolation, and namespace mapping.
pub struct ExememApiStore {
    client: Arc<Client>,
    base_url: String,
    namespace: String,
    auth: ExememAuth,
}

impl ExememApiStore {
    pub fn new(client: Arc<Client>, base_url: String, namespace: String, auth: ExememAuth) -> Self {
        Self {
            client,
            base_url,
            namespace,
            auth,
        }
    }

    fn endpoint(&self, action: &str) -> String {
        format!("{}/api/storage/{}", self.base_url, action)
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth {
            ExememAuth::UserHash(hash) => req.header("X-User-Hash", hash),
            ExememAuth::ApiKey(key) => req.header("X-API-Key", key),
            ExememAuth::BearerToken(token) => {
                req.header("Authorization", format!("Bearer {}", token))
            }
        }
    }

    async fn post(&self, action: &str, body: Value) -> StorageResult<Value> {
        let req = self.client.post(self.endpoint(action)).json(&body);
        let req = self.apply_auth(req);

        let response = req
            .send()
            .await
            .map_err(|e| StorageError::BackendError(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        let text = response.text().await.map_err(|e| {
            StorageError::BackendError(format!("Failed to read response body: {e}"))
        })?;

        let json: Value = serde_json::from_str(&text).map_err(|e| {
            StorageError::BackendError(format!(
                "Invalid JSON response (status {status}): {e}: {text}"
            ))
        })?;

        if json.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let error = json
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(StorageError::BackendError(format!(
                "Storage API error: {error}"
            )));
        }

        Ok(json)
    }

    fn encode_key(key: &[u8]) -> String {
        BASE64.encode(key)
    }

    fn encode_value(value: &[u8]) -> String {
        BASE64.encode(value)
    }

    fn decode_value(b64: &str) -> StorageResult<Vec<u8>> {
        BASE64
            .decode(b64)
            .map_err(|e| StorageError::BackendError(format!("Invalid base64 in response: {e}")))
    }
}

#[async_trait]
impl KvStore for ExememApiStore {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let body = json!({
            "namespace": self.namespace,
            "key": Self::encode_key(key),
        });

        let resp = self.post("get", body).await?;

        match resp.get("value") {
            Some(Value::String(b64)) => Ok(Some(Self::decode_value(b64)?)),
            Some(Value::Null) | None => Ok(None),
            _ => Err(StorageError::BackendError(
                "Unexpected 'value' type in get response".to_string(),
            )),
        }
    }

    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()> {
        let body = json!({
            "namespace": self.namespace,
            "key": Self::encode_key(key),
            "value": Self::encode_value(&value),
        });

        self.post("put", body).await?;
        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> StorageResult<bool> {
        let body = json!({
            "namespace": self.namespace,
            "key": Self::encode_key(key),
        });

        self.post("delete", body).await?;
        // The Storage API does not indicate whether the key existed,
        // so we return true on success.
        Ok(true)
    }

    async fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let body = json!({
            "namespace": self.namespace,
            "key": Self::encode_key(key),
        });

        let resp = self.post("exists", body).await?;

        resp.get("exists").and_then(|v| v.as_bool()).ok_or_else(|| {
            StorageError::BackendError("Missing 'exists' field in exists response".to_string())
        })
    }

    async fn scan_prefix(&self, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let body = json!({
            "namespace": self.namespace,
            "prefix": Self::encode_key(prefix),
        });

        let resp = self.post("scan-prefix", body).await?;

        let items = resp
            .get("items")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                StorageError::BackendError(
                    "Missing 'items' array in scan-prefix response".to_string(),
                )
            })?;

        let mut results = Vec::with_capacity(items.len());
        for item in items {
            let key_b64 = item.get("key").and_then(|v| v.as_str()).ok_or_else(|| {
                StorageError::BackendError("Missing 'key' in scan-prefix item".to_string())
            })?;
            let value_b64 = item.get("value").and_then(|v| v.as_str()).ok_or_else(|| {
                StorageError::BackendError("Missing 'value' in scan-prefix item".to_string())
            })?;

            results.push((Self::decode_value(key_b64)?, Self::decode_value(value_b64)?));
        }

        Ok(results)
    }

    async fn batch_put(&self, items: Vec<(Vec<u8>, Vec<u8>)>) -> StorageResult<()> {
        const BATCH_SIZE: usize = 25;
        for chunk in items.chunks(BATCH_SIZE) {
            let encoded_items: Vec<Value> = chunk
                .iter()
                .map(|(k, v)| {
                    json!({
                        "key": Self::encode_key(k),
                        "value": Self::encode_value(v),
                    })
                })
                .collect();

            let body = json!({
                "namespace": self.namespace,
                "items": encoded_items,
            });

            self.post("batch-put", body).await?;
        }

        Ok(())
    }

    async fn batch_delete(&self, keys: Vec<Vec<u8>>) -> StorageResult<()> {
        const BATCH_SIZE: usize = 25;
        for chunk in keys.chunks(BATCH_SIZE) {
            let encoded_items: Vec<Value> = chunk
                .iter()
                .map(|k| {
                    json!({
                        "key": Self::encode_key(k),
                    })
                })
                .collect();

            let body = json!({
                "namespace": self.namespace,
                "items": encoded_items,
            });

            self.post("batch-delete", body).await?;
        }

        Ok(())
    }

    async fn flush(&self) -> StorageResult<()> {
        // Storage API is eventually consistent, no flush needed
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "exemem-api"
    }

    fn execution_model(&self) -> ExecutionModel {
        ExecutionModel::Async
    }

    fn flush_behavior(&self) -> FlushBehavior {
        FlushBehavior::NoOp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_key() {
        let key = b"my_key";
        let encoded = ExememApiStore::encode_key(key);
        assert_eq!(encoded, BASE64.encode(b"my_key"));
    }

    #[test]
    fn test_encode_value() {
        let value = b"some_value";
        let encoded = ExememApiStore::encode_value(value);
        let decoded = ExememApiStore::decode_value(&encoded).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn test_decode_value_invalid_base64() {
        let result = ExememApiStore::decode_value("not!valid!base64!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_endpoint_construction() {
        let client = Arc::new(Client::new());
        let store = ExememApiStore::new(
            client,
            "https://api.example.com".to_string(),
            "main".to_string(),
            ExememAuth::UserHash("test_user".to_string()),
        );
        assert_eq!(
            store.endpoint("get"),
            "https://api.example.com/api/storage/get"
        );
        assert_eq!(
            store.endpoint("scan-prefix"),
            "https://api.example.com/api/storage/scan-prefix"
        );
    }

    #[test]
    fn test_backend_metadata() {
        let client = Arc::new(Client::new());
        let store = ExememApiStore::new(
            client,
            "https://api.example.com".to_string(),
            "main".to_string(),
            ExememAuth::ApiKey("test_key".to_string()),
        );
        assert_eq!(store.backend_name(), "exemem-api");
        assert_eq!(store.execution_model(), ExecutionModel::Async);
        assert_eq!(store.flush_behavior(), FlushBehavior::NoOp);
    }
}
