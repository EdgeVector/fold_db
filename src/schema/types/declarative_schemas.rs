use crate::schema::types::field::Field;
use crate::schema::types::key_config::KeyConfig;
use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
use crate::schema::types::topology::{JsonTopology, TopologyNode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::convert::TryFrom;

#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct FieldDefinition {
    pub field_expression: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[serde(into = "String", try_from = "String")]
#[cfg_attr(feature = "ts-bindings", ts(type = "string"))]
pub struct FieldMapper {
    source_schema: String,
    source_field: String,
}

impl FieldMapper {
    pub fn new<S: Into<String>, F: Into<String>>(source_schema: S, source_field: F) -> Self {
        Self {
            source_schema: source_schema.into(),
            source_field: source_field.into(),
        }
    }

    pub fn source_schema(&self) -> &str {
        &self.source_schema
    }

    pub fn source_field(&self) -> &str {
        &self.source_field
    }
}

impl TryFrom<String> for FieldMapper {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("FieldMapper definition cannot be empty".to_string());
        }

        let mut parts = trimmed.split('.');
        let source_schema = parts
            .next()
            .ok_or_else(|| "FieldMapper must include source schema".to_string())?
            .trim();
        let source_field = parts
            .next()
            .ok_or_else(|| "FieldMapper must include source field".to_string())?
            .trim();

        if source_schema.is_empty() || source_field.is_empty() {
            return Err("FieldMapper must include non-empty source schema and field".to_string());
        }

        if parts.next().is_some() {
            return Err("FieldMapper must be in 'schema.field' format".to_string());
        }

        Ok(Self::new(source_schema, source_field))
    }
}

impl TryFrom<&str> for FieldMapper {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}

impl From<FieldMapper> for String {
    fn from(mapper: FieldMapper) -> Self {
        format!("{}.{}", mapper.source_schema, mapper.source_field)
    }
}

impl<'__s> utoipa::ToSchema<'__s> for FieldMapper {
    fn schema() -> (
        &'__s str,
        utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
    ) {
        (
            "FieldMapper",
            utoipa::openapi::schema::ObjectBuilder::new()
                .schema_type(utoipa::openapi::schema::SchemaType::String)
                .into(),
        )
    }
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
            #[serde(skip_serializing_if = "Option::is_none")]
            descriptive_name: Option<String>,
            // Allow schema_type to be omitted; we will derive from key if missing
            schema_type: Option<SchemaType>,
            #[serde(skip_serializing_if = "Option::is_none")]
            key: Option<KeyConfig>,
            // Accept either an array of strings or an object map and normalize later
            #[serde(skip_serializing_if = "Option::is_none")]
            fields: Option<serde_json::Value>,
            #[serde(skip_serializing_if = "Option::is_none")]
            transform_fields: Option<HashMap<String, String>>,
            #[serde(skip_serializing_if = "Option::is_none", default)]
            field_mappers: Option<HashMap<String, FieldMapper>>,
            #[serde(skip_serializing_if = "Option::is_none", default)]
            field_molecule_uuids: Option<HashMap<String, String>>,
            #[serde(default)]
            field_topologies: HashMap<String, JsonTopology>,
            #[serde(skip_serializing_if = "Option::is_none", default)]
            field_topology_hashes: Option<HashMap<String, String>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            topology_hash: Option<String>,
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
                            return Err(serde::de::Error::custom(
                                "Invalid fields array; expected strings",
                            ));
                        }
                    }
                    Some(out)
                } else if let Some(obj) = val.as_object() {
                    // Accept object map and use keys as field names
                    let mut names: Vec<String> = obj.keys().cloned().collect();
                    names.sort();
                    Some(names)
                } else {
                    return Err(serde::de::Error::custom(
                        "Invalid fields; expected array or object map",
                    ));
                }
            }
        };

        // Determine schema_type if omitted
        let normalized_schema_type = match (&helper.schema_type, &helper.key) {
            (Some(st), _) => st.clone(),
            (None, Some(k)) => {
                let has_hash = k.hash_field.is_some();
                let has_range = k.range_field.is_some();
                if has_hash && has_range {
                    SchemaType::HashRange
                } else if has_range || has_hash {
                    // If either key exists (but not both), treat as Range
                    SchemaType::Range
                } else {
                    SchemaType::Single
                }
            }
            (None, None) => SchemaType::Single,
        };

        // Use the constructor to create the actual struct with generated mappings
        let mut schema = DeclarativeSchemaDefinition::new(
            helper.name,
            normalized_schema_type,
            helper.key,
            normalized_fields,
            helper.transform_fields,
            helper.field_mappers,
        );

        // Preserve descriptive_name and field_molecule_uuids from deserialization
        schema.descriptive_name = helper.descriptive_name;
        schema.field_molecule_uuids = helper.field_molecule_uuids;
        
        // Merge topologies from helper with schema's default topologies
        for (field_name, topology) in helper.field_topologies {
            schema.field_topologies.insert(field_name, topology);
        }
        
        // Preserve topology hashes
        schema.field_topology_hashes = helper.field_topology_hashes;
        schema.topology_hash = helper.topology_hash;

        Ok(schema)
    }
}

