use crate::error::{FoldDbError, FoldDbResult};
use crate::fold_db_core::infrastructure::message_bus::{MessageBus, MessageStream};
use crate::storage::CloudConfig;
use async_trait::async_trait;
use aws_sdk_sns::Client as SnsClient;
use aws_sdk_sqs::Client as SqsClient;

/// Cloud-based message bus using AWS SNS for publishing and SQS for subscription
#[allow(dead_code)] // Fields used in implementation
pub struct CloudMessageBus {
    sns_client: SnsClient,
    sqs_client: SqsClient,
    config: CloudConfig,
    topic_arn_prefix: String,
}

impl CloudMessageBus {
    pub async fn new(config: CloudConfig) -> FoldDbResult<Self> {
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_sdk_dynamodb::config::Region::new(config.region.clone()))
            .load()
            .await;

        let sns_client = SnsClient::new(&aws_config);
        let sqs_client = SqsClient::new(&aws_config);

        // Construct a prefix for topic ARNs. This assumes standard AWS ARN format.
        // real ARN construction might need account ID, but we can also look up topics by name
        // or create them idempotent.
        // For now, we will assume topics are created externally or we create them.

        Ok(Self {
            sns_client,
            sqs_client,
            config,
            topic_arn_prefix: String::new(), // TODO: discover account ID or use lookups
        })
    }

    async fn get_create_topic(&self, topic_name: &str) -> FoldDbResult<String> {
        let resp = self
            .sns_client
            .create_topic()
            .name(topic_name)
            .send()
            .await
            .map_err(|e| FoldDbError::Other(format!("Failed to create/get SNS topic: {}", e)))?;

        resp.topic_arn
            .ok_or_else(|| FoldDbError::Other("SNS response missing topic ARN".to_string()))
    }

    async fn get_create_queue(&self, queue_name: &str) -> FoldDbResult<String> {
        let resp = self
            .sqs_client
            .create_queue()
            .queue_name(queue_name)
            .send()
            .await
            .map_err(|e| FoldDbError::Other(format!("Failed to create/get SQS queue: {}", e)))?;

        resp.queue_url
            .ok_or_else(|| FoldDbError::Other("SQS response missing queue URL".to_string()))
    }
}

#[async_trait]
impl MessageBus for CloudMessageBus {
    async fn publish(&self, topic: &str, message: &[u8]) -> FoldDbResult<()> {
        let topic_arn = self.get_create_topic(topic).await?;

        // Base64 encode the message if sending binary, or assume string if UTF-8
        // SNS Message attribute is string. References say we should use String.
        // For arbitrary bytes, we might want to base64 encode.
        // But for simplicity let's try to convert to utf8 or fail?
        // Or use MessageAttributes for binary.
        // Let's base64 encode the payload into the body.

        // Use base64 for safety
        use base64::{engine::general_purpose, Engine as _};
        let payload = general_purpose::STANDARD.encode(message);

        self.sns_client
            .publish()
            .topic_arn(topic_arn)
            .message(payload)
            .send()
            .await
            .map_err(|e| FoldDbError::Other(format!("Failed to publish to SNS: {}", e)))?;

        Ok(())
    }

    async fn subscribe(&self, topic: &str) -> FoldDbResult<MessageStream> {
        let topic_arn = self.get_create_topic(topic).await?;

        // Create a queue for this subscription
        // Queue name: app-topic-uuid or similar
        let queue_name = format!("datafold-{}-sub-{}", topic, uuid::Uuid::new_v4());
        let queue_url = self.get_create_queue(&queue_name).await?;

        // Get Queue ARN for policy
        let queue_attrs = self
            .sqs_client
            .get_queue_attributes()
            .queue_url(&queue_url)
            .attribute_names(aws_sdk_sqs::types::QueueAttributeName::QueueArn)
            .send()
            .await
            .map_err(|e| FoldDbError::Other(format!("Failed to get queue attributes: {}", e)))?;

        let queue_arn = queue_attrs
            .attributes()
            .ok_or(FoldDbError::Other("No attributes returned".to_string()))?
            .get(&aws_sdk_sqs::types::QueueAttributeName::QueueArn)
            .ok_or(FoldDbError::Other("QueueArn missing".to_string()))?;

        // Subscribe queue to SNS topic
        self.sns_client
            .subscribe()
            .topic_arn(&topic_arn)
            .protocol("sqs")
            .endpoint(queue_arn)
            .send()
            .await
            .map_err(|e| FoldDbError::Other(format!("Failed to subscribe queue to SNS: {}", e)))?;

        // Construct stream
        // We need to clone client for the stream
        let sqs_client = self.sqs_client.clone();
        let q_url = queue_url.clone();

        let stream = async_stream::stream! {
            loop {
                // Long poll
                let result = sqs_client.receive_message()
                    .queue_url(&q_url)
                    .wait_time_seconds(20)
                    .max_number_of_messages(10)
                    .send()
                    .await;

                match result {
                    Ok(output) => {
                        if let Some(messages) = output.messages {
                            for msg in messages {
                                if let Some(body) = msg.body {
                                    // SNS wraps the message in JSON. We need to parse it.
                                    #[derive(serde::Deserialize)]
                                    struct SnsNotification {
                                        #[serde(rename = "Message")]
                                        message: String,
                                    }

                                    let payload_bytes = match serde_json::from_str::<SnsNotification>(&body) {
                                        Ok(sns_msg) => {
                                             use base64::{engine::general_purpose, Engine as _};
                                             match general_purpose::STANDARD.decode(&sns_msg.message) {
                                                 Ok(b) => b,
                                                 Err(_) => sns_msg.message.into_bytes(), // Fallback if not base64
                                             }
                                        },
                                        Err(_) => body.into_bytes(), // Raw SQS message?
                                    };

                                    yield Ok(payload_bytes);

                                    // Delete message after processing (at least yielding)
                                    // In a real system, we might want explicit ack.
                                    // For now, auto-ack.
                                    if let Some(receipt_handle) = msg.receipt_handle {
                                        let _ = sqs_client.delete_message()
                                            .queue_url(&q_url)
                                            .receipt_handle(receipt_handle)
                                            .send()
                                            .await;
                                    }
                                }
                            }
                        }
                    },
                    Err(e) => {
                         yield Err(FoldDbError::Other(format!("SQS receive error: {}", e)));
                         // Backoff?
                         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}
