//! FoldDB Core - Event-driven database system
//!
//! This module contains the core components of the FoldDB system organized
//! into logical groups for better maintainability and understanding:
//!
//! - **managers/**: Core managers for different aspects of data management
//! - **services/**: Service layer components for operations
//! - **orchestration/**: Coordination and orchestration components

// Organized module declarations
pub mod event_monitor;
pub mod event_statistics;
pub mod factory;
pub mod fold_db;
pub mod orchestration;
pub mod pending_task_tracker;
pub mod process_results_subscriber;
pub mod query;

// Core components

pub mod mutation_manager;
pub mod sync_coordinator;
pub mod view_orchestrator;

// Re-export key components
pub use event_monitor::EventMonitor;
pub use query::QueryExecutor;

// Re-export core components

pub use mutation_manager::MutationManager;
pub use sync_coordinator::SyncCoordinator;

// Re-export the main FoldDB struct
pub use fold_db::FoldDB;