/// Declarative schema definition - the primary schema representation.
/// This is the unified schema type that replaces the old Schema/DeclarativeSchemaDefinition split.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(export, export_to = "bindings/src/datafold_node/static-react/src/types/generated.ts")
)]
pub struct DeclarativeSchemaDefinition {
    /// Schema name
    pub name: String,
    /// Human-readable descriptive name for the schema (used in AI-generated proposals)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptive_name: Option<String>,
    /// Schema type ("Single" | "Range" | "HashRange")
    pub schema_type: SchemaType,
    /// Key configuration (required when schema_type == "HashRange" or "Range")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<KeyConfig>,
    /// Field names - plain data fields without transformations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    /// Transform fields - computed fields with expressions (optional, only for transform schemas)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform_fields: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub field_mappers: Option<HashMap<String, FieldMapper>>,
    /// SHA256 hash of the schema content for integrity verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Molecule UUIDs for each field (persisted for data continuity after mutations)
    /// Maps field_name -> molecule_uuid. Synced from runtime_fields before persistence.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub field_molecule_uuids: Option<HashMap<String, String>>,
    /// Topology definitions for each field (defines JSON structure)
    /// Maps field_name -> JsonTopology. Every field MUST have a topology.
    pub field_topologies: HashMap<String, JsonTopology>,
    /// Hash of each field's topology for change detection
    /// Maps field_name -> topology_hash
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub field_topology_hashes: Option<HashMap<String, String>>,
    /// Hash of all field topologies combined - unique fingerprint of schema structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology_hash: Option<String>,

    // Runtime state fields (not serialized)
    /// Runtime field storage with molecules (for database operations)
    #[serde(skip)]
    #[cfg_attr(feature = "ts-bindings", ts(skip))]
    pub runtime_fields: HashMap<String, crate::schema::types::field::FieldVariant>,

    /// Input fields extracted from transform expressions
    #[serde(skip)]
    #[cfg_attr(feature = "ts-bindings", ts(skip))]
    inputs_schema_fields: Vec<String>,

    /// Source schemas extracted from input fields (for transforms)
    #[serde(skip)]
    #[cfg_attr(feature = "ts-bindings", ts(skip))]
    source_schemas: Vec<String>,

    /// Key to hash code mapping for transforms
    #[serde(skip)]
    #[cfg_attr(feature = "ts-bindings", ts(skip))]
    key_to_hash_code: HashMap<String, String>,

    /// Field to hash code mapping for transforms
    #[serde(skip)]
    #[cfg_attr(feature = "ts-bindings", ts(skip))]
    field_to_hash_code: HashMap<String, String>,

