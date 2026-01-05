//! Asynchronous message bus implementation
//!
//! This module provides the asynchronous message bus that uses tokio::sync::mpsc
//! for async communication between components.

use super::error_handling::{AsyncRecvError, AsyncTryRecvError, MessageBusError, MessageBusResult};
use super::events::{Event, EventType};
use super::{
    atom_events::{AtomCreated, FieldValueSet},
    query_events::{MutationExecuted, QueryExecuted},
};
use log::warn;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc as async_mpsc;
use tokio::time::{timeout, Duration as AsyncDuration};

/// Default channel capacity for subscribers
const DEFAULT_CHANNEL_CAPACITY: usize = 1000;

/// Trait for async event handlers
pub trait AsyncEventHandler<T: EventType>: Send + Sync {
    fn handle(&self, event: T) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;
}

/// Async consumer for event handling in async contexts
pub struct AsyncConsumer<T> {
    receiver: async_mpsc::Receiver<T>,
}

impl AsyncConsumer<Event> {
    /// Create a new async consumer
    pub(crate) fn new(receiver: async_mpsc::Receiver<Event>) -> Self {
        Self { receiver }
    }

    /// Async receive without blocking
    pub async fn recv(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }

    /// Async receive with timeout
    pub async fn recv_timeout(&mut self, duration: AsyncDuration) -> Result<Event, AsyncRecvError> {
        match timeout(duration, self.receiver.recv()).await {
            Ok(Some(event)) => Ok(event),
            Ok(None) => Err(AsyncRecvError::Disconnected),
            Err(_) => Err(AsyncRecvError::Timeout),
        }
    }

    /// Try to receive an event without waiting
    pub fn try_recv(&mut self) -> Result<Event, AsyncTryRecvError> {
        match self.receiver.try_recv() {
            Ok(event) => Ok(event),
            Err(async_mpsc::error::TryRecvError::Empty) => Err(AsyncTryRecvError::Empty),
            Err(async_mpsc::error::TryRecvError::Disconnected) => {
                Err(AsyncTryRecvError::Disconnected)
            }
        }
    }

    /// Filter events to specific type
    pub async fn recv_filtered<T: EventType>(&mut self) -> Option<T> {
        while let Some(event) = self.recv().await {
            if let Some(typed_event) = self.extract_typed_event::<T>(event) {
                return Some(typed_event);
            }
        }
        None
    }

    /// Extract typed event from unified Event enum
    fn extract_typed_event<T: EventType>(&self, _event: Event) -> Option<T> {
        // This is a helper method to extract specific event types from the unified Event
        // Implementation depends on how we want to handle this conversion
        // For now, return None as this is a complex type conversion
        None
    }
}

/// Async subscriber registry for managing async event subscribers
struct AsyncSubscriberRegistry {
    // Use unified Event type for simplicity and type safety
    event_subscribers: HashMap<String, Vec<async_mpsc::Sender<Event>>>,
}

impl AsyncSubscriberRegistry {
    fn new() -> Self {
        Self {
            event_subscribers: HashMap::new(),
        }
    }

    fn add_subscriber(&mut self, event_type: String, sender: async_mpsc::Sender<Event>) {
        self.event_subscribers
            .entry(event_type)
            .or_default()
            .push(sender);
    }

    fn get_subscribers(&self, event_type: &str) -> Vec<&async_mpsc::Sender<Event>> {
        self.event_subscribers
            .get(event_type)
            .map(|senders| senders.iter().collect())
            .unwrap_or_default()
    }
}

/// Async message bus for event-driven communication
pub struct AsyncMessageBus {
    registry: Arc<tokio::sync::Mutex<AsyncSubscriberRegistry>>,
}

impl AsyncMessageBus {
    /// Create a new async message bus instance
    pub fn new() -> Self {
        Self {
            registry: Arc::new(tokio::sync::Mutex::new(AsyncSubscriberRegistry::new())),
        }
    }

