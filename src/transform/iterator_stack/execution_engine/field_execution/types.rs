//! Type definitions for field execution
//!
//! Contains all data structures, traits, and result types used in field execution
//! and alignment processing.

use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::errors::IteratorStackResult;
use crate::transform::iterator_stack::execution_engine::core::{
    ExecutionContext, ExecutionWarning, IndexEntry,
};
use crate::transform::iterator_stack::execution_engine::field_evaluation::DefaultFieldEvaluator;
use crate::transform::iterator_stack::types::IteratorStack;

/// Result of executing a single field expression
#[derive(Debug, Clone, PartialEq)]
pub struct FieldExecutionResult {
    /// Generated index entries
    pub entries: Vec<IndexEntry>,
    /// Any warnings generated during execution
    pub warnings: Vec<ExecutionWarning>,
}

impl FieldExecutionResult {
    /// Creates a new empty field execution result
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

impl Default for FieldExecutionResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Field execution methods
pub trait FieldExecutor {
    /// Executes OneToOne alignment
    fn execute_one_to_one(
        &mut self,
        stack: &mut IteratorStack,
        chain: &ParsedChain,
        context: &ExecutionContext,
    ) -> IteratorStackResult<FieldExecutionResult>;

    /// Executes Broadcast alignment
    fn execute_broadcast(
        &mut self,
        stack: &mut IteratorStack,
        chain: &ParsedChain,
        context: &ExecutionContext,
    ) -> IteratorStackResult<FieldExecutionResult>;

    /// Executes Reduced alignment
    fn execute_reduced(
        &mut self,
        stack: &mut IteratorStack,
        chain: &ParsedChain,
        context: &ExecutionContext,
    ) -> IteratorStackResult<FieldExecutionResult>;
}

/// Default implementation of field execution methods
pub struct DefaultFieldExecutor {
    /// Field evaluator for processing field expressions
    pub field_evaluator: DefaultFieldEvaluator,
}

impl DefaultFieldExecutor {
    /// Creates a new default field executor
    pub fn new() -> Self {
        Self {
            field_evaluator: DefaultFieldEvaluator,
        }
    }
}

impl Default for DefaultFieldExecutor {
    fn default() -> Self {
        Self::new()
    }
}
