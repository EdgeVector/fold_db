use crate::schema::types::field::FieldVariant;
use crate::schema::types::key_config::KeyConfig;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use ts_rs::TS;
use utoipa::ToSchema;

/// Represents the schema-level type information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS, ToSchema)]
#[ts(export, export_to = "src/datafold_node/static-react/src/types/generated.ts")]
pub enum SchemaType {
    /// Single schema without range semantics
    Single,
    /// Schema that stores data in a key range
    Range { keyconfig: KeyConfig },
    /// Schema that uses hashed and ranged keys for partitioning
    HashRange { keyconfig: KeyConfig },
}


pub fn default_schema_type() -> SchemaType {
    SchemaType::Single
}

/// Defines the structure, permissions, and payment requirements for a data collection.
///
/// A Schema is the fundamental building block for data organization in the database.
/// It defines:
/// - The collection's name and identity
/// - Field definitions with their types and constraints
/// - Field-level permission policies
/// - Payment requirements for data access
/// - Field mappings for schema transformation
///
/// Schemas provide a contract for data storage and access, ensuring:
/// - Consistent data structure
/// - Proper access control
/// - Payment validation
/// - Data transformation rules
#[derive(Debug, Clone, Serialize, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "src/datafold_node/static-react/src/types/generated.ts")]
pub struct Schema {
    /// Unique name identifying this schema
    pub name: String,
    /// The type of schema. Defaults to a key range schema.
    pub schema_type: SchemaType,
    /// Universal key configuration for all schema types
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub key: Option<KeyConfig>,
    /// Collection of fields with their definitions and configurations
    #[ts(type = "Record<string, any>")]
    pub fields: HashMap<String, FieldVariant>,
    /// SHA256 hash of the schema content for integrity verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl Schema {
    /// Creates a new Schema with the specified name.
    ///
    /// Initializes an empty schema with:
    /// - No fields
    /// - Default payment configuration
    ///
    /// # Arguments
    ///
    /// * `name` - Unique identifier for this schema
    #[must_use]
    pub fn new(
        name: String,
        key: Option<KeyConfig>,
        fields: HashMap<String, FieldVariant>,
        hash: Option<String>,
    ) -> Self {
        let mut schema_type = SchemaType::Single;
        if let Some(key_config) = key.as_ref() {
            let has_hash = key_config.hash_field.is_some();
            let has_range = key_config.range_field.is_some();
            schema_type = if has_hash && has_range {
                SchemaType::HashRange { keyconfig: key_config.clone() }
            } else {
                SchemaType::Range { keyconfig: key_config.clone() }
            };
        }
        Self {
            name,
            schema_type: schema_type.clone(),
            key: key.as_ref().cloned(),
            fields,
            hash,
        }
    }
}