    /// Subscribe to events of a specific type through unified Event enum
    pub async fn subscribe(&self, event_type: &str) -> AsyncConsumer<Event> {
        let (sender, receiver) = async_mpsc::channel(DEFAULT_CHANNEL_CAPACITY);

        let mut registry = self.registry.lock().await;
        registry.add_subscriber(event_type.to_string(), sender);

        AsyncConsumer::new(receiver)
    }

    /// Subscribe to all events
    pub async fn subscribe_all(&self) -> AsyncConsumer<Event> {
        let (sender, receiver) = async_mpsc::channel(DEFAULT_CHANNEL_CAPACITY);

        let mut registry = self.registry.lock().await;
        // Subscribe to all event types using the unified list
        let event_types = Event::all_event_types();

        for event_type in &event_types {
            registry.add_subscriber(event_type.to_string(), sender.clone());
        }

        AsyncConsumer::new(receiver)
    }

    /// Publish an event (convenience method for individual event types)
    pub async fn publish_field_value_set(&self, event: FieldValueSet) -> MessageBusResult<()> {
        self.publish_event(Event::FieldValueSet(event)).await
    }

    pub async fn publish_atom_created(&self, event: AtomCreated) -> MessageBusResult<()> {
        self.publish_event(Event::AtomCreated(event)).await
    }

    pub async fn publish_query_executed(&self, event: QueryExecuted) -> MessageBusResult<()> {
        self.publish_event(Event::QueryExecuted(event)).await
    }

    pub async fn publish_mutation_executed(&self, event: MutationExecuted) -> MessageBusResult<()> {
        self.publish_event(Event::MutationExecuted(event)).await
    }

    /// Convenience method to publish a unified Event
    pub async fn publish_event(&self, event: Event) -> MessageBusResult<()> {
        let registry = self.registry.lock().await;
        let event_type = event.event_type();
        let subscribers = registry.get_subscribers(event_type);

        if subscribers.is_empty() {
            return Ok(());
        }

        let mut failed_sends = 0;
        let total_subscribers = subscribers.len();

        for subscriber in subscribers {
            // Use try_send to avoid blocking and handle backpressure
            match subscriber.try_send(event.clone()) {
                Ok(_) => {}
                Err(async_mpsc::error::TrySendError::Full(_)) => {
                    warn!(
                        "⚠️ MessageBus channel full for subscriber of {}, dropping event",
                        event_type
                    );
                    failed_sends += 1;
                }
                Err(async_mpsc::error::TrySendError::Closed(_)) => {
                    // Subscriber closed, count as failed send
                    failed_sends += 1;
                }
            }
        }

        if failed_sends > 0 {
            // We log but don't error out entirely to avoid breaking the publisher flow
            // But if all failed, we might want to know
            if failed_sends == total_subscribers {
                return Err(MessageBusError::SendFailed {
                    reason: format!(
                        "All {} async subscribers failed to receive event (Full/Closed)",
                        total_subscribers
                    ),
                });
            }
        }

        Ok(())
    }

    /// Get the number of subscribers for a given event type
    pub async fn subscriber_count(&self, event_type: &str) -> usize {
        let registry = self.registry.lock().await;
        registry.get_subscribers(event_type).len()
    }
}

