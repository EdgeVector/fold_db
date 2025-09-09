//! Statistics calculation helpers for execution engine
//!
//! Contains methods for calculating execution statistics and performance metrics.

use super::types::IndexEntry;
use std::collections::HashMap;

/// Helper methods for statistics calculation
pub struct StatisticsHelper;

impl StatisticsHelper {
    /// Calculates items per depth for statistics
    pub fn calculate_items_per_depth(entries: &[IndexEntry]) -> HashMap<usize, usize> {
        let mut items_per_depth = HashMap::new();
        for entry in entries {
            if let Some(depth) = entry.metadata.get("depth").and_then(|v| v.as_u64()) {
                *items_per_depth.entry(depth as usize).or_insert(0) += 1;
            }
        }
        items_per_depth
    }

    /// Estimates memory usage of index entries
    pub fn estimate_memory_usage(entries: &[IndexEntry]) -> usize {
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
