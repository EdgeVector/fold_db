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
    /// Sequence numbers assigned by the server (only populated for
    /// server-allocated org uploads). Ordered alongside `urls`.
    #[serde(default)]
    pub seq_numbers: Vec<u64>,
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
            SyncAuth::BearerToken(token) => req.header("Authorization", format!("Bearer {token}")),
        }
    }

    async fn post(&self, path: &str, body: serde_json::Value) -> SyncResult<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.http.post(&url).json(&body);
        let req = self.apply_auth(req).await;
        let req = observability::propagation::inject_w3c(req);

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

    /// Presign a single URL: post the body, parse a PresignedResponse, extract one URL.
    async fn presign_single_url(&self, body: serde_json::Value) -> SyncResult<PresignedUrl> {
        self.presign_urls(body)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| SyncError::Auth("no presigned URL returned".to_string()))
    }

    /// Request a presigned URL for uploading a snapshot.
    pub async fn presign_snapshot_upload(&self, snapshot_name: &str) -> SyncResult<PresignedUrl> {
        self.presign_single_url(serde_json::json!({
            "action": "presign_snapshot_upload",
            "snapshot_name": snapshot_name,
        }))
        .await
    }

    /// Request a presigned URL for downloading a snapshot.
    pub async fn presign_snapshot_download(&self, snapshot_name: &str) -> SyncResult<PresignedUrl> {
        self.presign_single_url(serde_json::json!({
            "action": "presign_snapshot_download",
            "snapshot_name": snapshot_name,
        }))
        .await
    }

    /// Request a presigned URL to upload to another user's inbox
    pub async fn presign_inbox_upload(
        &self,
        target_user_hash: &str,
        file_name: &str,
    ) -> SyncResult<PresignedUrl> {
        self.presign_single_url(serde_json::json!({
            "action": "presign_inbox_upload",
            "target_user_hash": target_user_hash,
            "snapshot_name": file_name,
        }))
        .await
    }

    /// Request a presigned URL to download an item from your own inbox
    pub async fn presign_inbox_download(&self, file_name: &str) -> SyncResult<PresignedUrl> {
        self.presign_single_url(serde_json::json!({
            "action": "presign_inbox_download",
            "snapshot_name": file_name,
        }))
        .await
    }

    /// Presign a DELETE URL for removing an inbox object (e.g., accepted/declined invite).
    pub async fn presign_inbox_delete(&self, file_name: &str) -> SyncResult<PresignedUrl> {
        self.presign_single_url(serde_json::json!({
            "action": "presign_inbox_delete",
            "snapshot_name": file_name,
        }))
        .await
    }

    /// Request presigned URLs for deleting log entries.
    pub async fn presign_log_delete(&self, seq_numbers: &[u64]) -> SyncResult<Vec<PresignedUrl>> {
        self.presign_urls(serde_json::json!({
            "action": "presign_log_delete",
            "seq_numbers": seq_numbers,
        }))
        .await
    }

    /// Presign a DELETE URL for a snapshot object (e.g., `latest.enc` or
    /// `{seq}.enc`). Used by the cloud-aware reset path that purges the
    /// personal sync log along with its snapshots.
    pub async fn presign_snapshot_delete(&self, snapshot_name: &str) -> SyncResult<PresignedUrl> {
        self.presign_single_url(serde_json::json!({
            "action": "presign_snapshot_delete",
            "snapshot_name": snapshot_name,
        }))
        .await
    }

    /// Acquire the device lock.
    pub async fn acquire_lock(&self, device_id: &str, ttl_secs: u64) -> SyncResult<bool> {
        let body = serde_json::json!({
            "action": "acquire_lock",
            "device_id": device_id,
            "ttl_secs": ttl_secs,
        });

        let resp = self.post("/api/storage-admin/lock", body).await?;
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

        let resp = self.post("/api/storage-admin/lock", body).await?;
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

        let resp = self.post("/api/storage-admin/lock", body).await?;
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

    /// Post to the presign endpoint and parse the response, returning all URLs.
    async fn presign_urls(&self, body: serde_json::Value) -> SyncResult<Vec<PresignedUrl>> {
        self.presign_response(body).await.map(|r| r.urls)
    }

    /// Post to the presign endpoint and return the full parsed response —
    /// including any server-assigned `seq_numbers` (for server-allocated org
    /// uploads).
    async fn presign_response(&self, body: serde_json::Value) -> SyncResult<PresignedResponse> {
        let action = body
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("presign")
            .to_string();
        let resp = self.post("/api/sync/presign", body).await?;
        let parsed: PresignedResponse = serde_json::from_value(resp)?;
        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed.error.unwrap_or_else(|| format!("{action} failed")),
            ));
        }
        Ok(parsed)
    }

    /// Presign multiple URLs for a log action (upload or download) on a sync target.
    async fn presign_log_urls(
        &self,
        action: &str,
        target: &super::org_sync::SyncTarget,
        seq_numbers: &[u64],
    ) -> SyncResult<Vec<PresignedUrl>> {
        let mut body = serde_json::json!({
            "action": action,
            "seq_numbers": seq_numbers,
        });
        if !target.prefix.is_empty() {
            body["org_hash"] = serde_json::Value::String(target.prefix.clone());
        }
        self.presign_urls(body).await
    }

    /// Presign URLs for uploading log entries to a sync target.
    pub async fn presign_upload(
        &self,
        target: &super::org_sync::SyncTarget,
        seq_numbers: &[u64],
    ) -> SyncResult<Vec<PresignedUrl>> {
        self.presign_log_urls("presign_log_upload", target, seq_numbers)
            .await
    }

    /// Ask the server to atomically allocate `count` sequence numbers for an
    /// org upload and presign matching URLs in the same request. Returns the
    /// server-assigned seqs paired with their presigned URLs, sorted in
    /// ascending seq order.
    ///
    /// Only valid for org targets (non-empty prefix) — personal uploads
    /// must keep client-assigned nanosecond seqs because they use the
    /// single-writer device lock for ordering.
    pub async fn presign_upload_alloc(
        &self,
        target: &super::org_sync::SyncTarget,
        count: u32,
    ) -> SyncResult<Vec<(u64, PresignedUrl)>> {
        if target.prefix.is_empty() {
            return Err(SyncError::Auth(
                "presign_upload_alloc requires an org target (empty prefix)".to_string(),
            ));
        }
        if count == 0 {
            return Ok(Vec::new());
        }
        let body = serde_json::json!({
            "action": "presign_log_upload",
            "org_hash": target.prefix,
            "count": count,
        });
        let resp = self.presign_response(body).await?;
        if resp.seq_numbers.len() != resp.urls.len() {
            return Err(SyncError::Auth(format!(
                "presign_upload_alloc: seq_numbers ({}) and urls ({}) length mismatch",
                resp.seq_numbers.len(),
                resp.urls.len(),
            )));
        }
        if resp.seq_numbers.len() != count as usize {
            return Err(SyncError::Auth(format!(
                "presign_upload_alloc: expected {} seqs, got {}",
                count,
                resp.seq_numbers.len(),
            )));
        }
        Ok(resp.seq_numbers.into_iter().zip(resp.urls).collect())
    }

    /// Presign URLs for downloading log entries from a sync target.
    pub async fn presign_download(
        &self,
        target: &super::org_sync::SyncTarget,
        seq_numbers: &[u64],
    ) -> SyncResult<Vec<PresignedUrl>> {
        self.presign_log_urls("presign_log_download", target, seq_numbers)
            .await
    }

    /// List log objects for a sync target.
    pub async fn list_log_objects(
        &self,
        target: &super::org_sync::SyncTarget,
    ) -> SyncResult<Vec<S3ObjectInfo>> {
        self.list_log_objects_after(target, None).await
    }

    /// List snapshot objects (`snapshots/*.enc`) under a sync target's prefix.
    ///
    /// Used by the cloud-aware reset path to enumerate every snapshot
    /// (including `latest.enc` and any compacted `{seq}.enc`) so each can
    /// be deleted via `presign_snapshot_delete`.
    pub async fn list_snapshot_objects(
        &self,
        target: &super::org_sync::SyncTarget,
    ) -> SyncResult<Vec<S3ObjectInfo>> {
        let mut body = serde_json::json!({
            "action": "list_objects",
            "prefix": "snapshots/",
        });
        if !target.prefix.is_empty() {
            body["org_hash"] = serde_json::Value::String(target.prefix.clone());
        }
        let resp = self.post("/api/sync/list", body).await?;
        let parsed: ListObjectsResponse = serde_json::from_value(resp)?;
        if !parsed.ok {
            return Err(SyncError::Auth(
                parsed
                    .error
                    .unwrap_or_else(|| "list snapshot objects failed".to_string()),
            ));
        }
        Ok(parsed.objects)
    }

    /// List log objects for a sync target, optionally starting after a given key.
    ///
    /// **WARNING — lex-ordered `start_after`:** S3 `start_after` filters keys
    /// by **lexicographic** order, not numeric. Do not pass a key built from
    /// an unpadded decimal seq (`log/52.enc`) expecting it to bound keys with
    /// higher numeric seqs — `log/100.enc` lex-sorts *before* `log/52.enc`
    /// and would be silently hidden. This caused alpha BLOCKER 30a7b. Only
    /// use `start_after` against keys whose natural ordering is already
    /// lexicographic (e.g., ISO-8601 timestamps, fixed-width hex). Numeric
    /// log seqs must be filtered client-side after a full prefix list.
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

    /// Post an org action and check the `ok` field. Returns the full response
    /// for callers that need to extract additional fields (e.g., `list_members`).
    async fn org_action(&self, body: serde_json::Value) -> SyncResult<serde_json::Value> {
        let action = body
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("org_action")
            .to_string();
        // Org management is now served by the standalone `org_service`
        // Lambda (split out of storage_service). The cloud endpoint is
        // `POST /api/org/{action}` — we post to `/api/org/action` and
        // the Lambda dispatches by the `action` field in the body.
        let resp = self.post("/api/org/action", body).await?;
        let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let default_msg = format!("{action} failed");
            let err = resp
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or(&default_msg);
            return Err(SyncError::Auth(err.to_string()));
        }
        Ok(resp)
    }

    pub async fn create_org(&self, org_hash: &str) -> SyncResult<()> {
        self.org_action(serde_json::json!({
            "action": "create_org",
            "org_hash": org_hash,
        }))
        .await?;
        Ok(())
    }

    pub async fn add_member(
        &self,
        org_hash: &str,
        target_user_hash: &str,
        role: &str,
    ) -> SyncResult<()> {
        self.org_action(serde_json::json!({
            "action": "add_member",
            "org_hash": org_hash,
            "target_user_hash": target_user_hash,
            "role": role,
        }))
        .await?;
        Ok(())
    }

    pub async fn remove_member(&self, org_hash: &str, target_user_hash: &str) -> SyncResult<()> {
        self.org_action(serde_json::json!({
            "action": "remove_member",
            "org_hash": org_hash,
            "target_user_hash": target_user_hash,
        }))
        .await?;
        Ok(())
    }

    pub async fn update_role(
        &self,
        org_hash: &str,
        target_user_hash: &str,
        role: &str,
    ) -> SyncResult<()> {
        self.org_action(serde_json::json!({
            "action": "update_role",
            "org_hash": org_hash,
            "target_user_hash": target_user_hash,
            "role": role,
        }))
        .await?;
        Ok(())
    }

    /// Notify the cloud that this user accepted an org invite (status -> active).
    pub async fn accept_invite(&self, org_hash: &str) -> SyncResult<()> {
        self.org_action(serde_json::json!({
            "action": "accept_invite",
            "org_hash": org_hash,
        }))
        .await?;
        Ok(())
    }

    /// Fetch the current member list for an org from the cloud.
    /// Returns a JSON array of `{ user_hash, role, status }` objects.
    pub async fn list_members(&self, org_hash: &str) -> SyncResult<Vec<serde_json::Value>> {
        let resp = self
            .org_action(serde_json::json!({
                "action": "list_members",
                "org_hash": org_hash,
            }))
            .await?;
        let members = resp
            .get("members")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(members)
    }

    /// Notify the cloud that this user declined an org invite (status -> declined).
    pub async fn decline_invite(&self, org_hash: &str) -> SyncResult<()> {
        self.org_action(serde_json::json!({
            "action": "decline_invite",
            "org_hash": org_hash,
        }))
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_update_auth_replaces_credential() {
        let http = Arc::new(Client::new());
        let client = AuthClient::new(
            http,
            "http://localhost".to_string(),
            SyncAuth::ApiKey("old".into()),
        );

        // Starts as API key
        assert!(!client.is_bearer_token().await);

        // Update to bearer token
        client
            .update_auth(SyncAuth::BearerToken("new-token".into()))
            .await;
        assert!(client.is_bearer_token().await);

        // Update back to API key
        client.update_auth(SyncAuth::ApiKey("new-key".into())).await;
        assert!(!client.is_bearer_token().await);
    }

    #[tokio::test]
    async fn test_auth_refresh_callback_type() {
        let cb: AuthRefreshCallback =
            Arc::new(|| Box::pin(async { Ok(SyncAuth::BearerToken("refreshed-token".into())) }));

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

    #[tokio::test]
    async fn presign_upload_alloc_rejects_personal_target() {
        use crate::crypto::CryptoProvider;
        use crate::sync::org_sync::SyncTarget;

        let http = Arc::new(Client::new());
        let client = AuthClient::new(
            http,
            "http://localhost:9999".to_string(),
            SyncAuth::ApiKey("test".into()),
        );

        // Personal target (empty prefix) must be rejected — server-allocated
        // seqs are only valid for orgs.
        let target = SyncTarget {
            label: "personal".to_string(),
            prefix: String::new(),
            crypto: Arc::new(crate::crypto::NoOpCryptoProvider) as Arc<dyn CryptoProvider>,
        };
        let result = client.presign_upload_alloc(&target, 3).await;
        assert!(result.is_err(), "personal target should be rejected");
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("org target"),
            "error message should mention org target requirement: {msg}"
        );
    }

    #[tokio::test]
    async fn presign_upload_alloc_count_zero_returns_empty() {
        use crate::crypto::CryptoProvider;
        use crate::sync::org_sync::SyncTarget;

        let http = Arc::new(Client::new());
        let client = AuthClient::new(
            http,
            "http://localhost:9999".to_string(),
            SyncAuth::ApiKey("test".into()),
        );

        let target = SyncTarget {
            label: "test-org".to_string(),
            prefix: "0".repeat(64),
            crypto: Arc::new(crate::crypto::NoOpCryptoProvider) as Arc<dyn CryptoProvider>,
        };
        // count = 0 returns immediately with an empty vec and does not hit
        // the network (so the localhost:9999 non-server does not matter).
        let result = client.presign_upload_alloc(&target, 0).await.unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn presigned_response_parses_server_assigned_seq_numbers() {
        // When the server allocates seqs for an org upload, it returns them
        // alongside the URLs. Old servers (personal upload path) return only
        // `urls` and the default empty `seq_numbers` must deserialize cleanly.
        let json_with_seqs = serde_json::json!({
            "ok": true,
            "urls": [
                {"url": "https://s3.example/put-42", "method": "PUT", "expires_in_secs": 900},
                {"url": "https://s3.example/put-43", "method": "PUT", "expires_in_secs": 900},
            ],
            "seq_numbers": [42, 43],
        });
        let parsed: PresignedResponse = serde_json::from_value(json_with_seqs).unwrap();
        assert!(parsed.ok);
        assert_eq!(parsed.urls.len(), 2);
        assert_eq!(parsed.seq_numbers, vec![42, 43]);

        let json_without_seqs = serde_json::json!({
            "ok": true,
            "urls": [
                {"url": "https://s3.example/put-1", "method": "PUT", "expires_in_secs": 900},
            ],
        });
        let parsed: PresignedResponse = serde_json::from_value(json_without_seqs).unwrap();
        assert!(parsed.ok);
        assert_eq!(parsed.urls.len(), 1);
        assert!(parsed.seq_numbers.is_empty());
    }
}
