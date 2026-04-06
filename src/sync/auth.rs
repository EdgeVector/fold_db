use super::error::{SyncError, SyncResult};
use super::s3::PresignedUrl;
use reqwest::Client;
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Authentication method for the sync auth Lambda.
#[derive(Clone)]
pub enum SyncAuth {
    ApiKey(String),
    BearerToken(String),
}

impl std::fmt::Debug for SyncAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncAuth::ApiKey(_) => f.write_str("SyncAuth::ApiKey(****)"),
            SyncAuth::BearerToken(_) => f.write_str("SyncAuth::BearerToken(****)"),
        }
    }
}

/// Callback type for refreshing authentication credentials.
///
/// Called when the sync engine receives a 401 from the auth Lambda.
/// Should return a fresh `SyncAuth` (e.g., by re-registering with Exemem).
pub type AuthRefreshCallback =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<SyncAuth, String>> + Send>> + Send + Sync>;

/// Response from the auth Lambda listing available S3 objects.
#[derive(Debug, Deserialize)]
pub struct ListObjectsResponse {
    pub ok: bool,
    #[serde(default)]
    pub objects: Vec<S3ObjectInfo>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct S3ObjectInfo {
    pub key: String,
    pub size: u64,
    pub last_modified: String,
}

/// Response from the auth Lambda with presigned URLs.
#[derive(Debug, Deserialize)]
pub struct PresignedResponse {
    pub ok: bool,
    #[serde(default)]
    pub urls: Vec<PresignedUrl>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Response from the auth Lambda for lock operations.
#[derive(Debug, Deserialize)]
pub struct LockResponse {
    pub ok: bool,
    #[serde(default)]
    pub locked_by: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Client for the sync auth Lambda.
///
/// The auth Lambda:
/// 1. Validates authentication (API key or bearer token)
/// 2. Returns presigned S3 URLs scoped to the user's prefix
/// 3. Manages device locks
///
/// The client never gets AWS credentials — only time-limited presigned URLs.
pub struct AuthClient {
    http: Arc<Client>,
    base_url: String,
    auth: Arc<RwLock<SyncAuth>>,
}

impl AuthClient {
    pub fn new(http: Arc<Client>, base_url: String, auth: SyncAuth) -> Self {
        Self {
            http,
            base_url,
            auth: Arc::new(RwLock::new(auth)),
        }
    }

    /// Replace the current authentication credential with a fresh one.
    ///
    /// Called after a successful token refresh to update the in-memory credential
    /// so subsequent requests use the new token.
    pub async fn update_auth(&self, new_auth: SyncAuth) {
        *self.auth.write().await = new_auth;
    }

    /// Check if the current auth credential is a bearer token.
    ///
    /// Useful for callers to decide whether a refresh is needed (bearer tokens
    /// expire, API keys do not).
    pub async fn is_bearer_token(&self) -> bool {
        matches!(&*self.auth.read().await, SyncAuth::BearerToken(_))
    }

    async fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let auth = self.auth.read().await;
        match &*auth {
            SyncAuth::ApiKey(key) => req.header("X-API-Key", key.clone()),
            SyncAuth::BearerToken(token) => {
                req.header("Authorization", format!("Bearer {token}"))
            }
        }
    }

    async fn post(&self, path: &str, body: serde_json::Value) -> SyncResult<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.http.post(&url).json(&body);
        let req = self.apply_auth(req).await;

        let response = req.send().await.map_err(|e| {
            if e.is_timeout() {
                SyncError::Network(format!("auth Lambda timeout: {e}"))
            } else if e.is_connect() {
                SyncError::Network(format!("auth Lambda unreachable: {e}"))
            } else {
                SyncError::Network(e.to_string())
            }
        })?;

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(SyncError::Auth(
                "authentication failed — re-authenticate".to_string(),
            ));
        }

        // Let HTTP 403 pass through to be handled by the caller, so we can discern
        // if it's a global ban vs an organizational-level ban.
        if status == reqwest::StatusCode::FORBIDDEN {
            // we try to parse it as JSON if it has an error payload, otherwise bubble it
        }

