//! Field execution methods for different alignment types

use crate::transform::iterator_stack::chain_parser::ParsedChain;
use crate::transform::iterator_stack::stack::IteratorStack;
use crate::transform::iterator_stack::errors::{IteratorStackError, IteratorStackResult};
use serde_json::Value;
use std::collections::HashMap;
use log::debug;

use super::core::{ExecutionContext, IndexEntry, ExecutionWarning, ExecutionWarningType};
use super::field_evaluation::{DefaultFieldEvaluator, FieldEvaluator};

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
    field_evaluator: DefaultFieldEvaluator,
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

impl FieldExecutor for DefaultFieldExecutor {
    /// Executes OneToOne alignment - each iteration produces one entry
    fn execute_one_to_one(
        &mut self,
        stack: &mut IteratorStack,
        chain: &ParsedChain,
        context: &ExecutionContext,
    ) -> IteratorStackResult<FieldExecutionResult> {
        let mut entries = Vec::new();
        let mut warnings = Vec::new();

        debug!("execute_one_to_one starting for chain: {} at emission_depth: {}", chain.expression, context.emission_depth);
        debug!("Stack has {} scopes", stack.len());

        // Check if we have any iterators that can actually iterate
        let can_iterate = (0..stack.len()).any(|depth| {
            if let Some(_scope) = stack.scope_at_depth(depth) {
                if let Some(context) = stack.context_at_depth(depth) {
                    let can_iterate = !context.iterator_state.items.is_empty() && !context.iterator_state.completed;
                    debug!("Scope at depth {} can iterate: {} (items: {}, completed: {})",
                        depth, can_iterate, context.iterator_state.items.len(), context.iterator_state.completed);
                    can_iterate
                } else {
                    false
                }
            } else {
                false
            }
        });

        debug!("Stack can iterate: {}", can_iterate);

        if !can_iterate {
            // If we have no iterators, this might be a simple field expression
            // Try to evaluate it directly without iteration
            debug!("No iterators can iterate, trying direct evaluation");
            
            // For simple field expressions, we need to ensure the input data is available
            // If the stack has no scopes, create a temporary root scope
            if stack.is_empty() {
                // Create a temporary root scope for simple field expressions
                stack.push_scope(
                    crate::transform::iterator_stack::stack::ActiveScope {
                        depth: 0,
                        iterator_type: crate::transform::iterator_stack::stack::IteratorType::Schema { 
                            field_name: "_root".to_string() 
                        },
                        position: 0,
                        total_items: 1,
                        branch_path: "root".to_string(),
                        parent_depth: None,
                    }
                )?;
            }

            // Ensure root context has input data
            if let Some(root_context) = stack.context_at_depth_mut(0) {
                root_context.values.insert("_root".to_string(), context.input_data.clone());
            }
            
            let field_value = self.field_evaluator.evaluate_field_expression(chain, stack, 0)?;
            debug!("Direct evaluation returned: {}", field_value);
            
            // Create a single entry for simple field expressions
            let entry = IndexEntry {
                hash_value: field_value.clone(),
                range_value: field_value.clone(),
                atom_uuid: self.generate_atom_uuid(stack)?,
                metadata: self.extract_metadata(stack)?,
                expression: chain.expression.clone(),
            };
            entries.push(entry);
        } else {
            // Iterate through all combinations at the appropriate depth for the chain
            // For complex chains, we need to iterate to the depth where the chain can be evaluated
            // The iteration depth should be the maximum depth where we have actual iterators
            let max_available_depth = stack.len().saturating_sub(1); // Stack has scopes 0 to len-1
            let iteration_depth = chain.depth.min(context.emission_depth).min(max_available_depth);
            debug!("Using iteration depth: {} (chain.depth: {}, emission_depth: {}, max_available: {})", 
                iteration_depth, chain.depth, context.emission_depth, max_available_depth);
            self.iterate_to_depth(stack, iteration_depth, |current_stack, _current_path| {
                debug!("iterate_to_depth callback called for chain: {}", chain.expression);

                // Extract the field value at current context
                let field_value = self.field_evaluator.evaluate_field_expression(chain, current_stack, iteration_depth)?;
                debug!("evaluate_field_expression returned: {}", field_value);

                entries.push(IndexEntry {
                    hash_value: field_value,
                    range_value: Value::Null, // Will be set later when combining
                    atom_uuid: self.generate_atom_uuid(current_stack)?,
                    metadata: self.extract_metadata(current_stack)?,
                    expression: chain.expression.clone(),
                });

                Ok(())
            })?;
        }

        // Check for performance warnings
        if entries.len() > 1000 {
            warnings.push(ExecutionWarning {
                warning_type: ExecutionWarningType::PerformanceDegradation,
                message: format!("High entry count detected: {} entries generated. Consider using reduced alignment or optimizing field expressions.", entries.len()),
                field: Some(chain.expression.clone()),
            });
            debug!("Added performance warning for {} entries", entries.len());
        }

        debug!("execute_one_to_one completed, produced {} entries with {} warnings", entries.len(), warnings.len());

        Ok(FieldExecutionResult {
            entries,
            warnings,
        })
    }

