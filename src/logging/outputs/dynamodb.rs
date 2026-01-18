use crate::logging::core::{LogEntry, LogLevel, Logger};
use async_trait::async_trait;
use aws_sdk_dynamodb::{
    types::{
        AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ProvisionedThroughput,
        ScalarAttributeType,
    },
    Client,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Internal struct for DynamoDB serialization
#[derive(Debug, Serialize, Deserialize)]
struct DynamoDbLogItem {
    user_id: String,
    timestamp: i64,
    level: LogLevel,
    event_type: String,
    message: String,
    ttl: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<HashMap<String, String>>,
}

impl DynamoDbLogItem {
    fn try_from_entry(entry: LogEntry) -> Result<Self, String> {
        let ttl = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + (30 * 24 * 60 * 60)) as i64;

        Ok(Self {
            user_id: entry
                .user_id
                .ok_or_else(|| "Missing user_id for DynamoDB log entry".to_string())?,
            timestamp: entry.timestamp,
            level: entry.level,
            event_type: entry.event_type,
            message: entry.message,
            ttl,
            metadata: entry.metadata,
        })
    }

    fn into_entry(self) -> LogEntry {
        LogEntry {
            timestamp: self.timestamp,
            level: self.level,
            event_type: self.event_type,
            message: self.message,
            user_id: Some(self.user_id),
            metadata: self.metadata,
        }
    }
}

/// DynamoDB-backed logger for multi-tenant Lambda deployments
///
/// Features:
/// - Partition key: user_id (efficient queries per user)
/// - Sort key: timestamp (chronological ordering)
/// - TTL: 30 days automatic cleanup
/// - Metadata support via DynamoDB Map
///
/// This logger is stateless and thread-safe. It uses the `user_id` present in
/// each `LogEntry` to partition logs, making it suitable for multi-tenant
/// environments where a single Lambda container handles requests for multiple users.
pub struct DynamoDbLogger {
    client: Client,
    table_name: String,
}

impl DynamoDbLogger {
    /// Create a new DynamoDB logger and ensure table exists
    pub async fn new(table_name: String) -> Self {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);
        let logger = Self { client, table_name };
        let _ = logger.ensure_table_exists().await;
        logger
    }

    /// Create a DynamoDB logger with custom AWS config
    pub async fn with_config(table_name: String, aws_config: &aws_config::SdkConfig) -> Self {
        let client = Client::new(aws_config);
        let logger = Self { client, table_name };
        let _ = logger.ensure_table_exists().await;
        logger
    }

    /// Create a DynamoDB logger with an existing client
    pub fn with_client(table_name: String, client: Client) -> Self {
        // Note: Can't easily ensure table exists here without async,
        // so we assume caller handled it or it will happen lazily/fail.
        Self { client, table_name }
    }

    /// Ensure the log table exists, creating it if necessary
    pub async fn ensure_table_exists(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let result = self
            .client
            .create_table()
            .table_name(&self.table_name)
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("user_id")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .unwrap(),
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("timestamp")
                    .attribute_type(ScalarAttributeType::N)
                    .build()
                    .unwrap(),
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("user_id")
                    .key_type(KeyType::Hash)
                    .build()
                    .unwrap(),
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("timestamp")
                    .key_type(KeyType::Range)
                    .build()
                    .unwrap(),
            )
            .provisioned_throughput(
                ProvisionedThroughput::builder()
                    .read_capacity_units(5)
                    .write_capacity_units(5)
                    .build()
                    .unwrap(),
            )
            .send()
            .await;

        match result {
            Ok(_) => {
                // Wait a bit for table to be active?
                Ok(())
            }
            Err(err) => {
                if let Some(service_err) = err.as_service_error() {
                    if service_err.is_resource_in_use_exception() {
                        return Ok(());
                    }
                }
                Err(Box::new(err))
            }
        }
    }
}

#[async_trait]
impl Logger for DynamoDbLogger {
    /// Log an event to DynamoDB
    async fn log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let item_struct = DynamoDbLogItem::try_from_entry(entry)
            .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
        let item = serde_dynamo::to_item(item_struct)?;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await?;

        Ok(())
    }

    /// Query logs for a specific user
    ///
    /// Returns logs in reverse chronological order (most recent first).
    async fn query(
        &self,
        user_id: &str,
        limit: Option<usize>,
        from_timestamp: Option<i64>,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let mut query = self
            .client
            .query()
            .table_name(&self.table_name)
            .scan_index_forward(false); // Most recent first

        // Build key condition expression
        if let Some(from_ts) = from_timestamp {
            query = query
                .key_condition_expression("user_id = :uid AND #ts >= :from_ts")
                .expression_attribute_values(":uid", AttributeValue::S(user_id.to_string()))
                .expression_attribute_values(":from_ts", AttributeValue::N(from_ts.to_string()))
                .expression_attribute_names("#ts", "timestamp");
        } else {
            query = query
                .key_condition_expression("user_id = :uid")
                .expression_attribute_values(":uid", AttributeValue::S(user_id.to_string()));
        }

        if let Some(lim) = limit {
            query = query.limit(lim as i32);
        }

        let response = query.send().await?;

        if let Some(items) = response.items {
            let log_items: Vec<DynamoDbLogItem> = serde_dynamo::from_items(items)?;
            Ok(log_items
                .into_iter()
                .map(|item| item.into_entry())
                .collect())
        } else {
            Ok(vec![])
        }
    }
}
