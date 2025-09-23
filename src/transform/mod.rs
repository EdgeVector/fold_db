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
//! * `aggregation` - Result aggregation for different schema types
//! * `hash_range_executor` - HashRange schema executor
//! * `range_executor` - Range schema executor
//! * `single_executor` - Single schema executor
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

pub mod ast;
pub mod executor;
pub mod expression_evaluator;
pub mod function_registry;
pub mod interpreter;
pub mod mutation_examples;
pub mod native_executor;
pub mod parser;
pub mod restricted_access;
pub mod restricted_access_example;
pub mod restricted_access_integration_test;
pub mod safe_access;
pub mod standardized_executor;

// New modular components
pub mod aggregation;
pub mod coordination;
pub mod hash_range_executor;
pub mod iterator_stack;
pub mod native;
pub mod native_schema_registry;
pub mod range_executor;
pub mod shared_utilities;
pub mod single_executor;
pub mod validation;

// Public re-exports
pub use crate::schema::types::Transform;
pub use ast::{Expression, Operator, TransformDeclaration, UnaryOperator, Value};
pub use executor::TransformExecutor;
pub use expression_evaluator::{ExpressionEvaluator, ExpressionEvaluationError};
pub use function_registry::{FunctionRegistry, FunctionRegistryError};
pub use interpreter::Interpreter;
pub use native_executor::NativeTransformExecutor;
pub use mutation_examples::{
    BatchMutationExecutor, ConditionalMutationExecutor, MutationBasedDataStorage,
    TransformWithMutationStorage,
};
pub use native::{
    FieldDefinition as NativeFieldDefinition, FieldDefinitionError as NativeFieldDefinitionError,
    FieldMapping as NativeFieldMapping, FieldType as NativeFieldType, FieldValue,
    FilterCondition as NativeFilterCondition, FilterTransform as NativeFilterTransform,
    MapTransform as NativeMapTransform, ReduceTransform as NativeReduceTransform,
    ReducerType as NativeReducerType, TransformSpec as NativeTransformSpec,
    TransformSpecError as NativeTransformSpecError, TransformType as NativeTransformType,
};
pub use parser::TransformParser;
pub use restricted_access::{
    MutationBasedPersistence, TransformAccessError, TransformAccessValidator,
    TransformDataPersistence,
};
pub use safe_access::{
    DatabaseTransformDataAccess, ReadOnlyAtom, ReadOnlyMolecule, ReadOnlyMoleculeRange,
    TransformSafeDataAccess,
};
pub use standardized_executor::{
    DatabaseInputProvider, EventDrivenInputProvider, ExecutionMetadata, InputProvider,
    MutationExecutor, MutationServiceExecutor, OrchestratedTransformExecutor,
    StandardizedExecutionResult, StandardizedTransformExecutor,
};