    /// Executes Broadcast alignment - broadcasts values across all iterations
    fn execute_broadcast(
        &mut self,
        stack: &mut IteratorStack,
        chain: &ParsedChain,
        context: &ExecutionContext,
    ) -> IteratorStackResult<FieldExecutionResult> {
        let mut entries = Vec::new();
        let mut warnings = Vec::new();

        debug!("execute_broadcast starting for chain: {} at emission_depth: {}", chain.expression, context.emission_depth);

        // Check if we have any iterators that can actually iterate
        let can_iterate = (0..stack.len()).any(|depth| {
            if let Some(_scope) = stack.scope_at_depth(depth) {
                if let Some(context) = stack.context_at_depth(depth) {
                    !context.iterator_state.items.is_empty() && !context.iterator_state.completed
                } else {
                    false
                }
            } else {
                false
            }
        });

        debug!("Can iterate: {}, stack len: {}", can_iterate, stack.len());

        if !can_iterate {
            // If we have no iterators, this might be a simple field expression
            // Try to evaluate it directly without iteration
            debug!("Broadcast - No iterators can iterate, trying direct evaluation");
            
            // For simple field expressions, we need to ensure the input data is available
            // If the stack has no scopes, create a temporary root scope
            if stack.is_empty() {
                // Create a temporary root scope for simple field expressions
                stack.push_scope(
                    crate::transform::iterator_stack::stack::ActiveScope {
                        depth: 0,
                        iterator_type: crate::transform::iterator_stack::stack::IteratorType::Schema { 
                            field_name: "_root".to_string() 
                        },
                        position: 0,
                        total_items: 1,
                        branch_path: "root".to_string(),
                        parent_depth: None,
                    }
                )?;
            }

            // Ensure root context has input data
            if let Some(root_context) = stack.context_at_depth_mut(0) {
                root_context.values.insert("_root".to_string(), context.input_data.clone());
            }
            
            let field_value = self.field_evaluator.evaluate_field_expression(chain, stack, 0)?;
            debug!("Broadcast - Direct evaluation returned: {}", field_value);
            
            // Create a single entry for simple field expressions
            let entry = IndexEntry {
                hash_value: field_value.clone(),
                range_value: field_value.clone(),
                atom_uuid: self.generate_atom_uuid(stack)?,
                metadata: self.extract_metadata(stack)?,
                expression: chain.expression.clone(),
            };
            entries.push(entry);
        } else {
            // For broadcast, we need to iterate to the emission depth
            let actual_max_depth = stack.len().saturating_sub(1);
            let emission_depth = context.emission_depth.min(actual_max_depth);

            // Calculate how many entries we'll generate
            let mut emission_count = 1;
            for d in 0..=emission_depth {
                if let Some(context) = stack.context_at_depth(d) {
                    emission_count *= context.iterator_state.items.len().max(1);
                }
            }

            debug!("Broadcast emission_count: {}, actual_max_depth: {}, emission_depth: {}", emission_count, actual_max_depth, context.emission_depth);
            
            if emission_count == 0 {
                // No iterations at emission depth, nothing to broadcast
                debug!("Broadcast returning early - emission_count is 0");
                return Ok(FieldExecutionResult {
                    entries,
                    warnings,
                });
            }

            // Iterate to the emission depth and broadcast the field value
            self.iterate_to_depth(stack, emission_depth, |current_stack, _current_path| {
                debug!("Processing iteration, current entries: {}", entries.len());
                let field_value = self.field_evaluator.evaluate_field_expression(chain, current_stack, emission_depth)?;
                debug!("Field value: {}", field_value);

                entries.push(IndexEntry {
                    hash_value: field_value,
                    range_value: Value::Null, // Will be set later when combining
                    atom_uuid: self.generate_atom_uuid(current_stack)?,
                    metadata: self.extract_metadata(current_stack)?,
                    expression: chain.expression.clone(),
                });

                Ok(())
            })?;
        }

        // Check for performance warnings
        if entries.len() > 1000 {
            warnings.push(ExecutionWarning {
                warning_type: ExecutionWarningType::PerformanceDegradation,
                message: format!("High entry count detected: {} entries generated. Consider using reduced alignment or optimizing field expressions.", entries.len()),
                field: Some(chain.expression.clone()),
            });
        }

        Ok(FieldExecutionResult {
            entries,
            warnings,
        })
    }

