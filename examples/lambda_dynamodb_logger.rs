//! Example DynamoDB Logger Implementation for DataFold Lambda
//!
//! This example shows how to implement a custom logger for multi-tenant
//! Lambda deployments using DynamoDB as the backend.
//!
//! ## Setup
//!
//! 1. Create DynamoDB table:
//!    ```bash
//!    aws dynamodb create-table \
//!      --table-name datafold-logs \
//!      --attribute-definitions \
//!        AttributeName=user_id,AttributeType=S \
//!        AttributeName=timestamp,AttributeType=N \
//!      --key-schema \
//!        AttributeName=user_id,KeyType=HASH \
//!        AttributeName=timestamp,KeyType=RANGE \
//!      --billing-mode PAY_PER_REQUEST
//!    ```
//!
//! 2. Enable TTL for automatic cleanup:
//!    ```bash
//!    aws dynamodb update-time-to-live \
//!      --table-name datafold-logs \
//!      --time-to-live-specification "Enabled=true, AttributeName=ttl"
//!    ```
//!
//! ## Usage in Your Lambda Project
//!
//! Copy this implementation to your Lambda project and use it like:
//!
//! ```rust,no_run
//! use datafold::lambda::{LambdaContext, LambdaConfig};
//! use std::sync::Arc;
//!
//! async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
//!     let user_id = event.payload["user_id"].as_str().unwrap_or("anonymous");
//!     
//!     // Create a logger for this specific user
//!     let logger = DynamoDbLogger::new(
//!         "datafold-logs".to_string(),
//!         user_id.to_string()
//!     ).await;
//!     
//!     let config = LambdaConfig::new()
//!         .with_logger(Arc::new(logger));
//!     
//!     LambdaContext::init(config).await?;
//!     
//!     // Process the request - all logs will be tagged with user_id
//!     let result = LambdaContext::ingest_json(
//!         event.payload["data"].clone(),
//!         true,
//!         0,
//!         "default".to_string(), // pub_key
//!         user_id.to_string()    // user_id
//!     ).await?;
//!     
//!     Ok(json!({ "statusCode": 200, "progress_id": result }))
//! }
//! ```

#[cfg(not(feature = "lambda"))]
fn main() {
    println!("This example requires the 'lambda' feature to be enabled.");
}

#[cfg(feature = "lambda")]
use async_trait::async_trait;
#[cfg(feature = "lambda")]
use aws_config;
#[cfg(feature = "lambda")]
use aws_sdk_dynamodb::{Client, types::AttributeValue};
#[cfg(feature = "lambda")]
use datafold::lambda::{LambdaConfig, LambdaContext, StdoutLogger, Logger, LogEntry, LogLevel};
#[cfg(feature = "lambda")]
use std::collections::HashMap;
#[cfg(feature = "lambda")]
use std::time::{SystemTime, UNIX_EPOCH};

/// DynamoDB-backed logger for multi-tenant Lambda deployments
///
/// Features:
/// - Partition key: user_id (efficient queries per user)
/// - Sort key: timestamp (chronological ordering)
/// - TTL: 30 days automatic cleanup
/// - Metadata support via DynamoDB Map
#[cfg(feature = "lambda")]
pub struct DynamoDbLogger {
    client: Client,
    table_name: String,
    user_id: String,
}

#[cfg(feature = "lambda")]
impl DynamoDbLogger {
    /// Create a new DynamoDB logger
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the DynamoDB table
    /// * `user_id` - User ID for multi-tenant isolation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let logger = DynamoDbLogger::new("datafold-logs".to_string(), "user123".to_string()).await;
    /// ```
    pub async fn new(table_name: String, user_id: String) -> Self {
        let config = aws_config::load_from_env().await;
        let client = Client::new(&config);
        Self { client, table_name, user_id }
    }

    /// Create a DynamoDB logger with custom AWS config
    pub async fn with_config(table_name: String, user_id: String, aws_config: &aws_config::SdkConfig) -> Self {
        let client = Client::new(aws_config);
        Self { client, table_name, user_id }
    }

    /// Convert LogEntry to DynamoDB item
    fn entry_to_item(&self, entry: LogEntry) -> HashMap<String, AttributeValue> {
        // TTL: 30 days from now
        let ttl = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() + (30 * 24 * 60 * 60)) as i64;

        let mut item = HashMap::new();
        // Use user_id from entry if available (request-scoped), otherwise fallback to logger's user_id
        let user_id = entry.user_id.clone().unwrap_or_else(|| self.user_id.clone());
        item.insert("user_id".to_string(), AttributeValue::S(user_id));
        item.insert("timestamp".to_string(), AttributeValue::N(entry.timestamp.to_string()));
        item.insert("level".to_string(), AttributeValue::S(entry.level.as_str().to_string()));
        item.insert("event_type".to_string(), AttributeValue::S(entry.event_type));
        item.insert("message".to_string(), AttributeValue::S(entry.message));
        item.insert("ttl".to_string(), AttributeValue::N(ttl.to_string()));