    /// Hash to code mapping for transforms
    #[serde(skip)]
    #[cfg_attr(feature = "ts-bindings", ts(skip))]
    hash_to_code: HashMap<String, String>,
}

// Manual PartialEq implementation that excludes runtime_fields from comparison
impl PartialEq for DeclarativeSchemaDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.descriptive_name == other.descriptive_name
            && self.schema_type == other.schema_type
            && self.key == other.key
            && self.fields == other.fields
            && self.transform_fields == other.transform_fields
            && self.field_mappers == other.field_mappers
            && self.hash == other.hash
            && self.field_molecule_uuids == other.field_molecule_uuids
            && self.field_topologies == other.field_topologies
            && self.field_topology_hashes == other.field_topology_hashes
            && self.topology_hash == other.topology_hash
        // Exclude runtime_fields, inputs_schema_fields, source_schemas, and hash mappings
        // These are derived/runtime state and don't affect schema identity
    }
}

impl DeclarativeSchemaDefinition {
    /// Populates runtime_fields from declarative schema definition
    /// This is called after deserializing from database to ensure runtime state is initialized
    /// Also regenerates transform metadata (hash mappings, inputs, source schemas) which are not persisted
    pub fn populate_runtime_fields(&mut self) -> Result<(), crate::schema::SchemaError> {
        use crate::schema::types::field::{FieldVariant, HashRangeField, RangeField, SingleField};
        use std::collections::HashMap;

        let default_field_mappers = HashMap::new();
        let mut runtime_fields = HashMap::new();
        let mut add_field = |field_name: String| {
            let schema_type = self.schema_type.clone();
            match &schema_type {
                SchemaType::HashRange => {
                    let hashrange_field = HashRangeField::new(default_field_mappers.clone(), None);
                    runtime_fields.insert(field_name, FieldVariant::HashRange(hashrange_field));
                }
                SchemaType::Range => {
                    let range_field = RangeField::new(default_field_mappers.clone(), None);
                    runtime_fields.insert(field_name, FieldVariant::Range(range_field));
                }
                SchemaType::Single => {
                    let single_field = SingleField::new(default_field_mappers.clone(), None);
                    runtime_fields.insert(field_name, FieldVariant::Single(single_field));
                }
            }
        };

        if let Some(field_list) = self.fields.clone() {
            for field_name in field_list {
                add_field(field_name);
            }
        }

        if let Some(transform_map) = self.transform_fields.clone() {
            for (field_name, _) in transform_map {
                add_field(field_name);
            }
        }

        self.runtime_fields = runtime_fields;

        if let Some(field_mappers) = &self.field_mappers {
            for (field_name, mapper) in field_mappers {
                if let Some(field) = self.runtime_fields.get_mut(field_name) {
                    let mut mapper_map = HashMap::new();
                    mapper_map.insert(field_name.clone(), mapper.clone());
                    field.common_mut().set_field_mappers(mapper_map);
                }
            }
        }

        // Restore molecule_uuids from persisted field_molecule_uuids
        if let Some(molecule_uuids) = &self.field_molecule_uuids {
            for (field_name, molecule_uuid) in molecule_uuids {
                if let Some(field) = self.runtime_fields.get_mut(field_name) {
                    field.common_mut().set_molecule_uuid(molecule_uuid.clone());
                }
            }
        }

        // Regenerate transform metadata that isn't persisted (marked with #[serde(skip)])
        // This is needed when schemas are loaded from the database
        self.generate_hash_to_code_mappings();
        self.generate_inputs();
        self.fetch_source_schemas();

        Ok(())
    }

