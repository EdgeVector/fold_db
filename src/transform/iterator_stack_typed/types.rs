use std::collections::HashMap;

use crate::schema::types::field::FieldValue;
use crate::schema::types::key_value::KeyValue;

/// Input dataset type for the typed iterator engine
pub type TypedInput = HashMap<String, HashMap<KeyValue, FieldValue>>;

/// A single item flowing through iteration
#[derive(Clone, Debug)]
pub struct IterationItem {
    pub key: KeyValue,
    pub value: FieldValue,
}

/// An emitted entry from evaluation
#[derive(Clone, Debug)]
pub struct EmittedEntry {
    pub row_id: String,
    /// For index fields that persist full atoms, use the original atom_uuid
    pub atom_uuid: String,
    /// The evaluated textual value for split operations (e.g., a single word)
    pub value_text: Option<String>,
}

#[derive(Clone, Debug)]
pub enum IteratorSpec {
    /// Iterate over items of a source field (schema.map())
    Schema { field_name: String },
    /// Split an array field into its elements (field.split_array())
    ArraySplit { field_name: String },
    /// Split a string value into words (field.split_by_word())
    WordSplit { field_name: String },
}


