//! Type aliases for common data structures

use std::collections::HashMap;
use crate::schema::types::KeyValue;
use crate::schema::types::field::FieldValue;

/// Type alias for query result maps (field -> key -> value)
pub type QueryResultMap = HashMap<String, HashMap<KeyValue, FieldValue>>;

