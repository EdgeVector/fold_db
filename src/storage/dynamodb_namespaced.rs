//! DynamoDB-backed namespaced store with table management.
//!
//! Handles namespace-to-table resolution, table creation, and
//! dispatching to the appropriate KvStore implementation.

use super::dynamodb_backend::DynamoDbKvStore;
use super::dynamodb_native_index::DynamoDbNativeIndexStore;
use super::error::{StorageError, StorageResult};
use super::traits::{KvStore, NamespacedStore};
use async_trait::async_trait;
use aws_sdk_dynamodb::types::{
    AttributeDefinition, BillingMode, KeySchemaElement, KeyType, ScalarAttributeType, TableStatus,
};
use aws_sdk_dynamodb::Client;
use std::collections::HashMap;
use std::sync::Arc;

/// Strategy for resolving table names from namespaces
#[derive(Clone, Debug)]
pub enum TableNameResolver {
    /// Append namespace to prefix: "{prefix}-{namespace}"
    Prefix(String),
    /// Map namespace to exact table name. keys are namespaces ("main", "metadata", etc)
    Explicit(HashMap<String, String>),
}

pub struct DynamoDbNamespacedStore {
    client: Arc<Client>,
    /// Strategy to resolve namespace to table name
    resolver: TableNameResolver,
    /// Whether to automatically create tables if they don't exist
    auto_create: bool,
}

impl DynamoDbNamespacedStore {
    /// Create a new DynamoDB NamespacedStore with flexible configuration
    pub fn new(client: Client, resolver: TableNameResolver, auto_create: bool) -> Self {
        Self {
            client: Arc::new(client),
            resolver,
            auto_create,
        }
    }

    /// Create a new DynamoDB NamespacedStore with legacy prefix behavior (auto-create enabled)
    pub fn new_with_prefix(client: Client, prefix: String) -> Self {
        Self::new(client, TableNameResolver::Prefix(prefix), true)
    }

    /// Generate table name for a namespace
    fn table_name_for_namespace(&self, namespace: &str) -> StorageResult<String> {
        match &self.resolver {
            TableNameResolver::Prefix(prefix) => Ok(format!("{}-{}", prefix, namespace)),
            TableNameResolver::Explicit(map) => map.get(namespace).cloned().ok_or_else(|| {
                StorageError::ConfigurationError(format!(
                    "No explicit table name configured for namespace '{}'",
                    namespace
                ))
            }),
        }
    }

    /// Test helper to get table name for a namespace
    #[cfg(test)]
    pub fn get_table_name_for_namespace(&self, namespace: &str) -> String {
        self.table_name_for_namespace(namespace)
            .unwrap_or_else(|_| "unknown".to_string())
    }

