use super::error::{SyncError, SyncResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Client for interacting with S3 via presigned URLs.
///
/// This client never has AWS credentials — it only uses presigned URLs
/// obtained from the auth Lambda. Each URL is scoped to a single S3 object
/// and a single operation (GET or PUT), expiring after a short window.
pub struct S3Client {
    http: Arc<Client>,
}

/// A presigned URL for a specific S3 operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresignedUrl {
    pub url: String,
    pub method: String,
    pub expires_in_secs: u64,
}

impl S3Client {
    pub fn new(http: Arc<Client>) -> Self {
        Self { http }
    }

    /// Upload bytes to S3 using a presigned PUT URL.
    pub async fn upload(&self, presigned: &PresignedUrl, data: Vec<u8>) -> SyncResult<()> {
        let response = self
            .http
            .put(&presigned.url)
            .header("Content-Type", "application/octet-stream")
            .body(data)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SyncError::S3(format!(
                "upload failed: HTTP {status}: {body}"
            )));
        }

        Ok(())
    }

    /// Download bytes from S3 using a presigned GET URL.
    ///
    /// Returns `None` if the object doesn't exist (404).
    pub async fn download(&self, presigned: &PresignedUrl) -> SyncResult<Option<Vec<u8>>> {
        let response = self.http.get(&presigned.url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SyncError::S3(format!(
                "download failed: HTTP {status}: {body}"
            )));
        }

        let bytes = response.bytes().await?;
        Ok(Some(bytes.to_vec()))
    }

    /// Delete an S3 object using a presigned DELETE URL.
    pub async fn delete(&self, presigned: &PresignedUrl) -> SyncResult<()> {
        let response = self.http.delete(&presigned.url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SyncError::S3(format!(
                "delete failed: HTTP {status}: {body}"
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presigned_url_deserialize() {
        let json = r#"{"url":"https://bucket.s3.amazonaws.com/key?X-Amz-Signature=abc","method":"PUT","expires_in_secs":900}"#;
        let url: PresignedUrl = serde_json::from_str(json).unwrap();
        assert_eq!(url.method, "PUT");
        assert_eq!(url.expires_in_secs, 900);
        assert!(url.url.contains("X-Amz-Signature"));
    }
}
