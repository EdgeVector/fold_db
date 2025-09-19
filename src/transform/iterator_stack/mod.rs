//! # Schema Indexing Iterator Stack Model
//!
//! This module implements the iterator stack model for schema indexing that handles
//! fan-out using a stack of iterators (scopes). Each field expression is evaluated
//! within this stacked scope, with the field containing the deepest active iterator
//! determining the number of output rows.
//!
//! ## Architecture Overview
//!
//! The Iterator Stack provides a sophisticated execution model for complex data transformations
//! with nested iterations, fan-out operations, and multi-dimensional data processing. It serves
//! as the core execution engine for declarative transforms across different schema types.
//!
//! ### Key Principles
//!
//! 1. **Stack-Based Execution**: Nested iterations are managed as a stack of scopes
//! 2. **Depth-Determined Output**: The deepest iterator determines the output cardinality
//! 3. **Alignment Validation**: Fields must be properly aligned relative to the deepest iterator
//! 4. **Broadcast Semantics**: Values are broadcast across iterations when appropriate
//! 5. **Efficient Execution**: Deduplication and optimization prevent redundant computation
//!
//! ### Execution Flow
//!
//! ```text
//! Expression String -> Chain Parser -> Parsed Chain -> Field Alignment -> Execution Engine -> Final Result
//! ```
//!
//! ## Components
//!
//! * `chain_parser` - Parse chain syntax expressions like `blogpost.map().content.split_by_word().map()`
//!   - Converts expression strings into executable `ParsedChain` objects
//!   - Tracks iterator depths and branch structures
//!   - Validates chain syntax and structure
//!
//! * `stack` - Manage iterator depths and scope contexts
//!   - Maintains runtime stack of active iterator scopes
//!   - Tracks current depth and scope contexts
//!   - Manages iterator state and progression
//!
//! * `field_alignment` - Validate field alignment rules (1:1, broadcast, reduced)
//!   - Enforces alignment rules relative to the deepest iterator
//!   - Validates that all fields align properly
//!   - Optimizes alignment for performance
//!
//! * `execution_engine` - Runtime execution engine for broadcasting and emission
//!   - Coordinates execution of multiple field expressions
//!   - Manages broadcasting and emission of index entries
//!   - Handles deduplication and optimization
//!
//! * `types` - Core data structures and type definitions
//!   - `IteratorStack`: Main stack management structure
//!   - `ActiveScope`: Individual scope information
//!   - `IteratorType`: Different types of iterators
//!
//! * `errors` - Error types for iterator stack operations
//!   - Comprehensive error handling with context
//!   - Graceful degradation and recovery strategies
//!
//! ## Usage Examples
//!
//! ### Simple Field Access
//! ```rust
//! // Expression: "input.value"
//! // Result: Single value, no iteration
//! ```
//!
//! ### Schema Iteration
//! ```rust
//! // Expression: "blogpost.map()"
//! // Result: One row per blogpost
//! ```
//!
//! ### Nested Iteration
//! ```rust
//! // Expression: "blogpost.map().content.split_by_word().map()"
//! // Result: One row per word in each blogpost
//! ```
//!
//! ### Mixed Alignment
//! ```rust
//! // Field A: "blogpost.map()" (depth 1, 3 items)
//! // Field B: "blogpost.map().content.split_by_word().map()" (depth 3, 15 items)
//! // Result: Field A values broadcast to match Field B cardinality
//! ```
//!
//! ## Performance Features
//!
//! - **Expression Deduplication**: Identical expressions executed only once
//! - **Scope Caching**: Iterator states cached when possible
//! - **Lazy Evaluation**: Expressions evaluated only when needed
//! - **Memory Management**: Streaming, buffered, and in-memory iterator modes
//!
//! ## Integration
//!
//! The Iterator Stack integrates with the broader transform system:
//! - Used by `TransformExecutor` for complex field expressions
//! - Validated by field alignment before execution
//! - Results aggregated by the aggregation system
//! - Coordinated for multi-chain HashRange execution
//!
//! ## Documentation
//!
//! For detailed architecture documentation, see:
//! - [Iterator Stack Architecture](../../docs/design/iterator_stack_architecture.md)
//! - [Execution Flow Diagram](../../docs/design/iterator_stack_flow_diagram.md)

pub mod chain_parser;
pub mod errors;
pub mod execution_engine;
pub mod field_alignment;
pub mod stack;
pub mod types;

pub use chain_parser::*;
pub use errors::*;
pub use execution_engine::*;
pub use field_alignment::*;
pub use types::*;