    /// Executes Reduced alignment - reduces values to a single entry
    fn execute_reduced(
        &mut self,
        stack: &mut IteratorStack,
        chain: &ParsedChain,
        context: &ExecutionContext,
    ) -> IteratorStackResult<FieldExecutionResult> {
        let mut entries = Vec::new();
        let warnings = Vec::new();

        debug!("execute_reduced starting for chain: {} at emission_depth: {}", chain.expression, context.emission_depth);

        // For reduced alignment, we collect all values and then reduce them
        let mut values = Vec::new();

        // Check if we have any iterators that can actually iterate
        let can_iterate = (0..stack.len()).any(|depth| {
            if let Some(_scope) = stack.scope_at_depth(depth) {
                if let Some(context) = stack.context_at_depth(depth) {
                    !context.iterator_state.items.is_empty() && !context.iterator_state.completed
                } else {
                    false
                }
            } else {
                false
            }
        });

        if can_iterate {
            // Iterate through all combinations and collect values
            let max_available_depth = stack.len().saturating_sub(1);
            let iteration_depth = chain.depth.min(context.emission_depth).min(max_available_depth);

            self.iterate_to_depth(stack, iteration_depth, |current_stack, _current_path| {
                let field_value = self.field_evaluator.evaluate_field_expression(chain, current_stack, iteration_depth)?;
                values.push(field_value);
                Ok(())
            })?;
        } else {
            // Direct evaluation for simple field expressions
            let field_value = self.field_evaluator.evaluate_field_expression(chain, stack, 0)?;
            values.push(field_value);
        }

        // Reduce the collected values
        if !values.is_empty() {
            let reduced_value = self.apply_reducer(&values, "sum")?; // Default to sum reducer

            let entry = IndexEntry {
                hash_value: reduced_value.clone(),
                range_value: reduced_value,
                atom_uuid: self.generate_atom_uuid(stack)?,
                metadata: self.extract_metadata(stack)?,
                expression: chain.expression.clone(),
            };
            entries.push(entry);
        }

        Ok(FieldExecutionResult {
            entries,
            warnings,
        })
    }
}

// Helper methods for field execution
impl DefaultFieldExecutor {
    /// Iterates to a specific depth and calls a callback for each combination
    fn iterate_to_depth<F>(
        &self,
        stack: &mut IteratorStack,
        target_depth: usize,
        mut callback: F,
    ) -> IteratorStackResult<()>
    where
        F: FnMut(&mut IteratorStack, &[usize]) -> IteratorStackResult<()>,
    {
        debug!("iterate_to_depth called with target_depth: {}, stack len: {}", target_depth, stack.len());
        self.iterate_recursive(stack, target_depth, &mut callback, &mut Vec::new())
    }

