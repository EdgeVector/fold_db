//! Stateless schema interpretation: `DeclarativeSchemaDefinition` → runtime `Schema`.
//!
//! This is a pure ETL operation with no runtime state dependencies. It can be
//! tested in isolation from the `SchemaCore` cache manager. Extracting this from
//! `SchemaCore` makes it clear that interpretation is a pure transformation and
//! not entangled with persistent cache state or in-memory lookup maps.

use crate::schema::types::{DeclarativeSchemaDefinition, Schema, SchemaError};

/// Stateless interpreter that turns a parsed declarative schema definition
/// into a runtime [`Schema`] with `runtime_fields` populated.
pub struct SchemaInterpreter;

impl SchemaInterpreter {
    /// Convert a declarative schema definition into a runtime [`Schema`].
    ///
    /// This is a pure function: it does not touch the database, the schema
    /// cache, or any other shared state. It only materialises the
    /// `runtime_fields` map on the definition so that downstream query and
    /// mutation code can look fields up by name.
    pub fn interpret(
        mut declarative_schema: DeclarativeSchemaDefinition,
    ) -> Result<Schema, SchemaError> {
        declarative_schema.populate_runtime_fields()?;
        Ok(declarative_schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpret_populates_runtime_fields() {
        let json = r#"{
            "name": "InterpreterTest",
            "key": { "range_field": "ts" },
            "fields": { "content": {}, "ts": {} }
        }"#;
        let declarative: DeclarativeSchemaDefinition =
            serde_json::from_str(json).expect("parse declarative schema");

        // Pre-condition: runtime_fields is empty (it's #[serde(skip)]).
        assert!(declarative.runtime_fields.is_empty());

        let schema = SchemaInterpreter::interpret(declarative).expect("interpret");

        assert_eq!(schema.name, "InterpreterTest");
        assert!(schema.runtime_fields.contains_key("content"));
        assert!(schema.runtime_fields.contains_key("ts"));
    }

    #[test]
    fn interpret_is_idempotent_on_prepopulated_schema() {
        let json = r#"{
            "name": "Idem",
            "key": { "range_field": "ts" },
            "fields": { "a": {}, "ts": {} }
        }"#;
        let declarative: DeclarativeSchemaDefinition = serde_json::from_str(json).expect("parse");
        let once = SchemaInterpreter::interpret(declarative).expect("first interpret");
        let twice = SchemaInterpreter::interpret(once.clone()).expect("second interpret");
        assert_eq!(once.runtime_fields.len(), twice.runtime_fields.len());
        assert_eq!(once.name, twice.name);
    }
}
