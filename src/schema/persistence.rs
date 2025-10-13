use super::SchemaCore;
use crate::schema::types::{Schema, SchemaError, DeclarativeSchemaDefinition};
use std::collections::HashMap;
use std::path::Path;

impl SchemaCore {

    /// The definitive parser for declarative schema files.
    pub fn parse_schema_file(&self, path: &Path) -> Result<Option<Schema>, SchemaError> {
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return Err(SchemaError::InvalidData(format!("Failed to read {}: {}", path.display(), e)))
            }
        };
        let declarative_schema = serde_json::from_str::<DeclarativeSchemaDefinition>(&contents)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to parse declarative schema: {}", e)))?;
        Ok(Some(self.interpret_declarative_schema(declarative_schema)?))
    }


    /// Interprets a declarative schema definition and populates runtime fields.
    /// Now Schema = DeclarativeSchemaDefinition, so we just populate the runtime_fields HashMap
    pub fn interpret_declarative_schema(
        &self,
        mut declarative_schema: DeclarativeSchemaDefinition,
    ) -> Result<Schema, SchemaError> {
        // Populate runtime_fields using the method on DeclarativeSchemaDefinition
        declarative_schema.populate_runtime_fields()?;

        // Register transforms if this schema has transform_fields
        if let Some(transform_fields) = &declarative_schema.transform_fields {
            self.register_declarative_transforms(&declarative_schema, transform_fields)?;
        }

        Ok(declarative_schema)
    }

    /// Registers declarative transforms using the event bus
    pub(crate) fn register_declarative_transforms(
        &self,
        declarative_schema: &DeclarativeSchemaDefinition,
        transform_fields: &HashMap<String, String>,
    ) -> Result<(), SchemaError> {
        use crate::fold_db_core::infrastructure::message_bus::events::schema_events::TransformRegistrationRequest;
        use crate::schema::types::transform::{Transform, TransformRegistration};
        use uuid::Uuid;

        // Create ONE transform for the entire schema (stores only schema name, not full schema)
        let transform_id = declarative_schema.name.clone();
        let transform = Transform::from_schema_name(declarative_schema.name.clone());
        
        // Collect ALL trigger fields from ALL field expressions
        let mut all_trigger_fields = Vec::new();
        for field_expression in transform_fields.values() {
            let fields = DeclarativeSchemaDefinition::extract_inputs_from_expression(field_expression);
            all_trigger_fields.extend(fields);
        }
        
        // Remove duplicates by converting to HashSet and back
        let unique_trigger_fields: std::collections::HashSet<_> = all_trigger_fields.into_iter().collect();
        let trigger_fields: Vec<String> = unique_trigger_fields.into_iter().collect();
        
        // Create the registration for the single transform
        let registration = TransformRegistration {
            transform_id: transform_id.clone(),
            transform,
            trigger_fields,
        };

        // Create the registration request event
        let correlation_id = Uuid::new_v4().to_string();
        let registration_request = TransformRegistrationRequest {
            registration,
            correlation_id,
        };

        // Publish the event to the message bus
        self.get_message_bus().publish(registration_request)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to publish transform registration request: {}", e)))?;

        Ok(())
    }
}
