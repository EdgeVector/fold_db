//! Core execution engine types and main execution methods

use crate::transform::iterator_stack::chain_parser::{FieldAlignment, ParsedChain};
use crate::transform::iterator_stack::field_alignment::{FieldAlignmentInfo, AlignmentValidationResult};
use crate::transform::iterator_stack::stack::IteratorStack;
use crate::transform::iterator_stack::errors::{IteratorStackError, IteratorStackResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use log::debug;

use super::field_execution::{FieldExecutionResult, DefaultFieldExecutor, FieldExecutor};
use super::iterator_management::IteratorManager;

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
            items_per_depth: self.calculate_items_per_depth(&index_entries),
            memory_usage_bytes: self.estimate_memory_usage(&index_entries),
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
            self.create_default_scopes(&mut stack, chain, &context.input_data)?;
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

    /// Creates default iterator scopes when none are provided
    fn create_default_scopes(
        &self,
        stack: &mut IteratorStack,
        chain: &ParsedChain,
        input_data: &Value,
    ) -> IteratorStackResult<()> {
        debug!("Creating default scopes for chain: {}", chain.expression);
        
        // Analyze the chain operations to determine what scopes to create
        let mut current_depth = 0;
        let mut current_ops = Vec::new();
        let mut branch_path = String::new();
        let mut last_field_name = String::new();
        
        for operation in chain.operations.iter() {
            current_ops.push(operation.clone());
            
            match operation {
                crate::transform::iterator_stack::chain_parser::ChainOperation::FieldAccess(field_name) => {
                    last_field_name = field_name.clone();
                    if branch_path.is_empty() {
                        branch_path = field_name.clone();
                    } else {
                        branch_path = format!("{}.{}", branch_path, field_name);
                    }
                    
                    // Check if this field should create an iterator scope (for arrays)
                    if self.should_create_iterator_scope(field_name, input_data) {
                        debug!("Creating Schema iterator scope for field: {} at depth: {}", field_name, current_depth);
                        
                        let iterator_type = crate::transform::iterator_stack::stack::IteratorType::Schema {
                            field_name: field_name.clone(),
                        };
                        
                        let active_scope = crate::transform::iterator_stack::stack::ActiveScope {
                            depth: current_depth,
                            iterator_type,
                            position: 0,
                            total_items: 0,
                            branch_path: branch_path.clone(),
                            parent_depth: if current_depth > 0 { Some(current_depth - 1) } else { None },
                        };
                        
                        stack.push_scope(active_scope)?;
                        current_depth += 1;
                        current_ops.clear();
                    }
                }
                crate::transform::iterator_stack::chain_parser::ChainOperation::Map => {
                    // Check if we should create a scope for the last field
                    if !last_field_name.is_empty() && self.should_create_iterator_scope(&last_field_name, input_data) {
                        debug!("Creating Schema iterator scope for field: {} at depth: {}", last_field_name, current_depth);
                        
                        let iterator_type = crate::transform::iterator_stack::stack::IteratorType::Schema {
                            field_name: last_field_name.clone(),
                        };
                        
                        let active_scope = crate::transform::iterator_stack::stack::ActiveScope {
                            depth: current_depth,
                            iterator_type,
                            position: 0,
                            total_items: 0,
                            branch_path: branch_path.clone(),
                            parent_depth: if current_depth > 0 { Some(current_depth - 1) } else { None },
                        };
                        
                        stack.push_scope(active_scope)?;
                        current_depth += 1;
                        current_ops.clear();
                    }
                }
                crate::transform::iterator_stack::chain_parser::ChainOperation::SplitByWord => {
                    // Create a WordSplit iterator scope for the last field
                    if !last_field_name.is_empty() {
                        debug!("Creating WordSplit iterator scope for field: {} at depth: {}", last_field_name, current_depth);
                        
                        let iterator_type = crate::transform::iterator_stack::stack::IteratorType::WordSplit {
                            field_name: last_field_name.clone(),
                        };
                        
                        let active_scope = crate::transform::iterator_stack::stack::ActiveScope {
                            depth: current_depth,
                            iterator_type,
                            position: 0,
                            total_items: 0,
                            branch_path: branch_path.clone(),
                            parent_depth: if current_depth > 0 { Some(current_depth - 1) } else { None },
                        };
                        
                        stack.push_scope(active_scope)?;
                        current_depth += 1;
                        current_ops.clear();
                    }
                }
                crate::transform::iterator_stack::chain_parser::ChainOperation::SplitArray => {
                    // Create an ArraySplit iterator scope for the last field
                    if !last_field_name.is_empty() {
                        debug!("Creating ArraySplit iterator scope for field: {} at depth: {}", last_field_name, current_depth);
                        
                        let iterator_type = crate::transform::iterator_stack::stack::IteratorType::ArraySplit {
                            field_name: last_field_name.clone(),
                        };
                        
                        let active_scope = crate::transform::iterator_stack::stack::ActiveScope {
                            depth: current_depth,
                            iterator_type,
                            position: 0,
                            total_items: 0,
                            branch_path: branch_path.clone(),
                            parent_depth: if current_depth > 0 { Some(current_depth - 1) } else { None },
                        };
                        
                        stack.push_scope(active_scope)?;
                        current_depth += 1;
                        current_ops.clear();
                    }
                }
                _ => {
                    // For other operations, continue processing
                }
            }
        }
        
        debug!("Created {} scopes", stack.len());
        Ok(())
    }
    
    /// Determines if a field should create an iterator scope
    fn should_create_iterator_scope(&self, field_name: &str, input_data: &Value) -> bool {
        // Check if the field exists and is an array
        if let Some(field_value) = input_data.get(field_name) {
            if field_value.is_array() {
                let array = field_value.as_array().unwrap();
                // Create iterator scope for any array, even single-element arrays
                // This is needed for expressions like "BlogPost.map().title" where BlogPost
                // contains an array of blog post objects
                debug!("Field {} is an array with {} elements", field_name, array.len());
                return !array.is_empty();
            } else if field_value.is_string() {
                // For string fields, we can create scopes for word splitting
                let text = field_value.as_str().unwrap();
                let word_count = text.split_whitespace().count();
                debug!("Field {} is a string with {} words", field_name, word_count);
                return word_count > 1;
            }
        }
        debug!("Field {} is not an array/string or doesn't exist", field_name);
        false
    }

    /// Calculates items per depth for statistics
    fn calculate_items_per_depth(&self, entries: &[IndexEntry]) -> HashMap<usize, usize> {
        let mut items_per_depth = HashMap::new();
        for entry in entries {
            if let Some(depth) = entry.metadata.get("depth").and_then(|v| v.as_u64()) {
                *items_per_depth.entry(depth as usize).or_insert(0) += 1;
            }
        }
        items_per_depth
    }

    /// Estimates memory usage of index entries
    fn estimate_memory_usage(&self, entries: &[IndexEntry]) -> usize {
        let mut total_size = 0;
        for entry in entries {
            total_size += std::mem::size_of::<IndexEntry>();
            total_size += entry.hash_value.to_string().len();
            total_size += entry.range_value.to_string().len();
            total_size += entry.atom_uuid.len();
            total_size += entry.metadata.len() * 64; // Rough estimate for metadata
        }
        total_size
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for executing a set of field expressions
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Input data to process
    pub input_data: Value,
    /// Field alignment information
    pub field_alignments: HashMap<String, FieldAlignmentInfo>,
    /// Maximum depth for emission
    pub emission_depth: usize,
    /// Additional context variables
    pub variables: HashMap<String, Value>,
}

/// Result of executing field expressions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Generated index entries
    pub index_entries: Vec<IndexEntry>,
    /// Execution statistics
    pub statistics: ExecutionStatistics,
    /// Any warnings generated during execution
    pub warnings: Vec<ExecutionWarning>,
}

/// A single index entry produced by the execution engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Hash field value (used for indexing)
    pub hash_value: Value,
    /// Range field value (used for sorting/filtering)
    pub range_value: Value,
    /// Unique identifier for the atom
    pub atom_uuid: String,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
    /// Field expression that generated this entry
    pub expression: String,
}

/// Statistics about execution performance
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionStatistics {
    /// Total number of index entries generated
    pub total_entries: usize,
    /// Number of items per depth level
    pub items_per_depth: HashMap<usize, usize>,
    /// Estimated memory usage in bytes
    pub memory_usage_bytes: usize,
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of cache misses
    pub cache_misses: usize,
}

/// Warning generated during execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionWarning {
    /// Type of warning
    pub warning_type: ExecutionWarningType,
    /// Warning message
    pub message: String,
    /// Field that generated the warning (if applicable)
    pub field: Option<String>,
}

/// Types of execution warnings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionWarningType {
    /// Performance degradation warning
    PerformanceDegradation,
    /// Memory usage warning
    MemoryUsage,
    /// Data quality warning
    DataQuality,
    /// Configuration warning
    Configuration,
}
