//! Core field execution logic
//!
//! Contains the main execution algorithms for different alignment types:
//! OneToOne, Broadcast, and Reduced field execution.

use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::errors::IteratorStackResult;
use crate::transform::iterator_stack::types::IteratorStack;
use log::debug;
use serde_json::Value;

use crate::transform::iterator_stack::execution_engine::core::{
    ExecutionContext, ExecutionWarning, ExecutionWarningType, IndexEntry,
};
use crate::transform::iterator_stack::execution_engine::field_evaluation::FieldEvaluator;
use crate::transform::iterator_stack::execution_engine::field_execution::iteration::IterationHelper;
use crate::transform::iterator_stack::execution_engine::field_execution::reducers::ReducerHelper;
use crate::transform::iterator_stack::execution_engine::field_execution::types::{
    DefaultFieldExecutor, FieldExecutionResult,
};

/// Default field executor implementation
/// This executor is used to execute field expressions without alignment types
impl DefaultFieldExecutor {
    /// Executes field expression - unified approach without alignment types
    pub fn execute_field(
        &mut self,
        stack: &mut IteratorStack,
        chain: &ParsedChain,
        context: &ExecutionContext,
    ) -> IteratorStackResult<FieldExecutionResult> {
        let mut entries = Vec::new();
        let mut warnings = Vec::new();

        debug!(
            "🚀 execute_field starting for chain: {} at emission_depth: {}",
            chain.expression, context.emission_depth
        );
        debug!("📊 Stack has {} scopes", stack.len());

        // Check if we have any iterators that can actually iterate
        let can_iterate = self.check_iteration_capability(stack);
        debug!("Stack can iterate: {}", can_iterate);

        if !can_iterate {
            self.execute_direct_evaluation(stack, chain, context, &mut entries)?;
        } else {
            self.execute_iterative_evaluation(stack, chain, context, &mut entries)?;
        }

        self.check_performance_warnings(&mut warnings, &entries, chain);
        
        debug!(
            "execute_field completed, produced {} entries with {} warnings",
            entries.len(),
            warnings.len()
        );

        Ok(FieldExecutionResult { entries, warnings })
    }

    /// Checks if the stack has any iterators that can actually iterate
    fn check_iteration_capability(&self, stack: &IteratorStack) -> bool {
        (0..stack.len()).any(|depth| {
            self.check_depth_capability(stack, depth)
        })
    }

    /// Checks if a specific depth can iterate
    fn check_depth_capability(&self, stack: &IteratorStack, depth: usize) -> bool {
        stack.scope_at_depth(depth)
            .and_then(|_| stack.context_at_depth(depth))
            .map(|context| {
                let can_iterate = !context.iterator_state.items.is_empty()
                    && !context.iterator_state.completed;
                debug!(
                    "Scope at depth {} can iterate: {} (items: {}, completed: {})",
                    depth,
                    can_iterate,
                    context.iterator_state.items.len(),
                    context.iterator_state.completed
                );
                can_iterate
            })
            .unwrap_or(false)
    }

    /// Executes direct field evaluation for simple expressions without iteration
    fn execute_direct_evaluation(
        &mut self,
        stack: &mut IteratorStack,
        chain: &ParsedChain,
        context: &ExecutionContext,
        entries: &mut Vec<IndexEntry>,
    ) -> IteratorStackResult<()> {
        debug!("No iterators can iterate, trying direct evaluation");

        self.ensure_root_scope(stack)?;
        self.setup_root_context(stack, context);
        
        let field_value = self
            .field_evaluator
            .evaluate_field_expression(chain, stack, 0)?;
        debug!("Direct evaluation returned: {}", field_value);

        let entry = self.create_index_entry(stack, chain, field_value.clone(), None)?;
        entries.push(entry);
        
        Ok(())
    }

    /// Ensures a root scope exists for simple field expressions
    fn ensure_root_scope(&self, stack: &mut IteratorStack) -> IteratorStackResult<()> {
        if stack.is_empty() {
            stack.push_scope(crate::transform::iterator_stack::ActiveScope {
                depth: 0,
                iterator_type: crate::transform::iterator_stack::IteratorType::Schema {
                    field_name: "_root".to_string(),
                },
                position: 0,
                total_items: 1,
                branch_path: "root".to_string(),
                parent_depth: None,
            })?;
        }
        Ok(())
    }

