use super::{schema_lock_error, SchemaCore, SchemaState};
use crate::logging::features::{log_feature, LogFeature};
use crate::schema::constants::{
    ATOM_UUID_FIELD, DEFAULT_OUTPUT_FIELD_NAME, DEFAULT_TRANSFORM_ID_SUFFIX, KEY_FIELD_NAME,
};
use crate::schema::types::{JsonSchemaDefinition, Schema, SchemaError, DeclarativeSchemaDefinition};
use log::info;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::schema::types::schema::SchemaType;
use crate::schema::types::field::RangeField;
use crate::schema::types::field::HashRangeField;
use crate::schema::types::field::SingleField;
use crate::schema::types::field::FieldVariant;
use crate::fees::payment_config::SchemaPaymentConfig;
use crate::fees::types::config::FieldPaymentConfig;
use crate::fees::types::config::TrustDistanceScaling;
use crate::permissions::types::policy::{PermissionsPolicy, TrustDistance};
use crate::schema::types::field::common::FieldCommon;

impl SchemaCore {

    /// The definitive parser.
    pub(crate) fn parse_schema_file(&self, path: &Path) -> Result<Option<Schema>, SchemaError> {
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


    /// Interprets a declarative schema definition and converts it to a Schema.
    pub fn interpret_declarative_schema(
        &self,
        declarative_schema: DeclarativeSchemaDefinition,
    ) -> Result<Schema, SchemaError> {

        let default_permissions_policy = PermissionsPolicy::new(
            TrustDistance::Distance(0),
            TrustDistance::Distance(1),
        );
        let default_payment_config = FieldPaymentConfig {
            base_multiplier: 1.0,
            trust_distance_scaling: TrustDistanceScaling::None,
            min_payment: None,
        };
        let default_field_mappers = HashMap::new();
        let default_inner_field = FieldCommon::new(
            default_permissions_policy.clone(),
            default_payment_config.clone(),
            default_field_mappers.clone(),
        );

        // Convert fields from FieldDefinition to FieldVariant
        let mut fields = HashMap::new();
        let mut add_field = |field_name: String| {    
            let schema_type = declarative_schema.schema_type.clone();
            match &schema_type {
                SchemaType::HashRange { .. } => {

                    let hashrange_field = HashRangeField {
                        inner: default_inner_field.clone(),
                        molecule_hash_range: None,
                    };

                    fields.insert(field_name, FieldVariant::HashRange(hashrange_field));
                }
                SchemaType::Range { .. } => {
                    let range_field = RangeField {
                        inner: default_inner_field.clone(),
                        molecule_range: None,
                    };
                    
                    fields.insert(field_name, FieldVariant::Range(range_field));
                }
                SchemaType::Single => {
                    let single_field = SingleField {
                        inner: default_inner_field.clone(),
                    };

                    fields.insert(field_name, FieldVariant::Single(single_field));
                }
            }
        };

        for field_name in declarative_schema.fields.clone().unwrap() {
            add_field(field_name);
        }

        for (field_name, _) in declarative_schema.transform_fields.clone().unwrap() {
            add_field(field_name);
        }

        if let Some(transform_fields) = &declarative_schema.transform_fields {
            // Register declarative transforms using the event bus
            self.register_declarative_transforms(&declarative_schema, transform_fields)?;
        }

        // Create the schema with appropriate type
        let schema = Schema {
            name: declarative_schema.name.clone(),
            schema_type: declarative_schema.schema_type.clone(),
            key: declarative_schema.key.clone(), // Copy universal key configuration
            fields,
            payment_config: SchemaPaymentConfig {
                base_multiplier: 1.0,
                min_payment_threshold: 0,
            },
            hash: None,
        };

        Ok(schema)
    }

    /// Registers declarative transforms using the event bus
    fn register_declarative_transforms(
        &self,
        declarative_schema: &DeclarativeSchemaDefinition,
        transform_fields: &HashMap<String, String>,
    ) -> Result<(), SchemaError> {
        use crate::fold_db_core::infrastructure::message_bus::events::schema_events::TransformRegistrationRequest;
        use crate::schema::types::transform::{Transform, TransformRegistration};
        use uuid::Uuid;

        for (field_name, _field_expression) in transform_fields {
            // Create a transform ID based on schema name and field name
            let transform_id = format!("{}_{}", declarative_schema.name, field_name);
            
            // Create the transform from the declarative schema
            let transform = Transform::from_declarative_schema(declarative_schema.clone());
            
            // Determine trigger fields
            let trigger_fields = declarative_schema.get_inputs();
            
            // Create the registration
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

            log::info!(
                "📤 Published TransformRegistrationRequest for transform '{}' on field '{}'",
                transform_id,
                field_name
            );
        }

        Ok(())
    }
}
