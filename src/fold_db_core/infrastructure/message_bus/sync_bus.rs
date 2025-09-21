//! Synchronous message bus implementation
//!
//! This module provides the synchronous message bus that uses std::sync::mpsc
//! for communication between components.

use super::error_handling::MessageBusResult;
use super::events::{Event, EventType};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// Consumer handle for receiving events of a specific type
pub struct Consumer<T: EventType> {
    receiver: Receiver<T>,
}

impl<T: EventType> Consumer<T> {
    /// Try to receive an event without blocking
    pub fn try_recv(&mut self) -> Result<T, mpsc::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Receive an event, blocking until one is available
    pub fn recv(&mut self) -> Result<T, mpsc::RecvError> {
        self.receiver.recv()
    }

    /// Get an iterator over received events
    pub fn iter(&mut self) -> mpsc::Iter<'_, T> {
        self.receiver.iter()
    }

    /// Try to receive an event with a timeout
    pub fn recv_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<T, mpsc::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }
}

/// Internal registry for managing event subscribers
struct SubscriberRegistry {
    // Using type erasure to store different channel senders
    // Key: event type name, Value: list of boxed senders
    subscribers: HashMap<String, Vec<Box<dyn std::any::Any + Send>>>,
}

impl SubscriberRegistry {
    fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
        }
    }

    fn add_subscriber<T: EventType>(&mut self, sender: Sender<T>) {
        let type_id = T::type_id();
        let boxed_sender = Box::new(sender);

        self.subscribers
            .entry(type_id.to_string())
            .or_default()
            .push(boxed_sender);
    }

    fn get_subscribers<T: EventType>(&self) -> Vec<&Sender<T>> {
        let type_id = T::type_id();
        self.subscribers
            .get(type_id)
            .map(|senders| {
                senders
                    .iter()
                    .filter_map(|boxed| boxed.downcast_ref::<Sender<T>>())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Main synchronous message bus for event-driven communication
pub struct MessageBus {
    registry: Arc<Mutex<SubscriberRegistry>>,
}

impl MessageBus {
    /// Create a new message bus instance
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Mutex::new(SubscriberRegistry::new())),
        }
    }

    /// Subscribe to events of a specific type
    /// Returns a Consumer that can be used to receive events
    pub fn subscribe<T: EventType>(&self) -> Consumer<T> {
        let (sender, receiver) = mpsc::channel();

        let mut registry = self.registry.lock().unwrap();
        registry.add_subscriber(sender);

        Consumer { receiver }
    }

    /// Publish an event to all subscribers of that event type
    pub fn publish<T: EventType>(&self, event: T) -> MessageBusResult<()> {
        let mut registry = self.registry.lock().unwrap();
        let type_id = T::type_id().to_string();

        let Some(senders) = registry.subscribers.get_mut(&type_id) else {
            // No subscribers for this event type - this is not an error
            return Ok(());
        };

        let mut failed_sends = 0;

        senders.retain(|boxed_sender| {
            if let Some(sender) = boxed_sender.downcast_ref::<Sender<T>>() {
                match sender.send(event.clone()) {
                    Ok(_) => true,
                    Err(_) => {
                        failed_sends += 1;
                        false
                    }
                }
            } else {
                failed_sends += 1;
                false
            }
        });

        if senders.is_empty() {
            registry.subscribers.remove(&type_id);
        }

        if failed_sends > 0 {
            // Treat dropped subscribers as a recoverable condition instead of an error.
            return Ok(());
        }

        Ok(())
    }

    /// Convenience method to publish a unified Event
    pub fn publish_event(&self, event: Event) -> MessageBusResult<()> {
        match event {
            Event::FieldValueSet(e) => self.publish(e),
            Event::AtomCreated(e) => self.publish(e),
            Event::AtomUpdated(e) => self.publish(e),
            Event::MoleculeCreated(e) => self.publish(e),
            Event::MoleculeUpdated(e) => self.publish(e),
            Event::SchemaLoaded(e) => self.publish(e),
            Event::TransformExecuted(e) => self.publish(e),
            Event::SchemaChanged(e) => self.publish(e),
            Event::TransformTriggered(e) => self.publish(e),
            Event::QueryExecuted(e) => self.publish(e),
            Event::MutationExecuted(e) => self.publish(e),
            Event::AtomCreateRequest(e) => self.publish(e),
            Event::AtomCreateResponse(e) => self.publish(e),
            Event::AtomUpdateRequest(e) => self.publish(e),
            Event::AtomUpdateResponse(e) => self.publish(e),
            Event::MoleculeCreateRequest(e) => self.publish(e),
            Event::MoleculeCreateResponse(e) => self.publish(e),
            Event::MoleculeUpdateRequest(e) => self.publish(e),
            Event::MoleculeUpdateResponse(e) => self.publish(e),
            Event::MoleculeQueryRequest(e) => self.publish(e),
            Event::MoleculeQueryResponse(e) => self.publish(e),
            Event::MoleculeGetRequest(e) => self.publish(e),
            Event::MoleculeGetResponse(e) => self.publish(e),
            Event::FieldValueSetRequest(e) => self.publish(e),
            Event::FieldValueSetResponse(e) => self.publish(e),
            Event::FieldUpdateRequest(e) => self.publish(e),
            Event::FieldUpdateResponse(e) => self.publish(e),
            Event::SchemaLoadRequest(e) => self.publish(e),
            Event::SchemaLoadResponse(e) => self.publish(e),
            Event::SchemaApprovalRequest(e) => self.publish(e),
            Event::SchemaApprovalResponse(e) => self.publish(e),
            Event::AtomHistoryRequest(e) => self.publish(e),
            Event::AtomHistoryResponse(e) => self.publish(e),
            Event::AtomGetRequest(e) => self.publish(e),
            Event::AtomGetResponse(e) => self.publish(e),
            Event::FieldValueQueryRequest(e) => self.publish(e),
            Event::FieldValueQueryResponse(e) => self.publish(e),
            Event::SchemaStatusRequest(e) => self.publish(e),
            Event::SchemaStatusResponse(e) => self.publish(e),
            Event::SchemaDiscoveryRequest(e) => self.publish(e),
            Event::SchemaDiscoveryResponse(e) => self.publish(e),
            Event::SystemInitializationRequest(e) => self.publish(e),
            Event::SystemInitializationResponse(e) => self.publish(e),
        }
    }

    /// Get the number of subscribers for a given event type
    pub fn subscriber_count<T: EventType>(&self) -> usize {
        let registry = self.registry.lock().unwrap();
        registry.get_subscribers::<T>().len()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold_db_core::infrastructure::message_bus::{
        atom_events::{AtomCreated, FieldValueSet},
        query_events::{MutationExecuted, QueryExecuted},
        schema_events::{SchemaChanged, TransformTriggered},
    };
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn test_field_value_set_event() {
        let event = FieldValueSet::new("user.name", json!("Alice"), "test_source");
        assert_eq!(event.field, "user.name");
        assert_eq!(event.value, json!("Alice"));
        assert_eq!(event.source, "test_source");
    }

    #[test]
    fn test_atom_created_event() {
        let event = AtomCreated::new("atom-123", json!({"key": "value"}));
        assert_eq!(event.atom_id, "atom-123");
        assert_eq!(event.data, json!({"key": "value"}));
    }

    #[test]
    fn test_message_bus_basic_pubsub() {
        let bus = MessageBus::new();
        let mut consumer = bus.subscribe::<FieldValueSet>();

        // Verify no events initially
        assert!(consumer.try_recv().is_err());

        // Publish an event
        let event = FieldValueSet::new("test.field", json!("test_value"), "test");
        bus.publish(event.clone()).unwrap();

        // Consumer should receive the event
        let received = consumer.try_recv().unwrap();
        assert_eq!(received, event);
    }

    #[test]
    fn test_multiple_consumers_same_event_type() {
        let bus = MessageBus::new();
        let mut consumer1 = bus.subscribe::<AtomCreated>();
        let mut consumer2 = bus.subscribe::<AtomCreated>();

        assert_eq!(bus.subscriber_count::<AtomCreated>(), 2);

        let event = AtomCreated::new("atom-456", json!({"data": "test"}));
        bus.publish(event.clone()).unwrap();

        // Both consumers should receive the event
        assert_eq!(consumer1.try_recv().unwrap(), event);
        assert_eq!(consumer2.try_recv().unwrap(), event);
    }

    #[test]
    fn test_different_event_types() {
        let bus = MessageBus::new();
        let mut field_consumer = bus.subscribe::<FieldValueSet>();
        let mut atom_consumer = bus.subscribe::<AtomCreated>();

        // Publish different event types
        let field_event = FieldValueSet::new("test.field", json!("value"), "source");
        let atom_event = AtomCreated::new("atom-789", json!({}));

        bus.publish(field_event.clone()).unwrap();
        bus.publish(atom_event.clone()).unwrap();

        // Each consumer should only receive their event type
        assert_eq!(field_consumer.try_recv().unwrap(), field_event);
        assert!(field_consumer.try_recv().is_err()); // No more events

        assert_eq!(atom_consumer.try_recv().unwrap(), atom_event);
        assert!(atom_consumer.try_recv().is_err()); // No more events
    }

    #[test]
    fn test_publish_event_unified() {
        let bus = MessageBus::new();
        let mut consumer = bus.subscribe::<SchemaChanged>();

        let schema_event = SchemaChanged::new("test_schema");
        let unified_event = Event::SchemaChanged(schema_event.clone());

        bus.publish_event(unified_event).unwrap();

        assert_eq!(consumer.try_recv().unwrap(), schema_event);
    }

    #[test]
    fn test_no_subscribers() {
        let bus = MessageBus::new();

        // Publishing to no subscribers should not fail
        let event = TransformTriggered::new("transform-123");
        bus.publish(event).unwrap();
    }

    #[test]
    fn test_event_type_ids() {
        assert_eq!(FieldValueSet::type_id(), "FieldValueSet");
        assert_eq!(AtomCreated::type_id(), "AtomCreated");
        assert_eq!(SchemaChanged::type_id(), "SchemaChanged");
        assert_eq!(TransformTriggered::type_id(), "TransformTriggered");
        assert_eq!(QueryExecuted::type_id(), "QueryExecuted");
        assert_eq!(MutationExecuted::type_id(), "MutationExecuted");
    }

    #[test]
    fn test_event_serialization() {
        let event = FieldValueSet::new("test", json!("value"), "source");
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: FieldValueSet = serde_json::from_str(&serialized).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_consumer_recv_timeout() {
        let bus = MessageBus::new();
        let mut consumer = bus.subscribe::<AtomCreated>();

        // Should timeout since no events are published
        let result = consumer.recv_timeout(Duration::from_millis(10));
        assert!(matches!(result, Err(mpsc::RecvTimeoutError::Timeout)));
    }

    #[test]
    fn test_query_executed_event() {
        let bus = MessageBus::new();
        let mut consumer = bus.subscribe::<QueryExecuted>();

        let event = QueryExecuted::new("range_query", "Product", 250, 10);
        bus.publish(event.clone()).unwrap();

        assert_eq!(consumer.try_recv().unwrap(), event);
        assert_eq!(event.query_type, "range_query");
        assert_eq!(event.schema, "Product");
        assert_eq!(event.execution_time_ms, 250);
        assert_eq!(event.result_count, 10);
    }

    #[test]
    fn test_mutation_executed_event() {
        let bus = MessageBus::new();
        let mut consumer = bus.subscribe::<MutationExecuted>();

        let event = MutationExecuted::new("update", "User", 125, 2);
        bus.publish(event.clone()).unwrap();

        assert_eq!(consumer.try_recv().unwrap(), event);
        assert_eq!(event.operation, "update");
        assert_eq!(event.schema, "User");
        assert_eq!(event.execution_time_ms, 125);
        assert_eq!(event.fields_affected, 2);
    }
}
