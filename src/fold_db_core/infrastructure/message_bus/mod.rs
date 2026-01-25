pub mod async_bus;
pub mod constructors;
pub mod error_handling;
pub mod events;

pub use async_bus::AsyncMessageBus;
pub use error_handling::{MessageBusError, MessageBusResult};
pub use events::{atom_events, query_events, request_events, schema_events, Event, EventEnvelope};

use crate::error::FoldDbResult;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// A stream of messages from a subscription
pub type MessageStream = Pin<Box<dyn Stream<Item = FoldDbResult<Vec<u8>>> + Send>>;

/// Abstract interface for a message bus (pub/sub system)
#[async_trait]
pub trait MessageBus: Send + Sync {
    /// Publish a message to a topic
    async fn publish(&self, topic: &str, message: &[u8]) -> FoldDbResult<()>;

    /// Subscribe to a topic, returning a stream of messages
    async fn subscribe(&self, topic: &str) -> FoldDbResult<MessageStream>;
}
