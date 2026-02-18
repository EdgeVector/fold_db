pub mod config;
#[cfg(feature = "aws-backend")]
pub mod dynamodb_backend;
#[cfg(feature = "aws-backend")]
pub mod dynamodb_store;
#[cfg(feature = "aws-backend")]
pub mod dynamodb_utils;
pub mod encrypting_namespaced_store;
pub mod encrypting_store;
pub mod error;
pub mod exemem_api_store;
pub mod exemem_namespaced_store;
pub mod inmemory_backend;
#[cfg(feature = "aws-backend")]
pub mod message_bus;
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
pub use config::{CloudConfig, ExplicitTables};
#[cfg(feature = "aws-backend")]
pub type DynamoDbConfig = CloudConfig;
#[cfg(feature = "aws-backend")]
pub use dynamodb_backend::{DynamoDbNamespacedStore as CloudNamespacedStore, TableNameResolver};
#[cfg(feature = "aws-backend")]
pub type DynamoDbNamespacedStore = CloudNamespacedStore;
#[cfg(feature = "aws-backend")]
pub use dynamodb_store::DynamoDbSchemaStore;
pub use error::StorageError;
pub use inmemory_backend::InMemoryNamespacedStore;

pub use encrypting_namespaced_store::EncryptingNamespacedStore;
pub use encrypting_store::EncryptingKvStore;
pub use exemem_api_store::{ExememApiStore, ExememAuth};
pub use exemem_namespaced_store::ExememNamespacedStore;
#[cfg(feature = "aws-backend")]
pub use message_bus::CloudMessageBus;
pub use sled_backend::SledNamespacedStore;
pub use traits::{NamespacedStore, TypedKvStore};
pub use upload_storage::UploadStorage;
