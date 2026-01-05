//! # Internal Message Bus for FoldDB Core
//!
//! Provides a foundational event-driven messaging system for migrating fold_db_core
//! to an event-driven architecture. This module implements a simple pub/sub message bus
//! using Rust channels for internal communication between components.
//!
//! ## Design Goals
//! - Enable loose coupling between database components
//! - Support both synchronous and asynchronous event handling
//! - Provide a foundation for eventual migration to full event-driven architecture
//! - Maintain high performance with minimal overhead
//!

//! ## Module Structure
//!
//! The message bus has been decomposed into focused modules:
//!
//! - [`events`] - All event type definitions and the unified Event enum
//! - [`error_handling`] - Error types, retry logic, and dead letter queue support
//! - [`sync_bus`] - Synchronous message bus implementation using std::sync::mpsc
//! - [`async_bus`] - Asynchronous message bus implementation using tokio::sync::mpsc
//! - [`enhanced_bus`] - Enhanced features like retry, dead letter queue, and event sourcing
//! - [`constructors`] - Convenience constructor methods for all event types
//! - [`tests`] - Comprehensive test suite for all components
//!
//! ## Main Components
//!

//! ### Asynchronous Message Bus
//!
//! The [`AsyncMessageBus`] provides async pub/sub messaging:
//!
//! ```rust
//! use datafold::fold_db_core::infrastructure::message_bus::{AsyncMessageBus, Event, atom_events::AtomCreated};
//! use serde_json::json;
//!
//! # async fn example() {
//! let bus = AsyncMessageBus::new();
//! let mut consumer = bus.subscribe("AtomCreated").await;
//!
//! let event = AtomCreated::new("atom-123", json!({"name": "Alice"}));
//! bus.publish_atom_created(event).await.unwrap();
//!
//! let received_event = consumer.recv().await;
//! # }
//! ```
//!
//! ### Async Message Bus
//!
//! The [`AsyncMessageBus`] provides advanced features:
//!
//! ```rust
//! use datafold::fold_db_core::infrastructure::message_bus::{AsyncMessageBus, atom_events::FieldValueSet, Event};
//! use serde_json::json;
//!
//! # async fn example() {
//! let bus = AsyncMessageBus::new();
//!
//! let event = FieldValueSet::new("user.status", json!("active"), "user_service");
//! let wrapped_event = Event::FieldValueSet(event);
//! bus.publish_event(wrapped_event).await.unwrap();
//!
//! // Subscribe to events
//! let mut consumer = bus.subscribe("FieldValueSet").await;
//!
//! // Check for new events
//! let _received = consumer.try_recv();
//! # }
//! ```

// Re-export all public types and event modules
pub use async_bus::{AsyncConsumer, AsyncEventHandler, AsyncMessageBus};
pub use error_handling::{
    AsyncRecvError, AsyncTryRecvError, DeadLetterEvent, EventHistoryEntry, MessageBusError,
    MessageBusResult, RetryableEvent,
};
pub use events::{atom_events, query_events, request_events, schema_events, Event, EventType};

// Import constructor implementations (these add methods to the event types)

// Internal modules
mod async_bus;
mod constructors;
mod error_handling;
pub mod events;
