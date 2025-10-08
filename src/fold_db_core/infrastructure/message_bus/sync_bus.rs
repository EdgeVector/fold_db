//! Synchronous message bus implementation
//!
//! This module provides the synchronous message bus that uses std::sync::mpsc
//! for communication between components.

use super::error_handling::MessageBusResult;
use super::events::EventType;
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
                let cloned_event = event.clone();
                match sender.send(cloned_event) {
                    Ok(_) => {
                        true
                    }
                    Err(e) => {
                        log::error!("❌ MessageBus send failed for {}: {:?}", type_id, e);
                        failed_sends += 1;
                        false
                    }
                }
            } else {
                log::error!("❌ MessageBus downcast failed for {}", type_id);
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