//! FoldDB Core - Event-driven database system
//!
//! This module contains the core components of the FoldDB system organized
//! into logical groups for better maintainability and understanding:
//!
//! - **managers/**: Core managers for different aspects of data management
//! - **services/**: Service layer components for operations
//! - **infrastructure/**: Foundation components (message bus, initialization, etc.)
//! - **orchestration/**: Coordination and orchestration components
//! - **shared/**: Common utilities and shared components
//! - **transform_manager/**: Transform system (already well-organized)

// Organized module declarations
pub mod factory;
pub mod fold_db;
pub mod infrastructure;
pub mod orchestration;
pub mod query;
pub mod shared;

// Core components
pub mod mutation_completion_handler;
pub mod mutation_manager;

// Re-export key components for backwards compatibility
pub use infrastructure::{EventMonitor, MessageBus};
pub use orchestration::TransformOrchestrator;
pub use query::QueryExecutor;
pub use shared::*;

// Re-export core components
pub use mutation_completion_handler::{
    MutationCompletionDiagnostics, MutationCompletionError, MutationCompletionHandler,
    MutationCompletionResult, DEFAULT_COMPLETION_TIMEOUT,
};
pub use mutation_manager::MutationManager;

// Re-export the main FoldDB struct
pub use fold_db::FoldDB;
