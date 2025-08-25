//! Chain syntax parser for iterator stack expressions
//!
//! Parses expressions like `blogpost.map().content.split_by_word().map()` and
//! tracks iterator depths and branch structures.

use crate::schema::indexing::errors::{IteratorStackError, IteratorStackResult, constants};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single operation in a chain expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChainOperation {
    /// Access a field (e.g., `content`, `tags`)
    FieldAccess(String),
    /// Map operation that creates an iterator scope
    Map,
    /// Split array operation
    SplitArray,
    /// Split by word operation
    SplitByWord,
    /// Apply a reducer function
    Reducer(String),
    /// Access special field like `$atom_uuid`
    SpecialField(String),
}

/// Represents a parsed chain expression with depth and branch information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedChain {
    /// Original expression string
    pub expression: String,
    /// Sequence of operations in the chain
    pub operations: Vec<ChainOperation>,
    /// Iterator depth (number of .map() calls)
    pub depth: usize,
    /// Branch identifier for fan-out detection
    pub branch: String,
    /// Iterator scopes at each depth
    pub scopes: Vec<IteratorScope>,
}

/// Represents an iterator scope at a specific depth
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IteratorScope {
    /// Depth level (0 = root)
    pub depth: usize,
    /// Operations that led to this scope
    pub operations: Vec<ChainOperation>,
    /// Branch path up to this scope
    pub branch_path: String,
}

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

    /// Extracts the branch identifier up to a specific depth for fan-out detection
    pub fn extract_branch_up_to_depth(&self, operations: &[ChainOperation], target_depth: usize) -> IteratorStackResult<String> {
        let mut branch_parts = Vec::new();
        let mut current_depth = 0;

        for operation in operations {
            match operation {
                ChainOperation::FieldAccess(field) => {
                    branch_parts.push(field.clone());
                }
                ChainOperation::Map => {
                    current_depth += 1;
                    if current_depth >= target_depth {
                        // Stop when we reach the target depth
                        break;
                    }
                }
                ChainOperation::SplitArray | ChainOperation::SplitByWord => {
                    // These are part of the branch definition but don't increase depth
                    continue;
                }
                _ => {}
            }
        }

        if branch_parts.is_empty() {
            return Err(IteratorStackError::InvalidIteratorChain {
                chain: operations.iter().map(|op| format!("{:?}", op)).collect::<Vec<_>>().join("."),
                reason: "No field access found for branch extraction".to_string(),
            });
        }

        Ok(branch_parts.join("."))
    }

    /// Tokenizes the expression into individual operations
    fn tokenize(&self, expression: &str) -> IteratorStackResult<Vec<ChainOperation>> {
        let mut operations = Vec::new();
        let parts: Vec<&str> = expression.split('.').collect();

        for part in parts {
            let operation = match part {
                "map()" => ChainOperation::Map,
                "split_array()" => ChainOperation::SplitArray,
                "split_by_word()" => ChainOperation::SplitByWord,
                part if part.starts_with('$') => {
                    ChainOperation::SpecialField(part.to_string())
                }
                part if part.ends_with("()") => {
                    // Check if it's a reducer function
                    let func_name = &part[..part.len() - 2];
                    if self.is_reducer_function(func_name) {
                        ChainOperation::Reducer(func_name.to_string())
                    } else {
                        return Err(IteratorStackError::InvalidChainSyntax {
                            expression: expression.to_string(),
                            reason: format!("Unknown function: {}", part),
                        });
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

        self.validate_operation_sequence(&operations, expression)?;
        Ok(operations)
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

            match (prev, current) {
                // Valid transitions
                (ChainOperation::FieldAccess(_), ChainOperation::Map) => {}
                (ChainOperation::FieldAccess(_), ChainOperation::FieldAccess(_)) => {}
                (ChainOperation::FieldAccess(_), ChainOperation::SplitArray) => {}
                (ChainOperation::FieldAccess(_), ChainOperation::SplitByWord) => {}
                (ChainOperation::FieldAccess(_), ChainOperation::SpecialField(_)) => {}
                (ChainOperation::Map, ChainOperation::FieldAccess(_)) => {}
                (ChainOperation::SplitArray, ChainOperation::Map) => {}
                (ChainOperation::SplitByWord, ChainOperation::Map) => {}
                (ChainOperation::Map, ChainOperation::Reducer(_)) => {}
                // Allow Map -> SpecialField for accessing special fields after map operations
                (ChainOperation::Map, ChainOperation::SpecialField(_)) => {}
                // Allow Map -> SplitArray and Map -> SplitByWord for nested splitting
                (ChainOperation::Map, ChainOperation::SplitArray) => {}
                (ChainOperation::Map, ChainOperation::SplitByWord) => {}

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
                ChainOperation::SplitArray | ChainOperation::SplitByWord => {
                    // These are part of the branch definition
                    continue;
                }
                _ => {}
            }
        }

        if branch_parts.is_empty() {
            return Err(IteratorStackError::InvalidIteratorChain {
                chain: operations.iter().map(|op| format!("{:?}", op)).collect::<Vec<_>>().join("."),
                reason: "No field access found for branch extraction".to_string(),
            });
        }

        Ok(branch_parts.join("."))
    }

    /// Builds iterator scopes for each depth level
    fn build_scopes(&self, operations: &[ChainOperation]) -> IteratorStackResult<Vec<IteratorScope>> {
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

    /// Checks if a function name is a valid reducer
    fn is_reducer_function(&self, func_name: &str) -> bool {
        matches!(
            func_name,
            "first" | "last" | "count" | "join" | "sum" | "max" | "min"
        )
    }

    /// Checks if a chain contains any reducer operations
    fn contains_reducer_operation(&self, chain: &ParsedChain) -> bool {
        chain.operations.iter().any(|op| matches!(op, ChainOperation::Reducer(_)))
    }

    /// Analyzes multiple chains for compatibility
    pub fn analyze_compatibility(
        &self,
        chains: &[ParsedChain],
    ) -> IteratorStackResult<CompatibilityAnalysis> {
        if chains.is_empty() {
            return Ok(CompatibilityAnalysis {
                max_depth: 0,
                compatible: true,
                branches: HashMap::new(),
                alignment_requirements: Vec::new(),
            });
        }

        let max_depth = chains.iter().map(|c| c.depth).max().unwrap_or(0);
        let mut branches = HashMap::new();
        let mut alignment_requirements = Vec::new();

        // Group chains by branch
        for chain in chains {
            branches
                .entry(chain.branch.clone())
                .or_insert_with(Vec::new)
                .push(chain.clone());
        }

        // Check for incompatible branches at the same depth
        let mut depth_branches: HashMap<usize, Vec<String>> = HashMap::new();
        for chain in chains {
            depth_branches
                .entry(chain.depth)
                .or_default()
                .push(chain.branch.clone());
        }

        // Check for cartesian fan-outs at each depth level using scope information
        let _max_depth = chains.iter().map(|c| c.depth).max().unwrap_or(0);
        for depth in 1..=max_depth {
            let mut branch_paths_at_depth = Vec::new();
            for chain in chains {
                if depth <= chain.scopes.len() {
                    let scope = &chain.scopes[depth - 1];
                    branch_paths_at_depth.push(scope.branch_path.clone());
                }
            }

            if branch_paths_at_depth.len() > 1 {
                let unique_paths: std::collections::HashSet<_> = branch_paths_at_depth.iter().collect();
                if unique_paths.len() > 1 {
                    return Err(IteratorStackError::AmbiguousFanoutDifferentBranches {
                        branches: unique_paths
                            .into_iter()
                            .cloned()
                            .collect(),
                    });
                }
            }
        }

        // Generate alignment requirements
        for chain in chains {
            let alignment = if self.contains_reducer_operation(chain) {
                FieldAlignment::Reduced
            } else if chain.depth == max_depth {
                FieldAlignment::OneToOne
            } else if chain.depth < max_depth {
                FieldAlignment::Broadcast
            } else {
                // This case should never be reached since max_depth is the maximum of all chain depths
                FieldAlignment::Broadcast
            };

            alignment_requirements.push(FieldAlignmentRequirement {
                field_expression: chain.expression.clone(),
                depth: chain.depth,
                alignment,
                branch: chain.branch.clone(),
            });
        }

        Ok(CompatibilityAnalysis {
            max_depth,
            compatible: true,
            branches,
            alignment_requirements,
        })
    }
}

/// Result of analyzing multiple chains for compatibility
#[derive(Debug, Clone)]
pub struct CompatibilityAnalysis {
    /// Maximum depth among all chains
    pub max_depth: usize,
    /// Whether the chains are compatible
    pub compatible: bool,
    /// Chains grouped by branch
    pub branches: HashMap<String, Vec<ParsedChain>>,
    /// Field alignment requirements
    pub alignment_requirements: Vec<FieldAlignmentRequirement>,
}

/// Field alignment types based on depth relative to maximum depth
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldAlignment {
    /// 1:1 aligned - uses maximum depth D
    OneToOne,
    /// Broadcast - uses shallower depth, duplicated across all rows at depth D
    Broadcast,
    /// Reduced - would exceed depth D, must be reduced
    Reduced,
}

/// Field alignment requirement for a specific field
#[derive(Debug, Clone)]
pub struct FieldAlignmentRequirement {
    /// Original field expression
    pub field_expression: String,
    /// Iterator depth of this field
    pub depth: usize,
    /// Required alignment type
    pub alignment: FieldAlignment,
    /// Branch identifier
    pub branch: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_chain_parsing() {
        let parser = ChainParser::new();
        let result = parser.parse("blogpost.map()").unwrap();

        assert_eq!(result.depth, 1);
        assert_eq!(result.branch, "blogpost");
        assert_eq!(result.operations.len(), 2);
        assert_eq!(result.scopes.len(), 1);
    }

    #[test]
    fn test_complex_chain_parsing() {
        let parser = ChainParser::new();
        let result = parser
            .parse("blogpost.map().content.split_by_word().map()")
            .unwrap();

        assert_eq!(result.depth, 2);
        assert_eq!(result.branch, "blogpost");
        assert_eq!(result.operations.len(), 5);
        assert_eq!(result.scopes.len(), 2);
    }

    #[test]
    fn test_special_field_parsing() {
        let parser = ChainParser::new();
        let result = parser.parse("blogpost.map().$atom_uuid").unwrap();

        assert_eq!(result.depth, 1);
        assert_eq!(result.branch, "blogpost");
        assert!(matches!(
            result.operations.last(),
            Some(ChainOperation::SpecialField(_))
        ));
    }

    #[test]
    fn test_invalid_chain_syntax() {
        let parser = ChainParser::new();
        let result = parser.parse("map().blogpost");

        assert!(result.is_err());
        if let Err(IteratorStackError::InvalidChainSyntax { .. }) = result {
            // Expected error type
        } else {
            panic!("Expected InvalidChainSyntax error");
        }
    }

    #[test]
    fn test_compatibility_analysis() {
        let parser = ChainParser::new();
        let chain1 = parser
            .parse("blogpost.map().content.split_by_word().map()")
            .unwrap();
        let chain2 = parser.parse("blogpost.map().publish_date").unwrap();

        let analysis = parser
            .analyze_compatibility(&[chain1, chain2])
            .unwrap();

        assert_eq!(analysis.max_depth, 2);
        assert!(analysis.compatible);
        assert_eq!(analysis.alignment_requirements.len(), 2);
    }

    #[test]
    fn test_cartesian_fanout_detection() {
        let parser = ChainParser::new();
        let chain1 = parser.parse("blogpost.map().tags.split_array().map()").unwrap();
        let chain2 = parser.parse("blogpost.map().comments.map()").unwrap();

        let result = parser.analyze_compatibility(&[chain1, chain2]);

        assert!(result.is_err());
        if let Err(IteratorStackError::AmbiguousFanoutDifferentBranches { .. }) = result {
            // Expected error type
        } else {
            panic!("Expected AmbiguousFanoutDifferentBranches error");
        }
    }

    #[test]
    fn test_reducer_operation_alignment() {
        let parser = ChainParser::new();
        let chain1 = parser.parse("blogpost.map().content.map().first()").unwrap();
        let chain2 = parser.parse("blogpost.map().content.map().title").unwrap();

        let analysis = parser
            .analyze_compatibility(&[chain1, chain2])
            .unwrap();

        assert_eq!(analysis.max_depth, 2);
        assert!(analysis.compatible);
        assert_eq!(analysis.alignment_requirements.len(), 2);

        // Find the chain with reducer operation
        let reducer_chain = analysis
            .alignment_requirements
            .iter()
            .find(|req| req.field_expression.contains("first"))
            .unwrap();

        // The chain with reducer operation should have Reduced alignment
        assert_eq!(reducer_chain.alignment, FieldAlignment::Reduced);
        assert_eq!(reducer_chain.depth, 2); // depth is 2 for the chain with .map().map().first()

        // The chain without reducer should have OneToOne alignment (max depth)
        let non_reducer_chain = analysis
            .alignment_requirements
            .iter()
            .find(|req| req.field_expression.contains("title"))
            .unwrap();
        assert_eq!(non_reducer_chain.alignment, FieldAlignment::OneToOne);
        assert_eq!(non_reducer_chain.depth, 2); // depth is 2 for the chain with .map().map()
    }
}