    /// Ensure a DynamoDB table exists, creating it if necessary
    async fn ensure_table_exists(&self, table_name: &str) -> StorageResult<()> {
        // First, check if the table exists
        match self
            .client
            .describe_table()
            .table_name(table_name)
            .send()
            .await
        {
            Ok(response) => {
                // Table exists, check if it's active
                if let Some(table) = response.table() {
                    if let Some(status) = table.table_status() {
                        if status == &aws_sdk_dynamodb::types::TableStatus::Active {
                            // Table exists and is active, we're good
                            return Ok(());
                        } else {
                            // Table exists but not active yet - wait a bit
                            log::debug!(
                                "Table {} exists but status is {:?}, waiting...",
                                table_name,
                                status
                            );
                            // For now, we'll proceed anyway as the table will become active soon
                            return Ok(());
                        }
                    }
                }
                // Table exists (even if we couldn't check status), we're good
                return Ok(());
            }
            Err(e) => {
                let error_str = e.to_string();
                // Check for ResourceNotFoundException specifically
                if error_str.contains("ResourceNotFoundException") {
                    // Table doesn't exist, we'll create it below
                } else if error_str.contains("service error") {
                    // "service error" is often a transient error or permissions issue
                    // Try to proceed - if the table doesn't exist, creation will fail
                    // If it does exist, operations will work
                    log::warn!("Got 'service error' when checking table {} - proceeding to attempt creation", table_name);
                    // Do NOT return Ok(()) here; let it fall through to create_table
                } else {
                    // For other errors, still try to proceed but log a warning
                    log::warn!(
                        "Unexpected error checking table {}: {} - proceeding anyway",
                        table_name,
                        error_str
                    );
                    // Don't fail immediately - let the create attempt below handle it
                }
            }
        }

        // Table doesn't exist, create it
        let create_result = self
            .client
            .create_table()
            .table_name(table_name)
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("PK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .map_err(|e| {
                        StorageError::DynamoDbError(format!(
                            "Failed to build attribute definition: {}",
                            e
                        ))
                    })?,
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("SK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .map_err(|e| {
                        StorageError::DynamoDbError(format!(
                            "Failed to build attribute definition: {}",
                            e
                        ))
                    })?,
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("PK")
                    .key_type(KeyType::Hash)
                    .build()
                    .map_err(|e| {
                        StorageError::DynamoDbError(format!("Failed to build key schema: {}", e))
                    })?,
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("SK")
                    .key_type(KeyType::Range)
                    .build()
                    .map_err(|e| {
                        StorageError::DynamoDbError(format!("Failed to build key schema: {}", e))
                    })?,
            )
            .billing_mode(BillingMode::PayPerRequest)
            .send()
            .await;

        match create_result {
            Ok(_) => {
                // Wait for table to be ACTIVE before returning
                // Poll with exponential backoff (max 30 seconds total)
                let mut attempts = 0;
                const MAX_ATTEMPTS: u32 = 30;

                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                    match self
                        .client
                        .describe_table()
                        .table_name(table_name)
                        .send()
                        .await
                    {
                        Ok(response) => {
                            if let Some(table) = response.table {
                                if let Some(status) = table.table_status {
                                    if matches!(status, TableStatus::Active) {
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // Continue polling
                        }
                    }

                    attempts += 1;
                    if attempts >= MAX_ATTEMPTS {
                        return Err(StorageError::DynamoDbError(format!(
                            "Table '{}' did not become ACTIVE within timeout",
                            table_name
                        )));
                    }
                }
            }
            Err(e) => {
                // If table was created by another process between our check and create, that's ok
                if e.to_string().contains("ResourceInUseException") {
                    Ok(())
                } else {
                    Err(StorageError::DynamoDbError(format!(
                        "Failed to create table {}: {}",
                        table_name, e
                    )))
                }
            }
        }
    }
}

#[async_trait]
impl NamespacedStore for DynamoDbNamespacedStore {
    async fn open_namespace(&self, name: &str) -> StorageResult<Arc<dyn KvStore>> {
        let table_name = self.table_name_for_namespace(name)?;

        // Ensure the table exists if auto_create is enabled
        if self.auto_create {
            self.ensure_table_exists(&table_name).await?;
        }

        // For native_index namespace, use simplified key structure: feature as PK, term as SK
        // This enables efficient queries by feature type (word, email, etc.)
        if name == "native_index" {
            let store = DynamoDbNativeIndexStore::new(self.client.clone(), table_name);
            Ok(Arc::new(store))
        } else {
            let store = DynamoDbKvStore::new(self.client.clone(), table_name);
            Ok(Arc::new(store))
        }
    }

    async fn list_namespaces(&self) -> StorageResult<Vec<String>> {
        // This would require scanning all keys and extracting unique namespaces
        // For now, we'll return an error indicating it's not implemented
        Err(StorageError::InvalidOperation(
            "list_namespaces not implemented for DynamoDB - requires custom implementation"
                .to_string(),
        ))
    }

    async fn delete_namespace(&self, _name: &str) -> StorageResult<bool> {
        // Would need to scan and delete all items with the namespace prefix
        Err(StorageError::InvalidOperation(
            "delete_namespace not implemented for DynamoDB - requires custom implementation"
                .to_string(),
        ))
    }
}
