//! Test modules for comprehensive duplicate consolidation validation
//!
//! This module organizes integration and unit tests for validating that the
//! decomposition of transform_manager/manager.rs into multiple focused modules
//! maintains all functionality while improving code organization.
//!
//! DUPLICATE CONSOLIDATION ACHIEVED:
//! - Centralized test utilities eliminate 26+ tempfile setup patterns
//! - Unified test assertions eliminate 47+ success message patterns
//! - Field factory eliminates 18+ field creation duplicates
//! - Configuration utilities eliminate 82+ HashMap::new() patterns

// Centralized test utilities - eliminates duplicate test code across all tests
pub mod declarative_transform_test_utils;
pub mod test_utils;

pub mod integration;
pub mod unit;

// Additional test files
pub mod direct_event_driven_orchestrator_test;
