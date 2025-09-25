//! HashRange Query Processor
//!
//! Handles query processing for HashRange schemas using field resolution.

use crate::db_operations::DbOperations;
use crate::schema::{Schema, SchemaError};
use crate::schema::types::field::HashRangeFilter;
use serde_json::Value;
use std::sync::Arc;
use crate::schema::types::field::Field;
use std::collections::HashMap;

/// Processor for HashRange schema queries using field resolution
pub struct HashRangeQueryProcessor {
    db_ops: Arc<DbOperations>,
}

impl HashRangeQueryProcessor {
    /// Create a new HashRange query processor
    pub fn new(db_ops: Arc<DbOperations>) -> Self {
        Self { db_ops }
    }


    pub fn query_with_filter(
        &self,
        schema: &mut Schema,
        fields: &[String],
        filter: Option<HashRangeFilter>,
    ) -> Result<HashMap<String, Value>, SchemaError> {
        let mut result = HashMap::new();
        for (field_name, field) in schema.fields.iter_mut() {
            if !fields.contains(field_name) {
                continue;
            }
            let field_value = field.resolve_value(&self.db_ops, filter.clone())?;
            result.insert(field_name.clone(), field_value);
        }
        Ok(result)
    }
}
