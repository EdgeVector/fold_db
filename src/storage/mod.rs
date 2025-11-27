pub mod s3_sync;
pub mod config;
pub mod error;
pub mod upload_storage;
pub mod dynamodb_store;

// Storage abstraction layer
pub mod traits;
pub mod sled_backend;
pub mod dynamodb_backend;
pub mod inmemory_backend;

#[cfg(test)]
mod tests;

pub use config::{S3Config, StorageConfig, UploadStorageConfig};
pub use error::{StorageError, StorageResult};
pub use s3_sync::S3SyncedStorage;
pub use upload_storage::UploadStorage;
pub use dynamodb_store::{DynamoDbSchemaStore, DynamoDbConfig};

// Export storage abstraction types
pub use traits::{KvStore, NamespacedStore, TypedStore, TypedKvStore};
pub use sled_backend::{SledKvStore, SledNamespacedStore};
pub use dynamodb_backend::{DynamoDbKvStore, DynamoDbNamespacedStore};
pub use inmemory_backend::{InMemoryKvStore, InMemoryNamespacedStore};