    /// Recursive iteration helper
    #[allow(clippy::only_used_in_recursion)]
    fn iterate_recursive<F>(
        &self,
        stack: &mut IteratorStack,
        target_depth: usize,
        callback: &mut F,
        current_path: &mut Vec<usize>,
    ) -> IteratorStackResult<()>
    where
        F: FnMut(&mut IteratorStack, &[usize]) -> IteratorStackResult<()>,
    {
        debug!("iterate_recursive: current_path.len()={}, target_depth={}", current_path.len(), target_depth);
        
        if current_path.len() > target_depth {
            return Ok(());
        }

        if current_path.len() == target_depth {
            // We've reached the target depth, iterate over all items at this depth
            debug!("Reached target depth, iterating over items");
            let current_depth = current_path.len();
            if let Some(context) = stack.context_at_depth(current_depth) {
                let items = context.iterator_state.items.clone();
                debug!("Found {} items at target depth", items.len());
                
                for (index, _item) in items.iter().enumerate() {
                    debug!("Processing item {} at target depth", index);
                    // Set the current item for this depth
                    if let Some(context) = stack.context_at_depth_mut(current_depth) {
                        context.iterator_state.current_item = Some(items[index].clone());
                    }
                    
                    current_path.push(index);
                    
                    // Call the callback for this item
                    callback(stack, current_path)?;
                    
                    current_path.pop();
                }
            }
            return Ok(());
        }

        // Get the current depth
        let current_depth = current_path.len();
        
        if let Some(context) = stack.context_at_depth(current_depth) {
            let items = context.iterator_state.items.clone();
            debug!("At depth {}, found {} items", current_depth, items.len());
            
            for (index, _item) in items.iter().enumerate() {
                debug!("Processing item {} at depth {}", index, current_depth);
                // Set the current item for this depth
                if let Some(context) = stack.context_at_depth_mut(current_depth) {
                    context.iterator_state.current_item = Some(items[index].clone());
                }
                
                current_path.push(index);
                
                // Recursively iterate to the next depth
                self.iterate_recursive(stack, target_depth, callback, current_path)?;
                
                current_path.pop();
            }
        }
        
        Ok(())
    }

    /// Generates a unique atom UUID for an entry
    fn generate_atom_uuid(&self, _stack: &IteratorStack) -> IteratorStackResult<String> {
        // For now, generate a simple UUID
        // In a real implementation, this would be more sophisticated
        Ok(format!("atom_{}", uuid::Uuid::new_v4()))
    }

    /// Extracts metadata from the current stack state
    fn extract_metadata(&self, stack: &IteratorStack) -> IteratorStackResult<HashMap<String, Value>> {
        let mut metadata = HashMap::new();
        metadata.insert("depth".to_string(), Value::Number(serde_json::Number::from(stack.len())));
        metadata.insert("timestamp".to_string(), Value::String(chrono::Utc::now().to_rfc3339()));
        Ok(metadata)
    }

    /// Applies a reducer function to a list of values
    fn apply_reducer(&self, values: &[Value], reducer_name: &str) -> IteratorStackResult<Value> {
        match reducer_name {
            "sum" => {
                let mut sum = 0.0;
                for value in values {
                    if let Some(num) = value.as_f64() {
                        sum += num;
                    }
                }
                Ok(Value::Number(serde_json::Number::from_f64(sum).unwrap_or(serde_json::Number::from(0))))
            }
            "count" => Ok(Value::Number(serde_json::Number::from(values.len()))),
            "first" => Ok(values.first().cloned().unwrap_or(Value::Null)),
            "last" => Ok(values.last().cloned().unwrap_or(Value::Null)),
            _ => Err(IteratorStackError::ExecutionError {
                message: format!("Unknown reducer: {}", reducer_name)
            }),
        }
    }
}