        if status.is_server_error() {
            let body = response.text().await.unwrap_or_default();
            return Err(SyncError::Auth(format!(
                "auth Lambda error: HTTP {status}: {body}"
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SyncError::Auth(format!("invalid JSON from auth Lambda: {e}")))?;

        Ok(json)
    }

    /// List objects in the user's S3 prefix.
    pub async fn list_objects(&self, prefix: &str) -> SyncResult<Vec<S3ObjectInfo>> {
        let body = serde_json::json!({
            "action": "list_objects",
            "prefix": prefix,
        });

        let resp = self.post("/api/sync/list", body).await?;
        let parsed: ListObjectsResponse = serde_json::from_value(resp)?;

        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed.error.unwrap_or_else(|| "list failed".to_string()),
            ));
        }

        Ok(parsed.objects)
    }

    /// Request a presigned URL for uploading a snapshot.
    pub async fn presign_snapshot_upload(&self, snapshot_name: &str) -> SyncResult<PresignedUrl> {
        let body = serde_json::json!({
            "action": "presign_snapshot_upload",
            "snapshot_name": snapshot_name,
        });

        let resp = self.post("/api/sync/presign", body).await?;
        let parsed: PresignedResponse = serde_json::from_value(resp)?;

        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed.error.unwrap_or_else(|| "presign failed".to_string()),
            ));
        }

        parsed
            .urls
            .into_iter()
            .next()
            .ok_or_else(|| SyncError::Auth("no presigned URL returned".to_string()))
    }

    /// Request a presigned URL for downloading a snapshot.
    pub async fn presign_snapshot_download(&self, snapshot_name: &str) -> SyncResult<PresignedUrl> {
        let body = serde_json::json!({
            "action": "presign_snapshot_download",
            "snapshot_name": snapshot_name,
        });

        let resp = self.post("/api/sync/presign", body).await?;
        let parsed: PresignedResponse = serde_json::from_value(resp)?;

        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed.error.unwrap_or_else(|| "presign failed".to_string()),
            ));
        }

        parsed
            .urls
            .into_iter()
            .next()
            .ok_or_else(|| SyncError::Auth("no presigned URL returned".to_string()))
    }

    /// Request a presigned URL to upload to another user's inbox
    pub async fn presign_inbox_upload(
        &self,
        target_user_hash: &str,
        file_name: &str,
    ) -> SyncResult<PresignedUrl> {
        let body = serde_json::json!({
            "action": "presign_inbox_upload",
            "target_user_hash": target_user_hash,
            "snapshot_name": file_name,
        });

        let resp = self.post("/api/sync/presign", body).await?;
        let parsed: PresignedResponse = serde_json::from_value(resp)?;

        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed.error.unwrap_or_else(|| "presign failed".to_string()),
            ));
        }

        parsed
            .urls
            .into_iter()
            .next()
            .ok_or_else(|| SyncError::Auth("no presigned URL returned".to_string()))
    }

    /// Request a presigned URL to download an item from your own inbox
    pub async fn presign_inbox_download(&self, file_name: &str) -> SyncResult<PresignedUrl> {
        let body = serde_json::json!({
            "action": "presign_inbox_download",
            "snapshot_name": file_name,
        });

        let resp = self.post("/api/sync/presign", body).await?;
        let parsed: PresignedResponse = serde_json::from_value(resp)?;

        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed.error.unwrap_or_else(|| "presign failed".to_string()),
            ));
        }

        parsed
            .urls
            .into_iter()
            .next()
            .ok_or_else(|| SyncError::Auth("no presigned URL returned".to_string()))
    }

    /// Presign a DELETE URL for removing an inbox object (e.g., accepted/declined invite).
    pub async fn presign_inbox_delete(&self, file_name: &str) -> SyncResult<PresignedUrl> {
        let body = serde_json::json!({
            "action": "presign_inbox_delete",
            "snapshot_name": file_name,
        });
        let resp = self.post("/api/sync/presign", body).await?;
        let parsed: PresignedResponse = serde_json::from_value(resp)?;
        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed
                    .error
                    .unwrap_or_else(|| "presign inbox delete failed".to_string()),
            ));
        }
        parsed
            .urls
            .into_iter()
            .next()
            .ok_or_else(|| SyncError::Auth("no presigned URL returned".to_string()))
    }

    /// Request presigned URLs for deleting log entries.
    pub async fn presign_log_delete(&self, seq_numbers: &[u64]) -> SyncResult<Vec<PresignedUrl>> {
        let body = serde_json::json!({
            "action": "presign_log_delete",
            "seq_numbers": seq_numbers,
        });

        let resp = self.post("/api/sync/presign", body).await?;
        let parsed: PresignedResponse = serde_json::from_value(resp)?;

        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed.error.unwrap_or_else(|| "presign failed".to_string()),
            ));
        }

        Ok(parsed.urls)
    }

    /// Acquire the device lock.
    pub async fn acquire_lock(&self, device_id: &str, ttl_secs: u64) -> SyncResult<bool> {
        let body = serde_json::json!({
            "action": "acquire_lock",
            "device_id": device_id,
            "ttl_secs": ttl_secs,
        });

        let resp = self.post("/api/sync/lock", body).await?;
        let parsed: LockResponse = serde_json::from_value(resp)?;

        if !parsed.ok {
            if let Some(locked_by) = parsed.locked_by {
                return Err(SyncError::DeviceLocked {
                    device_id: locked_by,
                    expires_at: parsed.expires_at.unwrap_or_default(),
                });
            }
            return Err(SyncError::Auth(
                parsed.error.unwrap_or_else(|| "lock failed".to_string()),
            ));
        }

        Ok(true)
    }

    /// Release the device lock.
    pub async fn release_lock(&self, device_id: &str) -> SyncResult<()> {
        let body = serde_json::json!({
            "action": "release_lock",
            "device_id": device_id,
        });

        let resp = self.post("/api/sync/lock", body).await?;
        let parsed: LockResponse = serde_json::from_value(resp)?;

        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed.error.unwrap_or_else(|| "unlock failed".to_string()),
            ));
        }

        Ok(())
    }

    /// Renew the device lock (extend TTL).
    pub async fn renew_lock(&self, device_id: &str, ttl_secs: u64) -> SyncResult<()> {
        let body = serde_json::json!({
            "action": "renew_lock",
            "device_id": device_id,
            "ttl_secs": ttl_secs,
        });

        let resp = self.post("/api/sync/lock", body).await?;
        let parsed: LockResponse = serde_json::from_value(resp)?;

        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed.error.unwrap_or_else(|| "renew failed".to_string()),
            ));
        }

        Ok(())
    }

    // =========================================================================
    // Sync operations (unified personal + org)
    // =========================================================================

    /// Presign URLs for uploading log entries to a sync target.
    pub async fn presign_upload(
        &self,
        target: &super::org_sync::SyncTarget,
        seq_numbers: &[u64],
    ) -> SyncResult<Vec<PresignedUrl>> {
        let mut body = serde_json::json!({
            "action": "presign_log_upload",
            "seq_numbers": seq_numbers,
        });
        if !target.prefix.is_empty() {
            body["org_hash"] = serde_json::Value::String(target.prefix.clone());
        }
        let resp = self.post("/api/sync/presign", body).await?;
        let parsed: PresignedResponse = serde_json::from_value(resp)?;
        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed
                    .error
                    .unwrap_or_else(|| "presign upload failed".to_string()),
            ));
        }
        Ok(parsed.urls)
    }

    /// Presign URLs for downloading log entries from a sync target.
    pub async fn presign_download(
        &self,
        target: &super::org_sync::SyncTarget,
        seq_numbers: &[u64],
    ) -> SyncResult<Vec<PresignedUrl>> {
        let mut body = serde_json::json!({
            "action": "presign_log_download",
            "seq_numbers": seq_numbers,
        });
        if !target.prefix.is_empty() {
            body["org_hash"] = serde_json::Value::String(target.prefix.clone());
        }
        let resp = self.post("/api/sync/presign", body).await?;
        let parsed: PresignedResponse = serde_json::from_value(resp)?;
        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed
                    .error
                    .unwrap_or_else(|| "presign download failed".to_string()),
            ));
        }
        Ok(parsed.urls)
    }

    /// List log objects for a sync target.
    pub async fn list_log_objects(
        &self,
        target: &super::org_sync::SyncTarget,
    ) -> SyncResult<Vec<S3ObjectInfo>> {
        self.list_log_objects_after(target, None).await
    }

    /// List log objects for a sync target, optionally starting after a given key.
    pub async fn list_log_objects_after(
        &self,
        target: &super::org_sync::SyncTarget,
        start_after: Option<&str>,
    ) -> SyncResult<Vec<S3ObjectInfo>> {
        let mut body = serde_json::json!({
            "action": "list_objects",
            "prefix": "log/",
        });
        if !target.prefix.is_empty() {
            body["org_hash"] = serde_json::Value::String(target.prefix.clone());
        }
        if let Some(cursor) = start_after {
            body["start_after"] = serde_json::Value::String(cursor.to_string());
        }
        let resp = self.post("/api/sync/list", body).await?;
        let parsed: ListObjectsResponse = serde_json::from_value(resp)?;
        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed
                    .error
                    .unwrap_or_else(|| "list log objects failed".to_string()),
            ));
        }
        Ok(parsed.objects)
    }

    // =========================================================================
    // Org membership management
    // =========================================================================

    pub async fn create_org(&self, org_hash: &str) -> SyncResult<()> {
        let body = serde_json::json!({
            "action": "create_org",
            "org_hash": org_hash,
        });
        let resp = self.post("/api/sync/org", body).await?;
        let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let err = resp
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("create_org failed");
            return Err(SyncError::Auth(err.to_string()));
        }
        Ok(())
    }

    pub async fn add_member(
        &self,
        org_hash: &str,
        target_user_hash: &str,
        role: &str,
    ) -> SyncResult<()> {
        let body = serde_json::json!({
            "action": "add_member",
            "org_hash": org_hash,
            "target_user_hash": target_user_hash,
            "role": role,
        });
        let resp = self.post("/api/sync/org", body).await?;
        let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let err = resp
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("add_member failed");
            return Err(SyncError::Auth(err.to_string()));
        }
        Ok(())
    }

    pub async fn remove_member(&self, org_hash: &str, target_user_hash: &str) -> SyncResult<()> {
        let body = serde_json::json!({
            "action": "remove_member",
            "org_hash": org_hash,
            "target_user_hash": target_user_hash,
        });
        let resp = self.post("/api/sync/org", body).await?;
        let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let err = resp
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("remove_member failed");
            return Err(SyncError::Auth(err.to_string()));
        }
        Ok(())
    }

    pub async fn update_role(
        &self,
        org_hash: &str,
        target_user_hash: &str,
        role: &str,
    ) -> SyncResult<()> {
        let body = serde_json::json!({
            "action": "update_role",
            "org_hash": org_hash,
            "target_user_hash": target_user_hash,
            "role": role,
        });
        let resp = self.post("/api/sync/org", body).await?;
        let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let err = resp
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("update_role failed");
            return Err(SyncError::Auth(err.to_string()));
        }
        Ok(())
    }

    /// Notify the cloud that this user accepted an org invite (status -> active).
    pub async fn accept_invite(&self, org_hash: &str) -> SyncResult<()> {
        let body = serde_json::json!({
            "action": "accept_invite",
            "org_hash": org_hash,
        });
        let resp = self.post("/api/sync/org", body).await?;
        let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let err = resp
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("accept_invite failed");
            return Err(SyncError::Auth(err.to_string()));
        }
        Ok(())
    }

    /// Notify the cloud that this user declined an org invite (status -> declined).
    pub async fn decline_invite(&self, org_hash: &str) -> SyncResult<()> {
        let body = serde_json::json!({
            "action": "decline_invite",
            "org_hash": org_hash,
        });
        let resp = self.post("/api/sync/org", body).await?;
        let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let err = resp
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("decline_invite failed");
            return Err(SyncError::Auth(err.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_update_auth_replaces_credential() {
        let http = Arc::new(Client::new());
        let client =
            AuthClient::new(http, "http://localhost".to_string(), SyncAuth::ApiKey("old".into()));

        // Starts as API key
        assert!(!client.is_bearer_token().await);

        // Update to bearer token
        client
            .update_auth(SyncAuth::BearerToken("new-token".into()))
            .await;
        assert!(client.is_bearer_token().await);

        // Update back to API key
        client
            .update_auth(SyncAuth::ApiKey("new-key".into()))
            .await;
        assert!(!client.is_bearer_token().await);
    }

    #[tokio::test]
    async fn test_auth_refresh_callback_type() {
        // Verify the callback type compiles and can return SyncAuth
        let cb: AuthRefreshCallback = Arc::new(|| {
            Box::pin(async { Ok(SyncAuth::BearerToken("refreshed-token".into())) })
        });

        let result = cb().await;
        assert!(result.is_ok());
        match result.unwrap() {
            SyncAuth::BearerToken(t) => assert_eq!(t, "refreshed-token"),
            SyncAuth::ApiKey(_) => panic!("expected BearerToken"),
        }
    }

    #[tokio::test]
    async fn test_auth_refresh_callback_error() {
        let cb: AuthRefreshCallback =
            Arc::new(|| Box::pin(async { Err("network down".to_string()) }));

        let result = cb().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "network down");
    }
}
