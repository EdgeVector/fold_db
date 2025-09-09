//! Main execution engine implementation
//!
//! Contains the core ExecutionEngine struct and its main execution methods
//! for processing field expressions and coordinating execution.

use crate::transform::iterator_stack::chain_parser::{FieldAlignment, ParsedChain};
use crate::transform::iterator_stack::field_alignment::AlignmentValidationResult;
use crate::transform::iterator_stack::types::IteratorStack;
use crate::transform::iterator_stack::errors::{IteratorStackError, IteratorStackResult};
use serde_json::Value;
use std::collections::HashMap;
use log::debug;

use super::types::{ExecutionContext, ExecutionResult, ExecutionStatistics};
use super::scope_creation::ScopeCreationHelper;
use super::statistics::StatisticsHelper;
use crate::transform::iterator_stack::execution_engine::field_execution::{FieldExecutionResult, DefaultFieldExecutor, FieldExecutor};
use crate::transform::iterator_stack::execution_engine::iterator_management::IteratorManager;

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

    /// Creates a new execution engine with default configuration
    #[allow(dead_code)]
    fn default() -> Self {
        Self::new()
    }

    /// Executes multiple field expressions and returns combined results
    pub fn execute_fields(
        &mut self,
        chains: &[ParsedChain],
        alignment_result: &AlignmentValidationResult,
        input_data: Value,
    ) -> IteratorStackResult<ExecutionResult> {
        debug!("Executing {} field expressions", chains.len());

        // Create execution context
        let context = ExecutionContext {
            input_data: input_data.clone(),
            field_alignments: alignment_result.field_alignments.clone(),
            emission_depth: alignment_result.max_depth,
            variables: HashMap::new(),
        };

        let mut index_entries = Vec::new();
        let mut warnings = Vec::new();
        let cache_hits = 0;
        let cache_misses = 0;

        // Group chains by expression to avoid duplicate execution
        let mut expression_groups: HashMap<String, Vec<&ParsedChain>> = HashMap::new();
        for chain in chains {
            expression_groups.entry(chain.expression.clone()).or_default().push(chain);
        }

        // Execute each unique expression only once
        for (expression, chain_group) in expression_groups.iter() {
            debug!("Executing unique expression: {} (used by {} fields)", expression, chain_group.len());
            
            // Use the first chain as the representative for execution
            let representative_chain = chain_group[0];
            let field_result = self.execute_single_field(representative_chain, &context)?;
            debug!("Expression '{}' produced {} entries", expression, field_result.entries.len());

            // Add the results once (not duplicated for each field)
            index_entries.extend(field_result.entries);
            warnings.extend(field_result.warnings);
        }

        // Generate execution statistics
        let statistics = ExecutionStatistics {
            total_entries: index_entries.len(),
            items_per_depth: StatisticsHelper::calculate_items_per_depth(&index_entries),
            memory_usage_bytes: StatisticsHelper::estimate_memory_usage(&index_entries),
            cache_hits,
            cache_misses,
        };

        Ok(ExecutionResult {
            index_entries,
            statistics,
            warnings,
        })
    }

    /// Executes a single field expression
    fn execute_single_field(
        &mut self,
        chain: &ParsedChain,
        context: &ExecutionContext,
    ) -> IteratorStackResult<FieldExecutionResult> {
        debug!("Executing single field: {}", chain.expression);

        // Get alignment information for this field
        let alignment_info = context.field_alignments.get(&chain.expression)
            .ok_or_else(|| IteratorStackError::ExecutionError {
                message: format!("No alignment information found for field: {}", chain.expression)
            })?;

        // Create iterator stack from the chain
        let mut stack = IteratorStack::from_chain(chain)?;

        // If the stack is empty (no scopes), create default scopes based on the chain operations
        if stack.is_empty() {
            debug!("Stack is empty, creating default scopes for chain: {}", chain.expression);
            ScopeCreationHelper::create_default_scopes(&mut stack, chain, &context.input_data)?;
        }

        // Initialize the iterator stack with input data
        self.iterator_manager.initialize_stack(&mut stack, &context.input_data)?;

        // Execute based on alignment type
        debug!("Field {} has alignment: {:?}", chain.expression, alignment_info.alignment);
        match alignment_info.alignment {
            FieldAlignment::OneToOne => {
                debug!("Executing OneToOne for {}", chain.expression);
                let result = self.field_executor.execute_one_to_one(&mut stack, chain, context)?;
                debug!("OneToOne produced {} entries", result.entries.len());
                Ok(result)
            }
            FieldAlignment::Broadcast => {
                debug!("Executing Broadcast for {}", chain.expression);
                let result = self.field_executor.execute_broadcast(&mut stack, chain, context)?;
                debug!("Broadcast produced {} entries", result.entries.len());
                Ok(result)
            }
            FieldAlignment::Reduced => {
                debug!("Executing Reduced for {}", chain.expression);
                let result = self.field_executor.execute_reduced(&mut stack, chain, context)?;
                debug!("Reduced produced {} entries", result.entries.len());
                Ok(result)
            }
        }
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}
