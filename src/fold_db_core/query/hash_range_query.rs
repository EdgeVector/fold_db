//! HashRange Query Processor
//!
//! Handles query processing for HashRange schemas using field resolution.

use crate::db_operations::DbOperations;
use crate::schema::types::field::Field;
use crate::schema::types::field::FieldValue;
use crate::schema::types::field::HashRangeFilter;
use crate::schema::types::key_value::KeyValue;
use crate::schema::{Schema, SchemaError};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;

/// Processor for HashRange schema queries using field resolution
pub struct HashRangeQueryProcessor {
    db_ops: Arc<DbOperations>,
}

impl HashRangeQueryProcessor {
    /// Create a new HashRange query processor
    pub fn new(db_ops: Arc<DbOperations>) -> Self {
        Self { db_ops }
    }

    pub async fn query_with_filter(
        &self,
        schema: &mut Schema,
        fields: &[String],
        filter: Option<HashRangeFilter>,
        as_of: Option<DateTime<Utc>>,
    ) -> Result<HashMap<String, HashMap<KeyValue, FieldValue>>, SchemaError> {
        let current_user = crate::logging::core::get_current_user_id();
        log::info!(
            "🔍 HashRangeQueryProcessor: schema={}, filter={:?}, user_context={:?}",
            schema.name,
            filter,
            current_user
        );
        let mut result = HashMap::new();
        let return_all = fields.is_empty();
        for (field_name, field) in schema.runtime_fields.iter_mut() {
            if !return_all && !fields.contains(field_name) {
                continue;
            }
            log::debug!("🔍 Resolving field: {}", field_name);
            let field_value = field
                .resolve_value(&self.db_ops, filter.clone(), as_of)
                .await?;
            log::debug!(
                "✅ Field '{}' resolved {} values",
                field_name,
                field_value.len()
            );
            result.insert(field_name.clone(), field_value);
        }
        log::debug!(
            "✅ HashRangeQueryProcessor::query_with_filter: returning {} fields",
            result.len()
        );
        Ok(result)
    }
}
