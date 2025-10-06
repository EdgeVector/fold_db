//! Validation logic for chain parser
//!
//! Contains validation algorithms and compatibility analysis for
//! chain expressions and multiple chain coordination.

use crate::transform::chain_parser::parser::ChainParser;
use crate::transform::chain_parser::types::{
    ChainOperation, CompatibilityAnalysis, ParsedChain,
};
use crate::transform::chain_parser::errors::{IteratorStackError, IteratorStackResult};
use std::collections::HashMap;

impl ChainParser {

    /// Extracts the branch identifier up to a specific depth for fan-out detection
    pub fn extract_branch_up_to_depth(
        &self,
        operations: &[ChainOperation],
        target_depth: usize,
    ) -> IteratorStackResult<String> {
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
                ChainOperation::Function { .. } => {
                    // Functions are part of the branch definition but don't increase depth
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
            });
        }

        let max_depth = chains.iter().map(|c| c.depth).max().unwrap_or(0);
        let mut branches = HashMap::new();

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
                let unique_paths: std::collections::HashSet<_> =
                    branch_paths_at_depth.iter().collect();
                if unique_paths.len() > 1 {
                    return Err(IteratorStackError::AmbiguousFanoutDifferentBranches {
                        branches: unique_paths.into_iter().cloned().collect(),
                    });
                }
            }
        }

        Ok(CompatibilityAnalysis {
            max_depth,
            compatible: true,
            branches,
        })
    }
}
