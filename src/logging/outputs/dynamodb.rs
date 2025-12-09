use async_trait::async_trait;
use aws_config;
use aws_sdk_dynamodb::{Client, types::AttributeValue};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::lambda::logging::{Logger, LogEntry, LogLevel};

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
    /// Create a new DynamoDB logger
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the DynamoDB table
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() {
    /// use datafold::logging::outputs::dynamodb::DynamoDbLogger;
    /// let logger = DynamoDbLogger::new("datafold-logs".to_string()).await;
    /// # }
    /// ```
    pub async fn new(table_name: String) -> Self {
        let config = aws_config::load_from_env().await;
        let client = Client::new(&config);
        Self { client, table_name }
    }

    /// Create a DynamoDB logger with custom AWS config
    pub async fn with_config(table_name: String, aws_config: &aws_config::SdkConfig) -> Self {
        let client = Client::new(aws_config);
        Self { client, table_name }
    }
    
    /// Create a DynamoDB logger with an existing client
    pub fn with_client(table_name: String, client: Client) -> Self {
        Self { client, table_name }
    }

    /// Convert LogEntry to DynamoDB item
    fn entry_to_item(&self, entry: LogEntry) -> HashMap<String, AttributeValue> {
        // TTL: 30 days from now
        let ttl = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() + (30 * 24 * 60 * 60)) as i64;

        let mut item = HashMap::new();
        // Use user_id from entry, defaulting to "anonymous" if missing
        let user_id = entry.user_id.clone().unwrap_or_else(|| "anonymous".to_string());
        
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
        let user_id = item.get("user_id").and_then(|v| v.as_s().ok()).map(|s| s.to_string());

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
            user_id,
        })
    }
}

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

    /// Query logs for a specific user
    ///
    /// Returns logs in reverse chronological order (most recent first).
    async fn query(
        &self,
        user_id: &str,
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
        let items = response.items.unwrap_or_default();

        let logs: Vec<LogEntry> = items
            .iter()
            .filter_map(|item| self.item_to_entry(item))
            .collect();

        Ok(logs)
    }
}
