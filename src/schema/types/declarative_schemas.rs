

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use crate::schema::types::key_config::KeyConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldDefinition {
    pub field_expression: Option<String>,
}

// Custom deserializer for DeclarativeSchemaDefinition that uses the constructor
impl<'de> serde::Deserialize<'de> for DeclarativeSchemaDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Define a temporary struct for deserialization
        #[derive(serde::Deserialize)]
        struct DeclarativeSchemaDefinitionHelper {
            name: String,
            // Allow schema_type to be omitted; we will derive from key if missing
            schema_type: Option<crate::schema::types::schema::SchemaType>,
            #[serde(skip_serializing_if = "Option::is_none")]
            key: Option<KeyConfig>,
            // Accept either an array of strings or an object map and normalize later
            #[serde(skip_serializing_if = "Option::is_none")]
            fields: Option<serde_json::Value>,
            #[serde(skip_serializing_if = "Option::is_none")]
            transform_fields: Option<HashMap<String, String>>,
        }

        // Deserialize into the helper struct
        let helper = DeclarativeSchemaDefinitionHelper::deserialize(deserializer)?;
        
        // Normalize fields into Option<Vec<String>> supporting multiple shapes
        let normalized_fields: Option<Vec<String>> = match helper.fields {
            None => None,
            Some(val) => {
                if let Some(arr) = val.as_array() {
                    // Expect array of strings
                    let mut out: Vec<String> = Vec::new();
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            out.push(s.to_string());
                        } else {
                            return Err(serde::de::Error::custom("Invalid fields array; expected strings"));
                        }
                    }
                    Some(out)
                } else if let Some(obj) = val.as_object() {
                    // Accept object map and use keys as field names
                    let mut names: Vec<String> = obj.keys().cloned().collect();
                    names.sort();
                    Some(names)
                } else {
                    return Err(serde::de::Error::custom("Invalid fields; expected array or object map"));
                }
            }
        };

        // Determine schema_type if omitted
        let normalized_schema_type = match (&helper.schema_type, &helper.key) {
            (Some(st), _) => st.clone(),
            (None, Some(k)) => {
                use crate::schema::types::schema::SchemaType;
                let has_hash = k.hash_field.is_some();
                let has_range = k.range_field.is_some();
                if has_hash && has_range {
                    SchemaType::HashRange { keyconfig: k.clone() }
                } else if has_range || has_hash {
                    // If either key exists (but not both), treat as Range
                    SchemaType::Range { keyconfig: k.clone() }
                } else {
                    SchemaType::Single
                }
            }
            (None, None) => {
                use crate::schema::types::schema::SchemaType;
                SchemaType::Single
            }
        };

        // Use the constructor to create the actual struct with generated mappings
        Ok(DeclarativeSchemaDefinition::new(
            helper.name,
            normalized_schema_type,
            helper.key,
            normalized_fields,
            helper.transform_fields,
        ))
    }
}


/// Declarative schema definition used by declarative transforms.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DeclarativeSchemaDefinition {
        /// Schema name (same as transform name)
        pub name: String,
        /// Schema type ("Single" | "HashRange")
        pub schema_type: crate::schema::types::schema::SchemaType,
        /// Key configuration (required when schema_type == "HashRange")
        #[serde(skip_serializing_if = "Option::is_none")]
        pub key: Option<KeyConfig>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub fields: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub transform_fields: Option<HashMap<String, String>>,

        #[serde(skip)]
        inputs_schema_fields: Vec<String>,

        // Key to hash code.  Used for unique resolution of keys.
        #[serde(skip)]
        key_to_hash_code: HashMap<String, String>,

        // Field to hash code.  Used for unique resolution of fields.
        #[serde(skip)]
        field_to_hash_code: HashMap<String, String>,

        // Hash of the code to the code itself.  Used for unique resolution of transforms.
        #[serde(skip)]
        hash_to_code: HashMap<String, String>,
    }

