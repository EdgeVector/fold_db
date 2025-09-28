use crate::schema::types::field::FieldVariant;
use crate::schema::types::key_config::KeyConfig;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use ts_rs::TS;

/// Represents the schema-level type information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
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
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "src/datafold_node/static-react/src/types/generated.ts")]
pub struct Schema {
    /// Unique name identifying this schema
    pub name: String,
    /// The type of schema. Defaults to a key range schema.
    #[serde(default = "default_schema_type")]
    pub schema_type: SchemaType,
    /// Universal key configuration for all schema types
    #[serde(skip_serializing_if = "Option::is_none")]
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
        schema_type: SchemaType,
        key: Option<KeyConfig>,
        fields: HashMap<String, FieldVariant>,
        hash: Option<String>,
    ) -> Self {
        Self {
            name,
            schema_type,
            key,
            fields,
            hash,
        }
    }
}