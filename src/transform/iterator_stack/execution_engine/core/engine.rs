//! Main execution engine implementation
//!
//! Contains the core ExecutionEngine struct and its main execution methods
//! for processing field expressions and coordinating execution.

use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::errors::{IteratorStackError, IteratorStackResult};
use crate::transform::iterator_stack::types::IteratorStack;
use std::collections::HashMap;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::field::FieldValue;

use super::scope_creation::ScopeCreationHelper;
use super::types::{ExecutionContext, ExecutionResult};
use crate::transform::iterator_stack::execution_engine::field_execution::{
    DefaultFieldExecutor, FieldExecutionResult,
};
use crate::transform::iterator_stack::execution_engine::iterator_management::{
    IteratorDatasetCache, IteratorManager,
};

/// Runtime execution engine for iterator stack operations
pub struct ExecutionEngine {
    /// Manager for iterator stack operations
    iterator_manager: IteratorManager,
    /// Field executor for different alignment types
    field_executor: DefaultFieldExecutor,
}

impl ExecutionEngine {
    /// Creates a new execution engine
    pub fn new() -> Self {
        Self {
            iterator_manager: IteratorManager::new(),
            field_executor: DefaultFieldExecutor::new(),
        }
    }

    /// Executes multiple field expressions and returns combined results
    pub fn execute_fields(
        &mut self,
        chains: HashMap<String, ParsedChain>,
        input_data: HashMap<String, HashMap<KeyValue, FieldValue>>,
    ) -> IteratorStackResult<ExecutionResult> {
        // Determine the maximum depth that needs to be emitted across all chains so that
        // deeper iterators (e.g., nested map operations) are actually traversed during
        // execution. Previously this was hardcoded to `0`, which prevented the engine
        // from iterating past the root scope and caused expressions like
        // `BlogPost.map().content.split_by_word().map()` to emit only the first word.
        let max_emission_depth = chains.values().map(|chain| chain.depth).max().unwrap_or(0);

        // Create execution context using the calculated emission depth so every chain can
        // iterate to its required depth.
        let context = ExecutionContext {
            input_data,
            emission_depth: max_emission_depth,
            variables: HashMap::new(),
        };

        let mut index_entries = HashMap::new();
        let mut dataset_cache = IteratorDatasetCache::new();
        let mut warnings = HashMap::new();

        // Execute each unique expression only once
        for (field, chain) in chains.iter() {
            let field_result = self.execute_single_field(chain, &context, &mut dataset_cache, field.clone())?;
            index_entries.insert(field.clone(), field_result.entries);
            warnings.insert(field.clone(), field_result.warnings);
        }

        Ok(ExecutionResult {
            index_entries,
            warnings,
        })
    }

    /// Executes a single field expression
    fn execute_single_field(
        &mut self,
        chain: &ParsedChain,
        context: &ExecutionContext,
        field: &str,
        cache: &mut IteratorDatasetCache,
    ) -> IteratorStackResult<FieldExecutionResult> {
        // Create iterator stack from the chain
        let mut stack = IteratorStack::from_chain(chain)?;

        // If the stack is empty (no scopes), create default scopes based on the chain operations
        if stack.is_empty() {
            ScopeCreationHelper::create_default_scopes(&mut stack, chain, context.input_data.clone(), field.clone())?;
        }

        // Initialize the iterator stack with input data
        self.iterator_manager
            .initialize_stack(&mut stack, context.input_data.get(field).unwrap().clone(), cache)?;

        // Execute field expression using unified approach
        let result = self
            .field_executor
            .execute_field(&mut stack, chain, context, field.clone())?;
        Ok(result)
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}