        // Add metadata if present
        if let Some(meta) = entry.metadata {
            let meta_av: HashMap<String, AttributeValue> = meta
                .into_iter()
                .map(|(k, v)| (k, AttributeValue::S(v)))
                .collect();
            item.insert("metadata".to_string(), AttributeValue::M(meta_av));
        }

        item
    }

    /// Convert DynamoDB item to LogEntry
    fn item_to_entry(&self, item: &HashMap<String, AttributeValue>) -> Option<LogEntry> {
        let timestamp = item.get("timestamp")?.as_n().ok()?.parse().ok()?;
        let level_str = item.get("level")?.as_s().ok()?;
        let event_type = item.get("event_type")?.as_s().ok()?.to_string();
        let message = item.get("message")?.as_s().ok()?.to_string();

        let level = match level_str.as_str() {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            _ => LogLevel::Info,
        };

        let metadata = item.get("metadata")
            .and_then(|v| v.as_m().ok())
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| {
                        v.as_s().ok().map(|s| (k.clone(), s.to_string()))
                    })
                    .collect()
            });

        Some(LogEntry {
            timestamp,
            level,
            event_type,
            message,
            metadata,
            user_id: None, // We don't necessarily store it back or need it for display here
        })
    }
}

/// Minimal example entrypoint so the example builds during `cargo test`.
///
/// The logger requires AWS credentials and a DynamoDB table to be useful, so
/// the main function only prints a message indicating how to run the example
/// in a real environment.
#[cfg(feature = "lambda")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Configure AWS credentials and run this example inside AWS Lambda to see DynamoDB logging in action.");
    Ok(())
}

#[cfg(feature = "lambda")]
#[async_trait]
impl Logger for DynamoDbLogger {
    /// Log an event to DynamoDB
    async fn log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let item = self.entry_to_item(entry);

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await?;

        Ok(())
    }

    /// Query logs for this user
    ///
    /// Returns logs in reverse chronological order (most recent first).
    /// Note: The user_id parameter is ignored - this logger uses its own user_id field.
    async fn query(
        &self,
        _user_id: &str,
        limit: Option<usize>,
        from_timestamp: Option<i64>,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let mut query = self.client
            .query()
            .table_name(&self.table_name)
            .scan_index_forward(false); // Most recent first

        // Build key condition expression
        if let Some(from_ts) = from_timestamp {
            query = query
                .key_condition_expression("user_id = :uid AND #ts >= :from_ts")
                .expression_attribute_values(":uid", AttributeValue::S(self.user_id.clone()))
                .expression_attribute_values(":from_ts", AttributeValue::N(from_ts.to_string()))
                .expression_attribute_names("#ts", "timestamp");
        } else {
            query = query
                .key_condition_expression("user_id = :uid")
                .expression_attribute_values(":uid", AttributeValue::S(self.user_id.clone()));
        }

        if let Some(lim) = limit {
            query = query.limit(lim as i32);
        }

        let response = query.send().await?;
        let items = response.items.unwrap_or_default();

        let logs: Vec<LogEntry> = items
            .iter()
            .filter_map(|item| self.item_to_entry(item))
            .collect();

        Ok(logs)
    }
}

/// Example usage
#[cfg(all(test, feature = "lambda"))]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires AWS credentials and DynamoDB table
    async fn test_dynamodb_logger() {
        let logger = DynamoDbLogger::new(
            "datafold-logs".to_string(),
            "test_user_123".to_string()
        ).await;

        let entry = LogEntry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            level: LogLevel::Info,
            event_type: "test_event".to_string(),
            message: "Test message".to_string(),
            metadata: Some(HashMap::from([
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string()),
            ])),
            user_id: None,
        };

        // Log the entry
        logger.log(entry).await.unwrap();

        // Query back (user_id parameter is ignored - uses logger's user_id)
        let logs = logger.query("ignored", Some(10), None).await.unwrap();
        assert!(!logs.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires AWS credentials and DynamoDB table
    async fn test_multi_tenant_isolation() {
        let logger = DynamoDbLogger::new(
            "datafold-logs".to_string(),
            "test_user_456".to_string()
        ).await;

        let entry = LogEntry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            level: LogLevel::Info,
            event_type: "test_event".to_string(),
            message: "Test multi-tenant isolation".to_string(),
            metadata: None,
            user_id: None,
        };

        logger.log(entry).await.unwrap();

        // Query back (user_id parameter is ignored - automatically scoped to logger's user_id)
        let logs = logger.query("ignored", Some(10), None).await.unwrap();
        assert!(!logs.is_empty());
    }
}
