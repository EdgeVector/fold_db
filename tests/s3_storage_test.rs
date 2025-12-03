use datafold::{FoldDB, S3Config, StorageConfig};
use std::path::PathBuf;
use std::sync::Mutex;

// Mutex to serialize tests that modify environment variables
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_storage_config_from_env_defaults_to_local() {
    let _lock = ENV_MUTEX.lock().unwrap();
    
    // Clear any S3 env vars
    std::env::remove_var("DATAFOLD_STORAGE_MODE");
    std::env::remove_var("DATAFOLD_S3_BUCKET");
    std::env::remove_var("DATAFOLD_S3_REGION");
    
    let config = StorageConfig::from_env().unwrap();
    
    match config {
        StorageConfig::Local { path } => {
            assert_eq!(path, PathBuf::from("data"));
        }
        StorageConfig::S3 { .. } => {
            panic!("Expected Local storage config");
        }
    }
}

#[test]
fn test_s3_config_from_env_missing_bucket_fails() {
    let _lock = ENV_MUTEX.lock().unwrap();
    
    std::env::remove_var("DATAFOLD_S3_BUCKET");
    std::env::set_var("DATAFOLD_S3_REGION", "us-west-2");
    
    let result = S3Config::from_env();
    assert!(result.is_err());
    
    std::env::remove_var("DATAFOLD_S3_REGION");
}

#[test]
fn test_s3_config_from_env_with_all_vars() {
    let _lock = ENV_MUTEX.lock().unwrap();
    
    // Use a test-specific prefix to avoid conflicts
    std::env::set_var("DATAFOLD_S3_BUCKET", "test-bucket-from-env");
    std::env::set_var("DATAFOLD_S3_REGION", "us-west-2");
    std::env::set_var("DATAFOLD_S3_PREFIX", "test-prefix");
    std::env::set_var("DATAFOLD_S3_LOCAL_PATH", "/tmp/test-path");
    
    let config = S3Config::from_env().unwrap();
    
    // Cleanup after reading config
    std::env::remove_var("DATAFOLD_S3_BUCKET");
    std::env::remove_var("DATAFOLD_S3_REGION");
    std::env::remove_var("DATAFOLD_S3_PREFIX");
    std::env::remove_var("DATAFOLD_S3_LOCAL_PATH");
    
    // Check the result
    assert_eq!(config.bucket, "test-bucket-from-env");
    assert_eq!(config.region, "us-west-2");
    assert_eq!(config.prefix, "test-prefix");
    assert_eq!(config.local_path, PathBuf::from("/tmp/test-path"));
}

#[test]
fn test_s3_config_defaults() {
    let config = S3Config::new(
        "my-bucket".to_string(),
        "us-east-1".to_string(),
        "my-prefix".to_string(),
    );
    
    assert_eq!(config.bucket, "my-bucket");
    assert_eq!(config.region, "us-east-1");
    assert_eq!(config.prefix, "my-prefix");
    assert_eq!(config.local_path, PathBuf::from("/tmp/folddb-data"));
}

#[test]
fn test_s3_config_with_custom_local_path() {
    let config = S3Config::new(
        "my-bucket".to_string(),
        "us-east-1".to_string(),
        "my-prefix".to_string(),
    ).with_local_path(PathBuf::from("/custom/path"));
    
    assert_eq!(config.local_path, PathBuf::from("/custom/path"));
}

#[tokio::test]
async fn test_local_folddb_has_no_s3_storage() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_db");
    
    let db = FoldDB::new(db_path.to_str().unwrap()).await.unwrap();
    
    assert!(!db.has_s3_storage());
}



