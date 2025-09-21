//! Reducer functions and value processing
//!
//! Contains reducer functions, metadata extraction, and value processing
//! utilities for field execution.

use crate::transform::iterator_stack::errors::{IteratorStackError, IteratorStackResult};
use crate::transform::iterator_stack::types::IteratorStack;
use serde_json::Value;
use std::collections::HashMap;

/// Helper methods for reducer functions and value processing
pub struct ReducerHelper;

impl ReducerHelper {
    /// Generates a unique atom UUID for an entry
    pub fn generate_atom_uuid(_stack: &IteratorStack) -> IteratorStackResult<String> {
        // For now, generate a simple UUID
        // In a real implementation, this would be more sophisticated
        Ok(format!("atom_{}", uuid::Uuid::new_v4()))
    }

    /// Extracts metadata from the current stack state
    pub fn extract_metadata(stack: &IteratorStack) -> IteratorStackResult<HashMap<String, Value>> {
        let mut metadata = HashMap::new();
        metadata.insert(
            "depth".to_string(),
            Value::Number(serde_json::Number::from(stack.len())),
        );
        metadata.insert(
            "timestamp".to_string(),
            Value::String(chrono::Utc::now().to_rfc3339()),
        );
        Ok(metadata)
    }

    /// Applies a reducer function to a list of values
    pub fn apply_reducer(values: &[Value], reducer_name: &str) -> IteratorStackResult<Value> {
        match reducer_name {
            "sum" => {
                let mut sum = 0.0;
                for value in values {
                    if let Some(num) = value.as_f64() {
                        sum += num;
                    }
                }
                Ok(Value::Number(
                    serde_json::Number::from_f64(sum).unwrap_or(serde_json::Number::from(0)),
                ))
            }
            "count" => Ok(Value::Number(serde_json::Number::from(values.len()))),
            "first" => Ok(values.first().cloned().unwrap_or(Value::Null)),
            "last" => Ok(values.last().cloned().unwrap_or(Value::Null)),
            _ => Err(IteratorStackError::ExecutionError {
                message: format!("Unknown reducer: {}", reducer_name),
            }),
        }
    }
}
