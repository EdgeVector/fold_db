//! Field execution methods for different alignment types
//!
//! Provides execution methods for OneToOne, Broadcast, and Reduced field alignments
//! with comprehensive iteration and value processing capabilities.

pub mod types;
pub mod executor;
pub mod iteration;
pub mod reducers;

pub use types::*;
pub use iteration::*;
pub use reducers::*;
