use super::{SchemaCore, SchemaInterpreter};
use crate::schema::types::{DeclarativeSchemaDefinition, Schema, SchemaError};
use std::path::Path;

impl SchemaCore {
    /// The definitive parser for declarative schema files.
    pub async fn parse_schema_file(&self, path: &Path) -> Result<Option<Schema>, SchemaError> {
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return Err(SchemaError::InvalidData(format!(
                    "Failed to read {}: {}",
                    path.display(),
                    e
                )))
            }
        };
        let declarative_schema = serde_json::from_str::<DeclarativeSchemaDefinition>(&contents)
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to parse declarative schema: {}", e))
            })?;
        Ok(Some(SchemaInterpreter::interpret(declarative_schema)?))
    }
}
