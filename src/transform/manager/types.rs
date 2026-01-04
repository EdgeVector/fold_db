use crate::fold_db_core::query::formatter::Record;
use crate::schema::SchemaError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;

/// Result of transform execution containing structured records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResult {
    /// The records produced by the transform
    pub records: Vec<Record>,
    /// Metadata about the execution
    pub metadata: HashMap<String, Value>,
}

impl TransformResult {
    /// Create a new TransformResult with the given records
    pub fn new(records: Vec<Record>) -> Self {
        Self {
            records,
            metadata: HashMap::new(),
        }
    }

    /// Create a TransformResult from a single record
    pub fn from_single_record(record: Record) -> Self {
        Self::new(vec![record])
    }

    /// Convert to a single Record by merging all records (for backward compatibility)
    pub fn to_single_record(&self) -> Record {
        if self.records.is_empty() {
            return Record {
                fields: HashMap::new(),
                metadata: HashMap::new(),
            };
        }

        if self.records.len() == 1 {
            return self.records[0].clone();
        }

        // Merge all records into a single record
        let mut merged_fields = HashMap::new();
        let mut merged_metadata = HashMap::new();
        for (i, record) in self.records.iter().enumerate() {
            for (key, value) in &record.fields {
                // Add index suffix to avoid field name conflicts
                let indexed_key = if self.records.len() > 1 {
                    format!("{}_row_{}", key, i)
                } else {
                    key.clone()
                };
                merged_fields.insert(indexed_key.clone(), value.clone());

                // Merge metadata with the same indexed key
                if let Some(meta) = record.metadata.get(key) {
                    merged_metadata.insert(indexed_key, meta.clone());
                }
            }
        }

        Record {
            fields: merged_fields,
            metadata: merged_metadata,
        }
    }
}

use async_trait::async_trait;

/// Trait abstraction over transform execution for easier testing.
/// All execution is now event-driven through the message bus.
#[async_trait]
pub trait TransformRunner: Send + Sync {
    async fn execute_transform_with_context(
        &self,
        transform_id: &str,
        mutation_context: &Option<
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
        >,
    ) -> Result<TransformResult, SchemaError>;
    fn transform_exists(&self, transform_id: &str) -> Result<bool, SchemaError>;
    fn get_transforms_for_field(
        &self,
        schema_name: &str,
        field_name: &str,
    ) -> Result<HashSet<String>, SchemaError>;
}
