//! # Transform System
//!
//! The transform module implements a domain-specific language (DSL) for writing
//! secure, auditable transformations in the Datafold platform.
//!
//! ## Components
//!
//! * `ast` - Abstract Syntax Tree definitions for the transform DSL
//! * `parser` - Parser for the transform DSL
//! * `interpreter` - Interpreter for executing transforms
//! * `executor` - High-level executor for applying transforms to field values
//! * `validation` - Validation utilities for transform execution
//! * `coordination` - Multi-chain coordination for complex schemas
//! * `iterator_stack` - Sophisticated execution model for complex nested iterations and fan-out operations
//! * `executor` - Unified executor for all schema types (Single, Range, HashRange)
//! * `restricted_access` - Enforces mutation-only data persistence
//!
//! ## Architecture
//!
//! Transforms in Datafold define how data from source fields is processed to produce
//! derived values. The transform system consists of:
//!
//! 1. A parser that converts transform DSL code into an AST
//! 2. An interpreter that executes the AST to produce a result
//! 3. An executor that handles the integration with the schema system
//! 4. **Restricted access patterns that enforce mutation-only data persistence**
//!
//! ## Data Persistence Restrictions
//!
//! **CRITICAL**: Transforms cannot directly create atoms or molecules. All data
//! persistence must go through the mutation system to ensure:
//!
//! - Proper audit trails
//! - Event coordination
//! - Data integrity
//! - Security compliance
//!
//! Use the `TransformDataPersistence` trait and `MutationBasedPersistence`
//! implementation for all data persistence needs.
//!
//! ## Documentation
//!
//! For detailed architecture documentation, see:
//! - [Iterator Stack Architecture](../../docs/design/iterator_stack_architecture.md)
//! - [Iterator Stack Quick Reference](../../docs/design/iterator_stack_quick_reference.md)
//! - [Iterator Stack Flow Diagram](../../docs/design/iterator_stack_flow_diagram.md)

// Execution coordination components
pub mod iterator_stack;
pub mod iterator_stack_typed;
pub mod result_types;
pub mod shared_utilities;

// Public re-exports
pub use crate::schema::types::Transform;


// New modular components
pub mod manager;