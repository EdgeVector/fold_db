//! FoldDB Core - Event-driven database system
//!
//! This module contains the core components of the FoldDB system organized
//! into logical groups for better maintainability and understanding:
//!
//! - **managers/**: Core managers for different aspects of data management
//! - **services/**: Service layer components for operations
//! - **infrastructure/**: Foundation components (message bus, initialization, etc.)
//! - **orchestration/**: Coordination and orchestration components

// Organized module declarations
pub mod factory;
pub mod fold_db;
pub mod infrastructure;
pub mod orchestration;
pub mod query;

// Core components

pub mod mutation_manager;
pub mod sync_coordinator;
pub mod view_invalidation;

// Re-export key components
pub use infrastructure::EventMonitor;
pub use query::QueryExecutor;

// Re-export core components

pub use mutation_manager::MutationManager;
pub use sync_coordinator::SyncCoordinator;

// Re-export the main FoldDB struct
pub use fold_db::FoldDB;
