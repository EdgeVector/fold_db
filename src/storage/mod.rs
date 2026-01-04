pub mod config;
#[cfg(feature = "aws-backend")]
pub mod dynamodb_backend;
#[cfg(feature = "aws-backend")]
pub mod dynamodb_store;
#[cfg(feature = "aws-backend")]
pub mod dynamodb_utils;
pub mod error;
pub mod inmemory_backend;
#[cfg(feature = "aws-backend")]
pub mod reset_manager;
pub mod sled_backend;
pub mod traits;
pub mod upload_storage;

#[cfg(test)]
mod tests;

// Re-exports for convenience and backward compatibility
pub use config::DatabaseConfig;
#[cfg(feature = "aws-backend")]
pub use config::{DynamoDbConfig, ExplicitTables};
#[cfg(feature = "aws-backend")]
pub use dynamodb_backend::{DynamoDbNamespacedStore, TableNameResolver};
#[cfg(feature = "aws-backend")]
pub use dynamodb_store::DynamoDbSchemaStore;
pub use error::StorageError;
pub use inmemory_backend::InMemoryNamespacedStore;

pub use sled_backend::SledNamespacedStore;
pub use traits::{NamespacedStore, TypedKvStore};
pub use upload_storage::UploadStorage;