impl Default for AsyncMessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold_db_core::infrastructure::message_bus::request_events::MutationRequest;
    use crate::schema::types::Mutation;
    use tokio;

    #[tokio::test]
    async fn test_async_message_bus_creation() {
        let bus = AsyncMessageBus::new();
        assert_eq!(bus.subscriber_count("FieldValueSet").await, 0);
    }

    #[tokio::test]
    async fn test_async_subscribe() {
        let bus = AsyncMessageBus::new();
        let _consumer = bus.subscribe("FieldValueSet").await;
        assert_eq!(bus.subscriber_count("FieldValueSet").await, 1);
    }

    #[tokio::test]
    async fn test_async_publish_event() {
        let bus = AsyncMessageBus::new();
        let mut consumer = bus.subscribe("FieldValueSet").await;

        let event = FieldValueSet::new("test.field", serde_json::json!("value"), "source");
        bus.publish_field_value_set(event.clone()).await.unwrap();

        // Note: This test would need proper event extraction to work fully
        let received = consumer.recv().await;
        assert!(received.is_some());
    }

    #[tokio::test]
    async fn test_async_no_subscribers() {
        let bus = AsyncMessageBus::new();

        let event = AtomCreated::new("atom-123", serde_json::json!({}));
        let result = bus.publish_atom_created(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_async_subscribe_all() {
        let bus = AsyncMessageBus::new();
        let _consumer = bus.subscribe_all().await;

        // Should be subscribed to multiple event types
        assert!(bus.subscriber_count("FieldValueSet").await > 0);
        assert!(bus.subscriber_count("AtomCreated").await > 0);
        assert!(bus.subscriber_count("QueryExecuted").await > 0);
        // Verify new event types are engaged
        assert!(bus.subscriber_count("MutationRequest").await > 0);
    }

    #[tokio::test]
    async fn test_subscribe_all_receives_mutation_request() {
        let bus = AsyncMessageBus::new();
        let mut consumer = bus.subscribe_all().await;

        let mutation = Mutation::new(
            "test_schema".to_string(),
            std::collections::HashMap::new(),
            crate::schema::types::KeyValue::new(None, None),
            "pk_123".to_string(),
            0,
            crate::schema::types::operations::MutationType::Update,
        );
        let request = MutationRequest {
            correlation_id: "test_correlation".to_string(),
            mutation,
        };

        bus.publish_event(Event::MutationRequest(request))
            .await
            .unwrap();

        let received = consumer.recv().await;
        match received {
            Some(Event::MutationRequest(_)) => assert!(true),
            _ => panic!("Expected MutationRequest event"),
        }
    }

    #[tokio::test]
    async fn test_async_consumer_timeout() {
        let bus = AsyncMessageBus::new();
        let mut consumer = bus.subscribe("AtomCreated").await;

        let result = consumer.recv_timeout(AsyncDuration::from_millis(10)).await;
        assert!(matches!(result, Err(AsyncRecvError::Timeout)));
    }

    #[tokio::test]
    async fn test_async_consumer_try_recv() {
        let bus = AsyncMessageBus::new();
        let mut consumer = bus.subscribe("MutationExecuted").await;

        let result = consumer.try_recv();
        assert!(matches!(result, Err(AsyncTryRecvError::Empty)));
    }

    #[tokio::test]
    async fn test_backpressure_drop() {
        // Create a bus, subscribe with default capacity (1000)
        let bus = AsyncMessageBus::new();
        let _consumer = bus.subscribe("FieldValueSet").await;

        let event = FieldValueSet::new("test.field", serde_json::json!("value"), "source");

        // Fill the buffer
        for _ in 0..1000 {
            bus.publish_field_value_set(event.clone()).await.unwrap();
        }

        // The next one should fill the capacity (wait, channel(cap) means buffer size)
        // Actually, bounded channel of size N can hold N messages.
        // We sent 1000. Next one might be queued or potentially drop depending on try_send impl.
        // wait, try_send returns error if full.

        // Push one more, should result in log warning (we can't assert log easily)
        // but it should NOT error if we handle it gracefully (failed_sends > 0 but < total_subscribers if we have others, but here we have 1)
        // Wait, if failed_sends == total_subscribers, we return Err(SendFailed).
        // So with 1 subscriber, if it drops, it should error.

        let result = bus.publish_field_value_set(event.clone()).await;

        // It SHOULD error because we return Err if ALL subscribers failed.
        // With 1 subscriber full, it fails.
        // Wait, try_send returns full immediately.

        assert!(result.is_err());
        match result {
            Err(MessageBusError::SendFailed { reason }) => {
                assert!(reason.contains("All 1 async subscribers failed"));
            }
            _ => panic!("Expected SendFailed error due to backpressure"),
        }
    }
}