impl DeclarativeSchemaDefinition {

    /// Creates a new DeclarativeSchemaDefinition and generates all hash mappings.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The schema name (same as transform name)
    /// * `schema_type` - The schema type ("Single" | "HashRange")
    /// * `key` - Optional key configuration (required when schema_type == "HashRange")
    /// * `fields` - Field definitions with their mapping expressions
    /// 
    /// # Returns
    /// 
    /// A new DeclarativeSchemaDefinition with all hash mappings populated
    pub fn new(
        name: String,
        schema_type: crate::schema::types::schema::SchemaType,
        key: Option<KeyConfig>,
        fields: Option<Vec<String>>,
        transform_fields: Option<HashMap<String, String>>,
    ) -> Self {
        let mut schema = Self {
            name,
            schema_type,
            key,
            fields,
            transform_fields,
            inputs_schema_fields: Vec::new(),
            key_to_hash_code: HashMap::new(),
            field_to_hash_code: HashMap::new(),
            hash_to_code: HashMap::new(),
        };
        
        // Generate all mappings after creation
        schema.generate_hash_to_code_mappings();
        schema.generate_inputs();
        schema
    }

    fn generate_inputs(&mut self) {
        let mut inputs_schema_fields = Vec::new();
        for code_def in self.hash_to_code.keys() {
            inputs_schema_fields.push(
                self.hash_to_code.get(code_def).unwrap().split(".").take(2).collect::<Vec<&str>>().join(".")
            );
        }
        self.inputs_schema_fields = inputs_schema_fields;

    }

    pub fn get_inputs(&self) -> Vec<String> {
        self.inputs_schema_fields.clone()
    }

    pub fn get_field_to_hash_code(&self) -> HashMap<String, String> {
        self.field_to_hash_code.clone()
    }

    pub fn get_key_to_hash_code(&self) -> HashMap<String, String> {
        self.key_to_hash_code.clone()
    }

    /// Generates hash-to-code mappings for all keys and fields in the declarative schema.
    /// 
    /// This function hashes every line from the keys (hash_field and range_field) and every 
    /// field expression (atom_uuid expressions) and stores them in the hash_to_code HashMap.
    /// 
    /// # Adds mappings to key_to_hash_code, field_to_hash_code, and hash_to_code.
    fn generate_hash_to_code_mappings(&mut self) {
        let mut hash_to_code = HashMap::new();
        
        // Hash field expressions if provided; tolerate missing transform_fields
        if let Some(map) = self.transform_fields.as_ref() {
            for (field_name, field_def) in map.iter() {
                let field_def_str = field_def.as_str();
                if !field_def_str.trim().is_empty() {
                    let hash = Self::hash_expression(field_def_str);
                    hash_to_code.insert(hash.clone(), field_def_str.to_string());
                    self.field_to_hash_code.insert(field_name.clone(), hash);
                }
            }
        }
        
        self.hash_to_code = hash_to_code;
    }
    
    /// Generates a SHA256 hash for a given expression.
    /// 
    /// # Arguments
    /// 
    /// * `expression` - The expression string to hash
    /// 
    /// # Returns
    /// 
    /// A hex-encoded SHA256 hash of the expression
    fn hash_expression(expression: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(expression.as_bytes());
        let hash_bytes = hasher.finalize();
        format!("{:x}", hash_bytes)
    }

    /// Gets a reference to the key-to-hash-code mapping.
    pub fn key_to_hash_code(&self) -> &HashMap<String, String> {
        &self.key_to_hash_code
    }

    /// Gets a reference to the field-to-hash-code mapping.
    pub fn field_to_hash_code(&self) -> &HashMap<String, String> {
        &self.field_to_hash_code
    }

    /// Gets a reference to the hash-to-code mapping.
    pub fn hash_to_code(&self) -> &HashMap<String, String> {
        &self.hash_to_code
    }

    
}