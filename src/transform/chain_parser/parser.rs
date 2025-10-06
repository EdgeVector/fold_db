//! Core chain parsing logic
//!
//! Contains the main parsing algorithms and logic for converting
//! chain expressions into structured representations.

use crate::transform::chain_parser::types::{
    ChainOperation, IteratorScope, ParsedChain,
};
use crate::transform::chain_parser::errors::{
    constants, IteratorStackError, IteratorStackResult,
};
use crate::transform::functions::registry;

/// Parser for chain syntax expressions
pub struct ChainParser {
    /// Maximum allowed iterator depth
    max_depth: usize,
}

impl Default for ChainParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainParser {
    /// Creates a new chain parser with default settings
    pub fn new() -> Self {
        Self {
            max_depth: constants::MAX_ITERATOR_DEPTH,
        }
    }

    /// Creates a new chain parser with custom max depth
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self { max_depth }
    }

    /// Parses a chain expression into a structured representation
    pub fn parse(&self, expression: &str) -> IteratorStackResult<ParsedChain> {
        if expression.len() > constants::MAX_CHAIN_EXPRESSION_LENGTH {
            return Err(IteratorStackError::InvalidChainSyntax {
                expression: expression.to_string(),
                reason: format!(
                    "Expression too long: {} characters (max: {})",
                    expression.len(),
                    constants::MAX_CHAIN_EXPRESSION_LENGTH
                ),
            });
        }

        let operations = self.tokenize(expression)?;
        self.validate_operation_sequence(&operations, expression)?;
        let depth = self.calculate_depth(&operations);
        let branch = self.extract_branch(&operations)?;
        let scopes = self.build_scopes(&operations)?;

        if depth > self.max_depth {
            return Err(IteratorStackError::MaxDepthExceeded {
                current_depth: depth,
                max_depth: self.max_depth,
            });
        }

        Ok(ParsedChain {
            expression: expression.to_string(),
            operations,
            depth,
            branch,
            scopes,
        })
    }

    /// Tokenizes the expression into individual operations
    fn tokenize(&self, expression: &str) -> IteratorStackResult<Vec<ChainOperation>> {
        let mut operations = Vec::new();
        let parts: Vec<&str> = expression.split('.').collect();
        let reg = registry();

        for part in parts {
            let operation = match part {
                "map()" => ChainOperation::Map,
                part if part.starts_with('$') => ChainOperation::SpecialField(part.to_string()),
                part if part.ends_with("()") => {
                    // Extract function name and check registry
                    let func_name = &part[..part.len() - 2];
                    
                    if !reg.is_registered(func_name) {
                        return Err(IteratorStackError::InvalidChainSyntax {
                            expression: expression.to_string(),
                            reason: format!("Unknown function: {}", part),
                        });
                    }
                    
                    // TODO: Parse function parameters from syntax like split_by_word(sep: ",")
                    // For now, no parameters supported - params field exists for future use
                    ChainOperation::Function { 
                        name: func_name.to_string(),
                        params: Vec::new(),
                    }
                }
                part if !part.is_empty() => ChainOperation::FieldAccess(part.to_string()),
                _ => {
                    return Err(IteratorStackError::InvalidChainSyntax {
                        expression: expression.to_string(),
                        reason: "Empty part in chain".to_string(),
                    })
                }
            };
            operations.push(operation);
        }

        Ok(operations)
    }

    /// Calculates the iterator depth (number of .map() operations)
    fn calculate_depth(&self, operations: &[ChainOperation]) -> usize {
        operations
            .iter()
            .filter(|op| matches!(op, ChainOperation::Map))
            .count()
    }

    /// Extracts the branch identifier for fan-out detection
    fn extract_branch(&self, operations: &[ChainOperation]) -> IteratorStackResult<String> {
        let mut branch_parts = Vec::new();

        for operation in operations {
            match operation {
                ChainOperation::FieldAccess(field) => {
                    branch_parts.push(field.clone());
                }
                ChainOperation::Map => {
                    // Stop at the first map - everything before defines the branch
                    break;
                }
                ChainOperation::Function { .. } => {
                    // Functions are part of the branch definition
                    continue;
                }
                _ => {}
            }
        }

        if branch_parts.is_empty() {
            return Err(IteratorStackError::InvalidIteratorChain {
                chain: operations
                    .iter()
                    .map(|op| format!("{:?}", op))
                    .collect::<Vec<_>>()
                    .join("."),
                reason: "No field access found for branch extraction".to_string(),
            });
        }

        Ok(branch_parts.join("."))
    }

    /// Builds iterator scopes for each depth level
    fn build_scopes(
        &self,
        operations: &[ChainOperation],
    ) -> IteratorStackResult<Vec<IteratorScope>> {
        let mut scopes = Vec::new();
        let mut current_ops = Vec::new();
        let mut depth = 0;
        let mut branch_path = String::new();

        for operation in operations {
            current_ops.push(operation.clone());

            match operation {
                ChainOperation::FieldAccess(field) => {
                    if branch_path.is_empty() {
                        branch_path = field.clone();
                    } else {
                        branch_path = format!("{}.{}", branch_path, field);
                    }
                }
                ChainOperation::Map => {
                    scopes.push(IteratorScope {
                        depth,
                        operations: current_ops.clone(),
                        branch_path: branch_path.clone(),
                    });
                    depth += 1;
                    current_ops.clear();
                }
                _ => {}
            }
        }

        Ok(scopes)
    }

    /// Validates that the sequence of operations is valid
    fn validate_operation_sequence(
        &self,
        operations: &[ChainOperation],
        expression: &str,
    ) -> IteratorStackResult<()> {
        if operations.is_empty() {
            return Err(IteratorStackError::InvalidChainSyntax {
                expression: expression.to_string(),
                reason: "Empty expression".to_string(),
            });
        }

        // First operation must be a field access
        if !matches!(operations[0], ChainOperation::FieldAccess(_)) {
            return Err(IteratorStackError::InvalidChainSyntax {
                expression: expression.to_string(),
                reason: "Expression must start with a field access".to_string(),
            });
        }

        // Validate operation transitions
        for window in operations.windows(2) {
            let prev = &window[0];
            let current = &window[1];

            let reg = registry();
            
            match (prev, current) {
                // Valid transitions
                (ChainOperation::FieldAccess(_), ChainOperation::Map) => {}
                (ChainOperation::FieldAccess(_), ChainOperation::FieldAccess(_)) => {}
                (ChainOperation::FieldAccess(_), ChainOperation::Function { name, .. }) => {
                    // Iterator functions can follow field access
                    if !reg.is_iterator(name) && !reg.is_reducer(name) {
                        return Err(IteratorStackError::InvalidChainSyntax {
                            expression: expression.to_string(),
                            reason: format!("Function after field access must be registered: {}", name),
                        });
                    }
                }
                (ChainOperation::FieldAccess(_), ChainOperation::SpecialField(_)) => {}
                (ChainOperation::Map, ChainOperation::FieldAccess(_)) => {}
                (ChainOperation::Function { name, .. }, ChainOperation::Map) => {
                    // Iterator functions can be followed by map
                    if !reg.is_iterator(name) {
                        return Err(IteratorStackError::InvalidChainSyntax {
                            expression: expression.to_string(),
                            reason: format!("Only iterator functions can be followed by map(): {}", name),
                        });
                    }
                }
                (ChainOperation::Map, ChainOperation::Function { name, .. }) => {
                    // Both iterator and reducer functions can follow map
                    if !reg.is_iterator(name) && !reg.is_reducer(name) {
                        return Err(IteratorStackError::InvalidChainSyntax {
                            expression: expression.to_string(),
                            reason: format!("Function after map must be registered: {}", name),
                        });
                    }
                }
                // Allow Map -> SpecialField for accessing special fields after map operations
                (ChainOperation::Map, ChainOperation::SpecialField(_)) => {}
                
                (ChainOperation::Function { name: prev_name, .. }, ChainOperation::Function { name: curr_name, .. }) => {
                    // Iterator functions can be followed by reducer functions
                    if reg.is_iterator(prev_name) && reg.is_reducer(curr_name) {
                        // Valid: iterator -> reducer
                    } else {
                        return Err(IteratorStackError::InvalidChainSyntax {
                            expression: expression.to_string(),
                            reason: format!("Invalid function sequence: {} -> {}. Only iterator functions can be followed by reducer functions.", prev_name, curr_name),
                        });
                    }
                }

                // Invalid transitions
                _ => {
                    return Err(IteratorStackError::InvalidChainSyntax {
                        expression: expression.to_string(),
                        reason: format!("Invalid operation sequence: {:?} -> {:?}", prev, current),
                    });
                }
            }
        }

        Ok(())
    }
}
