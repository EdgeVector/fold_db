pub mod config;
pub mod dynamodb_backend;
pub mod dynamodb_store;
pub mod dynamodb_utils;
pub mod error;
pub mod inmemory_backend;
pub mod reset_manager;
pub mod sled_backend;
pub mod traits;
pub mod upload_storage;

#[cfg(test)]
mod tests;

// Re-exports for convenience and backward compatibility
pub use config::{StorageConfig};
pub use dynamodb_backend::{DynamoDbNamespacedStore, TableNameResolver};
pub use dynamodb_store::{DynamoDbConfig, DynamoDbSchemaStore};
pub use error::StorageError;
pub use inmemory_backend::InMemoryNamespacedStore;

pub use sled_backend::SledNamespacedStore;
pub use traits::{NamespacedStore, TypedKvStore};
pub use upload_storage::UploadStorage;