    /// Sets up root context with input data
    fn setup_root_context(&self, stack: &mut IteratorStack, context: &ExecutionContext) {
        if let Some(root_context) = stack.context_at_depth_mut(0) {
            let input_value = serde_json::to_value(&context.input_data)
                .unwrap_or(serde_json::Value::Null);
            root_context.values.insert("_root".to_string(), input_value);
        }
    }

    /// Executes iterative field evaluation for complex expressions
    fn execute_iterative_evaluation(
        &mut self,
        stack: &mut IteratorStack,
        chain: &ParsedChain,
        context: &ExecutionContext,
        entries: &mut Vec<IndexEntry>,
    ) -> IteratorStackResult<()> {
        let iteration_depth = self.calculate_iteration_depth(stack, chain, context);
        debug!(
            "Using iteration depth: {} (chain.depth: {}, emission_depth: {}, max_available: {})", 
            iteration_depth, 
            chain.depth, 
            context.emission_depth, 
            stack.len().saturating_sub(1)
        );
        
        let entry_creator = |current_stack: &mut IteratorStack, current_path: &[usize]| -> IteratorStackResult<()> {
            self.process_iteration_item(current_stack, chain, iteration_depth, entries, current_path)
        };
        
        IterationHelper::iterate_to_depth(stack, iteration_depth, entry_creator)?;
        Ok(())
    }

    /// Calculates the appropriate iteration depth for the chain
    fn calculate_iteration_depth(
        &self,
        stack: &IteratorStack,
        chain: &ParsedChain,
        context: &ExecutionContext,
    ) -> usize {
        let max_available_depth = stack.len().saturating_sub(1);
        chain.depth
            .min(context.emission_depth)
            .min(max_available_depth)
    }

    /// Processes a single item during iteration
    fn process_iteration_item(
        &mut self,
        current_stack: &mut IteratorStack,
        chain: &ParsedChain,
        iteration_depth: usize,
        entries: &mut Vec<IndexEntry>,
        current_path: &[usize],
    ) -> IteratorStackResult<()> {
        debug!(
            "iterate_to_depth callback called for chain: {}",
            chain.expression
        );

        let field_value = self.field_evaluator.evaluate_field_expression(
            chain,
            current_stack,
            iteration_depth,
        )?;
        debug!("evaluate_field_expression returned: {}", field_value);

        let entry = self.create_index_entry(
            current_stack, 
            chain, 
            field_value,
            Some(current_path)
        )?;
        entries.push(entry);

        Ok(())
    }

    /// Creates an IndexEntry with a deterministic row_id and field value
    fn create_index_entry(
        &self,
        stack: &IteratorStack,
        chain: &ParsedChain,
        value: Value,
        current_path: Option<&[usize]>,
    ) -> IteratorStackResult<IndexEntry> {
        let row_id = if let Some(path) = current_path {
            if path.is_empty() { "0".to_string() } else { path.iter().map(|i| i.to_string()).collect::<Vec<_>>().join("/") }
        } else {
            // Direct evaluation path
            "0".to_string()
        };
        Ok(IndexEntry {
            row_id,
            value,
            atom_uuid: ReducerHelper::generate_atom_uuid(stack)?,
            metadata: ReducerHelper::extract_metadata(stack)?,
            expression: chain.expression.clone(),
        })
    }

    /// Checks for performance warnings and adds them to the warnings vector
    fn check_performance_warnings(
        &self,
        warnings: &mut Vec<ExecutionWarning>,
        entries: &[IndexEntry],
        chain: &ParsedChain,
    ) {
        const PERFORMANCE_THRESHOLD: usize = 1000;
        
        if entries.len() > PERFORMANCE_THRESHOLD {
            warnings.push(ExecutionWarning {
                warning_type: ExecutionWarningType::PerformanceDegradation,
                message: format!("High entry count detected: {} entries generated.", entries.len()),
                field: Some(chain.expression.clone()),
            });
            debug!("Added performance warning for {} entries", entries.len());
        }
    }
}