    /// Syncs molecule UUIDs from runtime_fields to the persisted field_molecule_uuids
    /// Call this after mutations to ensure molecule_uuids are persisted
    pub fn sync_molecule_uuids(&mut self) {
        let mut molecule_uuids = HashMap::new();
        for (field_name, field) in &self.runtime_fields {
            if let Some(uuid) = field.common().molecule_uuid() {
                molecule_uuids.insert(field_name.clone(), uuid.clone());
            }
        }
        if !molecule_uuids.is_empty() {
            self.field_molecule_uuids = Some(molecule_uuids);
        }
    }

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
        schema_type: SchemaType,
        key: Option<KeyConfig>,
        fields: Option<Vec<String>>,
        transform_fields: Option<HashMap<String, String>>,
        field_mappers: Option<HashMap<String, FieldMapper>>,
    ) -> Self {
        let mut schema = Self {
            name,
            descriptive_name: None,
            schema_type,
            key,
            fields,
            transform_fields,
            field_mappers,
            hash: None,
            field_molecule_uuids: None,
            field_topologies: HashMap::new(),
            field_topology_hashes: None,
            topology_hash: None,
            runtime_fields: HashMap::new(),
            inputs_schema_fields: Vec::new(),
            source_schemas: Vec::new(),
            key_to_hash_code: HashMap::new(),
            field_to_hash_code: HashMap::new(),
            hash_to_code: HashMap::new(),
        };

        // Generate all mappings after creation
        schema.generate_hash_to_code_mappings();
        schema.generate_inputs();
        schema.fetch_source_schemas();
        schema
    }

    pub fn field_mappers(&self) -> Option<&HashMap<String, FieldMapper>> {
        self.field_mappers.as_ref()
    }

    /// Get classifications for a specific field from its topology
    pub fn get_field_classifications(&self, field_name: &str) -> Option<Vec<String>> {
        let topology = self.field_topologies.get(field_name)?;
        Self::extract_classifications_from_topology(&topology.root)
    }

    /// Extract classifications from a topology node (recursively)
    fn extract_classifications_from_topology(node: &TopologyNode) -> Option<Vec<String>> {
        match node {
            TopologyNode::Primitive { classifications, .. } => {
                classifications.clone()
            }
            TopologyNode::Array { value, .. } => {
                Self::extract_classifications_from_topology(value)
            }
            _ => None,
        }
    }

    /// Get topology for a specific field
    pub fn get_field_topology(&self, field_name: &str) -> Option<&JsonTopology> {
        self.field_topologies.get(field_name)
    }

    /// Check if a field has a topology defined
    pub fn has_field_topology(&self, field_name: &str) -> bool {
        self.field_topologies.contains_key(field_name)
    }

    /// Set topology for a specific field and compute its hash
    pub fn set_field_topology(&mut self, field_name: String, topology: JsonTopology) {
        // Compute and store individual field topology hash
        let topology_hash = topology.compute_hash();
        
        if self.field_topology_hashes.is_none() {
            self.field_topology_hashes = Some(HashMap::new());
        }
        if let Some(hashes) = self.field_topology_hashes.as_mut() {
            hashes.insert(field_name.clone(), topology_hash);
        }
        
        self.field_topologies.insert(field_name, topology);
        
        // Recompute schema-level topology hash
        self.compute_schema_topology_hash();
    }

    /// Validate a field value against its topology
    pub fn validate_field_value(
        &self,
        field_name: &str,
        value: &serde_json::Value,
    ) -> Result<(), crate::schema::types::errors::SchemaError> {
        if let Some(topology) = self.get_field_topology(field_name) {
            topology.validate(value)?;
            Ok(())
        } else {
            Err(crate::schema::types::errors::SchemaError::InvalidData(
                format!("No topology defined for field '{}'", field_name)
            ))
        }
    }

    /// Infer and set topologies from sample data
    pub fn infer_topologies_from_data(&mut self, data: &HashMap<String, serde_json::Value>) {
        for (field_name, value) in data {
            let topology = JsonTopology::infer_from_value(value);
            self.set_field_topology(field_name.clone(), topology);
        }
    }

    /// Compute schema-level topology hash from all field topology hashes
    /// This creates a unique fingerprint for the entire schema's structure
    pub fn compute_schema_topology_hash(&mut self) {
        if self.field_topologies.is_empty() {
            self.topology_hash = None;
            return;
        }

        // Collect and sort field topology hashes for deterministic hashing
        let mut sorted_fields: Vec<_> = self.field_topologies.keys().collect();
        sorted_fields.sort();

        let mut combined = String::new();
        for field_name in sorted_fields {
            if let Some(topology) = self.field_topologies.get(field_name) {
                combined.push_str(field_name);
                combined.push(':');
                combined.push_str(&topology.compute_hash());
                combined.push(';');
            }
        }

        let mut hasher = sha2::Sha256::new();
        hasher.update(combined.as_bytes());
        self.topology_hash = Some(format!("{:x}", hasher.finalize()));
    }

    /// Get the schema-level topology hash
    pub fn get_topology_hash(&self) -> Option<&String> {
        self.topology_hash.as_ref()
    }

    /// Get topology hash for a specific field
    pub fn get_field_topology_hash(&self, field_name: &str) -> Option<&String> {
        self.field_topology_hashes
            .as_ref()
            .and_then(|hashes| hashes.get(field_name))
    }

    fn generate_inputs(&mut self) {
        let mut inputs_schema_fields = Vec::new();
        for code_def in self.hash_to_code.keys() {
            let expression = self.hash_to_code.get(code_def).unwrap();

            // Split expression by "." and filter out elements containing "(" or ")"
            // This handles .map(), .filter(), .reduce(), etc.
            let parts: Vec<&str> = expression
                .split(".")
                .filter(|part| !part.contains("(") && !part.contains(")"))
                .collect();

            // Take the first two valid parts to form the field reference
            if parts.len() >= 2 {
                inputs_schema_fields.push(format!("{}.{}", parts[0], parts[1]));
            } else if parts.len() == 1 {
                // Fallback: if only one part, use it as-is
                inputs_schema_fields.push(parts[0].to_string());
            }
        }
        // Remove duplicates and sort
        inputs_schema_fields.sort();
        inputs_schema_fields.dedup();
        self.inputs_schema_fields = inputs_schema_fields;
    }

    pub fn get_inputs(&self) -> Vec<String> {
        self.inputs_schema_fields.clone()
    }

    fn fetch_source_schemas(&mut self) {
        let mut source_schemas = std::collections::HashSet::new();
        for input in &self.inputs_schema_fields {
            if let Some(source_schema) = input.split('.').next() {
                source_schemas.insert(source_schema.to_string());
            }
        }
        let mut source_schemas_vec: Vec<String> = source_schemas.into_iter().collect();
        source_schemas_vec.sort();
        self.source_schemas = source_schemas_vec;
    }

    pub fn get_source_schemas(&self) -> Vec<String> {
        self.source_schemas.clone()
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

    /// Extract input fields from a single transform expression.
    /// Example: "BlogPost.map().content.split_by_word().map()" -> ["BlogPost.content"]
    pub fn extract_inputs_from_expression(expression: &str) -> Vec<String> {
        let mut inputs = Vec::new();

        // Split expression by "." and filter out elements containing "(" or ")"
        // This handles .map(), .filter(), .reduce(), etc.
        let parts: Vec<&str> = expression
            .split(".")
            .filter(|part| !part.contains("(") && !part.contains(")"))
            .collect();

        // Take the first two valid parts to form the field reference
        if parts.len() >= 2 {
            inputs.push(format!("{}.{}", parts[0], parts[1]));
        } else if parts.len() == 1 {
            // Fallback: if only one part, use it as-is
            inputs.push(parts[0].to_string());
        }

        // Remove duplicates and sort
        inputs.sort();
        inputs.dedup();

        inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_metadata_after_serialize_deserialize() {
        use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;

        // Create a transform schema like BlogPostWordIndex
        let mut transform_fields = HashMap::new();
        transform_fields.insert(
            "word".to_string(),
            "BlogPost.map().content.split_by_word().map()".to_string(),
        );
        transform_fields.insert(
            "publish_date".to_string(),
            "BlogPost.map().publish_date".to_string(),
        );

        let schema = DeclarativeSchemaDefinition::new(
            "BlogPostWordIndex".to_string(),
            SchemaType::Single,
            None,
            None,
            Some(transform_fields),
            None,
        );

        // Check metadata was generated in constructor
        let inputs_before = schema.get_inputs();
        let source_schemas_before = schema.get_source_schemas();

        println!(
            "Before serialization: inputs={:?}, sources={:?}",
            inputs_before, source_schemas_before
        );
        assert!(
            !inputs_before.is_empty(),
            "Inputs should not be empty after construction"
        );
        assert!(
            !source_schemas_before.is_empty(),
            "Source schemas should not be empty after construction"
        );

        // Simulate save/load cycle
        let serialized = serde_json::to_string(&schema).unwrap();
        let mut loaded: DeclarativeSchemaDefinition = serde_json::from_str(&serialized).unwrap();

        // Check metadata after deserialization (should be empty due to #[serde(skip)])
        let inputs_after = loaded.get_inputs();
        let source_schemas_after = loaded.get_source_schemas();

        println!(
            "After deserialization (before populate): inputs={:?}, sources={:?}",
            inputs_after, source_schemas_after
        );

        // Now call populate_runtime_fields
        loaded.populate_runtime_fields().unwrap();

        let inputs_final = loaded.get_inputs();
        let source_schemas_final = loaded.get_source_schemas();

        println!(
            "After populate_runtime_fields: inputs={:?}, sources={:?}",
            inputs_final, source_schemas_final
        );

        assert!(
            !inputs_final.is_empty(),
            "Inputs should not be empty after populate"
        );
        assert!(
            !source_schemas_final.is_empty(),
            "Source schemas should not be empty after populate"
        );
        assert!(
            source_schemas_final.contains(&"BlogPost".to_string()),
            "Should have BlogPost as source schema"
        );
    }

    #[test]
    fn test_descriptive_name_serialization() {
        use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;

        // Create a schema with descriptive_name
        let mut schema = DeclarativeSchemaDefinition::new(
            "UserProfile".to_string(),
            SchemaType::Single,
            None,
            Some(vec!["name".to_string(), "email".to_string()]),
            None,
            None,
        );
        schema.descriptive_name = Some("User Profile Information".to_string());

        // Serialize to JSON
        let serialized = serde_json::to_string(&schema).unwrap();
        assert!(
            serialized.contains("User Profile Information"),
            "Serialized JSON should contain descriptive_name"
        );

        // Deserialize back
        let deserialized: DeclarativeSchemaDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(
            deserialized.descriptive_name,
            Some("User Profile Information".to_string()),
            "Descriptive name should be preserved after deserialization"
        );
        assert_eq!(
            deserialized.name, "UserProfile",
            "Schema name should be preserved"
        );
    }

    #[test]
    fn test_descriptive_name_optional() {
        use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;

        // Create a schema without descriptive_name
        let schema = DeclarativeSchemaDefinition::new(
            "UserProfile".to_string(),
            SchemaType::Single,
            None,
            Some(vec!["name".to_string(), "email".to_string()]),
            None,
            None,
        );

        assert_eq!(
            schema.descriptive_name, None,
            "Descriptive name should be None by default"
        );

        // Serialize to JSON
        let serialized = serde_json::to_string(&schema).unwrap();
        assert!(
            !serialized.contains("descriptive_name"),
            "Serialized JSON should not contain descriptive_name field when None"
        );

        // Deserialize back
        let deserialized: DeclarativeSchemaDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(
            deserialized.descriptive_name, None,
            "Descriptive name should remain None after deserialization"
        );
    }
}
