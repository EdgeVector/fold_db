use super::error::{SyncError, SyncResult};
use super::s3::PresignedUrl;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

/// Authentication method for the sync auth Lambda.
#[derive(Clone, Debug)]
pub enum SyncAuth {
    ApiKey(String),
    BearerToken(String),
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
            SyncAuth::BearerToken(token) => {
                req.header("Authorization", format!("Bearer {token}"))
            }
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
        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            return Err(SyncError::Auth("authentication failed — re-authenticate".to_string()));
        }

        if status.is_server_error() {
            let body = response.text().await.unwrap_or_default();
            return Err(SyncError::Auth(format!("auth Lambda error: HTTP {status}: {body}")));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            SyncError::Auth(format!("invalid JSON from auth Lambda: {e}"))
        })?;

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

        parsed.urls.into_iter().next().ok_or_else(|| {
            SyncError::Auth("no presigned URL returned".to_string())
        })
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

        parsed.urls.into_iter().next().ok_or_else(|| {
            SyncError::Auth("no presigned URL returned".to_string())
        })
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
}
