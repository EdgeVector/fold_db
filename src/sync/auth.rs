use super::error::{SyncError, SyncResult};
use super::s3::PresignedUrl;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

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
    auth: SyncAuth,
}

impl AuthClient {
    pub fn new(http: Arc<Client>, base_url: String, auth: SyncAuth) -> Self {
        Self {
            http,
            base_url,
            auth,
        }
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth {
            SyncAuth::ApiKey(key) => req.header("X-API-Key", key),
            SyncAuth::BearerToken(token) => req.header("Authorization", format!("Bearer {token}")),
        }
    }

    async fn post(&self, path: &str, body: serde_json::Value) -> SyncResult<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.http.post(&url).json(&body);
        let req = self.apply_auth(req);

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

    /// Request presigned URLs for uploading log entries.
    pub async fn presign_log_upload(&self, seq_numbers: &[u64]) -> SyncResult<Vec<PresignedUrl>> {
        let body = serde_json::json!({
            "action": "presign_log_upload",
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

    /// Request presigned URLs for downloading log entries.
    pub async fn presign_log_download(&self, seq_numbers: &[u64]) -> SyncResult<Vec<PresignedUrl>> {
        let body = serde_json::json!({
            "action": "presign_log_download",
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
    // Org sync operations
    // =========================================================================

    /// Request presigned URLs for uploading org log entries.
    ///
    /// The Lambda scopes URLs to `/{org_hash}/log/{seq}.enc`.
    pub async fn presign_org_log_upload(
        &self,
        org_hash: &str,
        member_id: &str,
        seq_numbers: &[u64],
    ) -> SyncResult<Vec<PresignedUrl>> {
        let body = serde_json::json!({
            "action": "presign_org_log_upload",
            "org_hash": org_hash,
            "member_id": member_id,
            "seq_numbers": seq_numbers,
        });

        let resp = self.post("/api/sync/presign", body).await?;
        let parsed: PresignedResponse = serde_json::from_value(resp)?;

        if !parsed.ok {
            let err_msg = parsed
                .error
                .unwrap_or_else(|| "org presign upload failed".to_string());
            let err_lower = err_msg.to_lowercase();
            if err_lower.contains("forbidden")
                || err_lower.contains("unauth")
                || err_lower.contains("not a member")
            {
                return Err(SyncError::OrgMembershipRevoked(org_hash.to_string()));
            }
            return Err(SyncError::Auth(err_msg));
        }

        Ok(parsed.urls)
    }

    /// Request presigned URLs for downloading org log entries from a specific member.
    pub async fn presign_org_log_download(
        &self,
        org_hash: &str,
        member_id: &str,
        seq_numbers: &[u64],
    ) -> SyncResult<Vec<PresignedUrl>> {
        let body = serde_json::json!({
            "action": "presign_org_log_download",
            "org_hash": org_hash,
            "member_id": member_id,
            "seq_numbers": seq_numbers,
        });

        let resp = self.post("/api/sync/presign", body).await?;
        let parsed: PresignedResponse = serde_json::from_value(resp)?;

        if !parsed.ok {
            let err_msg = parsed
                .error
                .unwrap_or_else(|| "org presign download failed".to_string());
            let err_lower = err_msg.to_lowercase();
            if err_lower.contains("forbidden")
                || err_lower.contains("unauth")
                || err_lower.contains("not a member")
            {
                return Err(SyncError::OrgMembershipRevoked(org_hash.to_string()));
            }
            return Err(SyncError::Auth(err_msg));
        }

        Ok(parsed.urls)
    }

    /// List objects in an org's S3 prefix.
    ///
    /// The prefix is relative to `/{org_hash}/`, so `"log/"` lists all log entries
    /// across all members.
    pub async fn list_org_objects(
        &self,
        org_hash: &str,
        prefix: &str,
    ) -> SyncResult<Vec<S3ObjectInfo>> {
        let body = serde_json::json!({
            "action": "list_objects",
            "org_hash": org_hash,
            "prefix": prefix,
        });

        let resp = self.post("/api/sync/list", body).await?;
        let parsed: ListObjectsResponse = serde_json::from_value(resp)?;

        if !parsed.ok {
            let err_msg = parsed
                .error
                .unwrap_or_else(|| "org list failed".to_string());
            let err_lower = err_msg.to_lowercase();
            if err_lower.contains("forbidden")
                || err_lower.contains("unauth")
                || err_lower.contains("not a member")
            {
                return Err(SyncError::OrgMembershipRevoked(org_hash.to_string()));
            }
            return Err(SyncError::Auth(err_msg));
        }

        Ok(parsed.objects)
    }

    // =========================================================================
    // Unified sync operations (transition wrappers)
    //
    // These dispatch to the old per-type endpoints during the backend
    // transition. After the backend is updated to use flat paths and
    // server-assigned seqs, these become the only presign methods.
    // =========================================================================

    /// Unified presign for log upload.
    pub async fn presign_upload(
        &self,
        target: &super::org_sync::SyncTarget,
        seq_numbers: &[u64],
    ) -> SyncResult<Vec<PresignedUrl>> {
        if target.is_org {
            self.presign_org_log_upload(&target.prefix, "_", seq_numbers)
                .await
        } else {
            self.presign_log_upload(seq_numbers).await
        }
    }

    /// Unified presign for log download.
    pub async fn presign_download(
        &self,
        target: &super::org_sync::SyncTarget,
        seq_numbers: &[u64],
    ) -> SyncResult<Vec<PresignedUrl>> {
        if target.is_org {
            self.presign_org_log_download(&target.prefix, "_", seq_numbers)
                .await
        } else {
            self.presign_log_download(seq_numbers).await
        }
    }

    /// Unified list log objects for a sync target.
    pub async fn list_log_objects(
        &self,
        target: &super::org_sync::SyncTarget,
    ) -> SyncResult<Vec<S3ObjectInfo>> {
        if target.is_org {
            self.list_org_objects(&target.prefix, "log/").await
        } else {
            self.list_objects("log/").await
        }
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
}
