//! Field execution methods for different alignment types
//!
//! Provides execution methods for OneToOne, Broadcast, and Reduced field alignments
//! with comprehensive iteration and value processing capabilities.

pub mod executor;
pub mod iteration;
pub mod reducers;
pub mod types;

pub use iteration::*;
pub use reducers::*;
pub use types::*;
