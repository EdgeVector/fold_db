use super::SchemaCore;
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
        Ok(Some(
            self.interpret_declarative_schema(declarative_schema)
                .await?,
        ))
    }

    /// Interprets a declarative schema definition and populates runtime fields.
    pub async fn interpret_declarative_schema(
        &self,
        mut declarative_schema: DeclarativeSchemaDefinition,
    ) -> Result<Schema, SchemaError> {
        // Populate runtime_fields using the method on DeclarativeSchemaDefinition
        declarative_schema.populate_runtime_fields()?;

        Ok(declarative_schema)
    }
}
