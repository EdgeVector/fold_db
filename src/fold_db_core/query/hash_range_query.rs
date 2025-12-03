//! HashRange Query Processor
//!
//! Handles query processing for HashRange schemas using field resolution.

use crate::db_operations::DbOperationsV2;
use crate::schema::{Schema, SchemaError};
use crate::schema::types::field::HashRangeFilter;
use std::sync::Arc;
use crate::schema::types::field::Field;
use std::collections::HashMap;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::field::FieldValue;

/// Processor for HashRange schema queries using field resolution
pub struct HashRangeQueryProcessor {
    db_ops: Arc<DbOperationsV2>,
}

impl HashRangeQueryProcessor {
    /// Create a new HashRange query processor
    pub fn new(db_ops: Arc<DbOperationsV2>) -> Self {
        Self { db_ops }
    }


    pub async fn query_with_filter(
        &self,
        schema: &mut Schema,
        fields: &[String],
        filter: Option<HashRangeFilter>,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        log::debug!("🔍 HashRangeQueryProcessor::query_with_filter: schema={}, fields={:?}, filter={:?}", 
            schema.name, fields, filter);
        let mut result = HashMap::new();
        for (field_name, field) in schema.runtime_fields.iter_mut() {
            if !fields.contains(field_name) {
                continue;
            }
            log::debug!("🔍 Resolving field: {}", field_name);
            let field_value = field.resolve_value(&self.db_ops, filter.clone()).await?;
            log::debug!("✅ Field '{}' resolved {} values", field_name, field_value.len());
            result.insert(field_name.clone(), field_value);
        }
        log::debug!("✅ HashRangeQueryProcessor::query_with_filter: returning {} fields", result.len());
        Ok(result)
    }
}
