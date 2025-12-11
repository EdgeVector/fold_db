use super::config::S3Config;
use super::error::{StorageError, StorageResult};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// S3-synced storage that downloads/uploads Sled directories to S3
pub struct S3SyncedStorage {
    local_path: PathBuf,
    s3_bucket: String,
    s3_prefix: String,
    s3_client: Client,
}

impl S3SyncedStorage {
    /// Creates a new S3-synced storage, downloading from S3 if exists
    pub async fn new(config: S3Config) -> StorageResult<Self> {
        let s3_client = create_s3_client(&config.region).await?;

        // Ensure local directory exists
        fs::create_dir_all(&config.local_path).await?;

        // Check if data exists in S3 and download if it does
        let exists = check_s3_data_exists(&s3_client, &config.bucket, &config.prefix).await?;

        if exists {
            download_directory_from_s3(
                &s3_client,
                &config.bucket,
                &config.prefix,
                &config.local_path,
            )
            .await?;
        }

        Ok(Self {
            local_path: config.local_path,
            s3_bucket: config.bucket,
            s3_prefix: config.prefix,
            s3_client,
        })
    }

    /// Returns the local path where Sled data is stored
    pub fn local_path(&self) -> &Path {
        &self.local_path
    }

    /// Syncs local Sled data to S3
    pub async fn sync_to_s3(&self) -> StorageResult<()> {
        upload_directory_to_s3(
            &self.s3_client,
            &self.local_path,
            &self.s3_bucket,
            &self.s3_prefix,
        )
        .await
    }
}

/// Creates an S3 client for the specified region
async fn create_s3_client(region: &str) -> StorageResult<Client> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(region.to_string()))
        .load()
        .await;

    Ok(Client::new(&config))
}

/// Checks if data exists in S3 at the specified location
async fn check_s3_data_exists(
    client: &Client,
    bucket: &str,
    prefix: &str,
) -> StorageResult<bool> {
    let list_result = client
        .list_objects_v2()
        .bucket(bucket)
        .prefix(format!("{}/", prefix))
        .max_keys(1)
        .send()
        .await
        .map_err(|e| StorageError::S3Error(format!("Failed to check S3 data: {}", e)))?;

    Ok(!list_result.contents().is_empty())
}

/// Downloads a directory from S3 to local filesystem
async fn download_directory_from_s3(
    client: &Client,
    bucket: &str,
    prefix: &str,
    local_path: &Path,
) -> StorageResult<()> {
    let mut continuation_token: Option<String> = None;

    loop {
        let mut list_request = client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(format!("{}/", prefix));

        if let Some(token) = continuation_token {
            list_request = list_request.continuation_token(token);
        }

        let list_result = list_request
            .send()
            .await
            .map_err(|e| StorageError::DownloadFailed(format!("Failed to list S3 objects: {}", e)))?;

        let contents = list_result.contents();
        
        for object in contents {
            let key = object.key()
                .ok_or_else(|| StorageError::DownloadFailed("Object key missing".to_string()))?;

            // Skip directory markers
            if key.ends_with('/') {
                continue;
            }

            // Get relative path by removing prefix
            let relative_path = key
                .strip_prefix(&format!("{}/", prefix))
                .unwrap_or(key);

            let local_file_path = local_path.join(relative_path);

            // Create parent directories
            if let Some(parent) = local_file_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Download the file
            let get_result = client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| {
                    StorageError::DownloadFailed(format!("Failed to download {}: {}", key, e))
                })?;

            let data = get_result
                .body
                .collect()
                .await
                .map_err(|e| {
                    StorageError::DownloadFailed(format!("Failed to read body for {}: {}", key, e))
                })?
                .into_bytes();

            let mut file = fs::File::create(&local_file_path).await?;
            file.write_all(&data).await?;
        }

        if list_result.is_truncated().unwrap_or(false) {
            continuation_token = list_result.next_continuation_token().map(|s| s.to_string());
        } else {
            break;
        }
    }

    Ok(())
}

/// Uploads a directory to S3
async fn upload_directory_to_s3(
    client: &Client,
    local_path: &Path,
    bucket: &str,
    prefix: &str,
) -> StorageResult<()> {
    let mut entries = Vec::new();
    collect_files_recursive(local_path, local_path, &mut entries)?;

    for (relative_path, absolute_path) in entries {
        let s3_key = format!(
            "{}/{}",
            prefix,
            relative_path
                .to_str()
                .ok_or_else(|| StorageError::InvalidPath("Invalid UTF-8 in path".to_string()))?
        );

        let contents = fs::read(&absolute_path).await?;

        client
            .put_object()
            .bucket(bucket)
            .key(s3_key)
            .body(contents.into())
            .send()
            .await
            .map_err(|e| {
                StorageError::UploadFailed(format!(
                    "Failed to upload {}: {}",
                    relative_path.display(),
                    e
                ))
            })?;
    }

    Ok(())
}

/// Recursively collects all files in a directory
fn collect_files_recursive(
    base_path: &Path,
    current_path: &Path,
    entries: &mut Vec<(PathBuf, PathBuf)>,
) -> StorageResult<()> {
    use std::fs;
    
    let dir_entries = fs::read_dir(current_path)?;

    for entry in dir_entries {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            collect_files_recursive(base_path, &path, entries)?;
        } else if metadata.is_file() {
            let relative_path = path
                .strip_prefix(base_path)
                .map_err(|_| {
                    StorageError::InvalidPath(format!(
                        "Failed to strip prefix from {}",
                        path.display()
                    ))
                })?
                .to_path_buf();

            entries.push((relative_path, path));
        }
    }

    Ok(())
}

