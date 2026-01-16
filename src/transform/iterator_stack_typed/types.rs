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
    /// Whether this item was generated from text tokens (should emit as value_text)
    pub is_text_token: bool,
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
    /// Iterate over items of a source field
    Schema { field_name: String },
    /// Apply a registered iterator function
    IteratorFunction {
        name: String,
        params: Vec<String>,
        field_name: String,
    },
    /// Apply a registered reducer function
    ReducerFunction {
        name: String,
        params: Vec<String>,
        field_name: String,
    },
}
