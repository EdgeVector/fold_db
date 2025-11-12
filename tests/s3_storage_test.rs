use datafold::{FoldDB, S3Config, StorageConfig};
use std::path::PathBuf;

#[test]
fn test_storage_config_from_env_defaults_to_local() {
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
    std::env::remove_var("DATAFOLD_S3_BUCKET");
    std::env::set_var("DATAFOLD_S3_REGION", "us-west-2");
    
    let result = S3Config::from_env();
    assert!(result.is_err());
    
    std::env::remove_var("DATAFOLD_S3_REGION");
}

#[test]
fn test_s3_config_from_env_with_all_vars() {
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

#[test]
fn test_local_folddb_has_no_s3_storage() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_db");
    
    let db = FoldDB::new(db_path.to_str().unwrap()).unwrap();
    
    assert!(!db.has_s3_storage());
}

// Integration tests with actual S3 would require AWS credentials and a test bucket
// These are marked as ignored and can be run manually with proper AWS setup

#[tokio::test]
#[ignore]
async fn test_s3_folddb_creation() {
    // This test requires:
    // - AWS credentials configured
    // - DATAFOLD_S3_BUCKET, DATAFOLD_S3_REGION env vars set
    // Run with: cargo test test_s3_folddb_creation -- --ignored --nocapture
    
    let config = S3Config::from_env().expect("S3 config from environment");
    let db = FoldDB::new_with_s3(config).await.expect("Create FoldDB with S3");
    
    assert!(db.has_s3_storage());
}

#[tokio::test]
#[ignore]
async fn test_s3_flush() {
    // This test requires:
    // - AWS credentials configured
    // - DATAFOLD_S3_BUCKET, DATAFOLD_S3_REGION env vars set
    // Run with: cargo test test_s3_flush -- --ignored --nocapture
    
    let config = S3Config::from_env().expect("S3 config from environment");
    let db = FoldDB::new_with_s3(config).await.expect("Create FoldDB with S3");
    
    // Perform a flush to S3
    db.flush_to_s3().await.expect("Flush to S3 should succeed");
}

