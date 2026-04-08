use crate::schema::types::data_classification::DataClassification;
use crate::schema::types::field::Field;
use crate::schema::types::key_config::KeyConfig;
use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
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

        // Split on first dot only — field names may contain dots (e.g. "hash.policy.number"
        // means schema="hash", field="policy.number")
        let (source_schema, source_field) = trimmed
            .split_once('.')
            .ok_or_else(|| "FieldMapper must be in 'schema.field' format".to_string())?;

        let source_schema = source_schema.trim();
        let source_field = source_field.trim();

        if source_schema.is_empty() || source_field.is_empty() {
            return Err("FieldMapper must include non-empty source schema and field".to_string());
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
            field_classifications: HashMap<String, Vec<String>>,
            #[serde(default)]
            field_descriptions: HashMap<String, String>,
            #[serde(default)]
            field_data_classifications: HashMap<String, DataClassification>,
            #[serde(default)]
            field_interest_categories: HashMap<String, String>,
            #[serde(default)]
            ref_fields: HashMap<String, String>,
            #[serde(default)]
            field_types: HashMap<String, crate::schema::types::field_value_type::FieldValueType>,
            #[serde(skip_serializing_if = "Option::is_none")]
            identity_hash: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none", default)]
            org_hash: Option<String>,
            #[serde(default)]
            field_access_policies: HashMap<String, crate::access::types::FieldAccessPolicy>,
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
                } else if has_hash {
                    SchemaType::Hash
                } else if has_range {
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

        // Merge classifications from helper
        for (field_name, classifications) in helper.field_classifications {
            schema
                .field_classifications
                .insert(field_name, classifications);
        }

        // Preserve field_descriptions, field_data_classifications, field_interest_categories, ref_fields, field_types and identity_hash
        schema.field_descriptions = helper.field_descriptions;
        schema.field_data_classifications = helper.field_data_classifications;
        schema.field_interest_categories = helper.field_interest_categories;
        schema.ref_fields = helper.ref_fields;
        schema.field_types = helper.field_types;
        schema.identity_hash = helper.identity_hash;
        schema.org_hash = helper.org_hash;
        schema.field_access_policies = helper.field_access_policies;

        Ok(schema)
    }
}

/// Declarative schema definition - the primary schema representation.
/// This is the unified schema type that replaces the old Schema/DeclarativeSchemaDefinition split.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(
        export,
        export_to = "bindings/src/fold_node/static-react/src/types/generated.ts"
    )
)]
pub struct DeclarativeSchemaDefinition {
    /// Schema name
    pub name: String,
    /// Human-readable descriptive name for the schema (used in AI-generated proposals)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptive_name: Option<String>,
    /// Schema type ("Single" | "Hash" | "Range" | "HashRange")
    pub schema_type: SchemaType,
    /// Key configuration (required when schema_type == "Hash", "Range", or "HashRange")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<KeyConfig>,
    /// Field names - plain data fields without transformations
    // #[serde(skip_serializing_if = "Option::is_none")]
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
    /// Classification tags for each field (e.g. "word", "name:person", "date", "number")
    /// Maps field_name -> list of classification strings
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub field_classifications: HashMap<String, Vec<String>>,
    /// Natural language descriptions for each field (e.g. "the person who created the artwork")
    /// Maps field_name -> description string. Used for semantic field matching in the canonical registry.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub field_descriptions: HashMap<String, String>,
    /// Data classification labels for each field: (sensitivity_level, data_domain).
    /// Maps field_name -> DataClassification. Required for new fields at schema creation.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub field_data_classifications: HashMap<String, DataClassification>,
    /// Interest categories for each field (e.g. "Photography", "Cooking", "Running").
    /// Assigned by the schema service from the canonical field registry.
    /// Maps field_name -> interest category string.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub field_interest_categories: HashMap<String, String>,
    /// Reference fields that point to child schemas
    /// Maps field_name -> child_schema_name
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub ref_fields: HashMap<String, String>,
    /// Strongly typed field value types from the canonical field registry.
    /// Maps field_name -> FieldValueType. Fields not in this map default to Any.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub field_types: HashMap<String, crate::schema::types::field_value_type::FieldValueType>,
    /// SHA256 hash of sorted field names — unique fingerprint of schema structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_hash: Option<String>,
    /// If set, this schema has been superseded by the named schema.
    /// Superseded schemas are excluded from active indexes and matching.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub superseded_by: Option<String>,
    /// If set, this schema belongs to an organization and its molecules sync to all org members.
    /// Value is the hex-encoded SHA256 hash of the org's Ed25519 public key.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub org_hash: Option<String>,

    /// Default trust domain for all fields in this schema.
    /// If set, fields without an explicit `trust_domain` in their access policy
    /// inherit this value. For org schemas, auto-set to `org:{org_hash}`.
    /// Default: "personal".
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub trust_domain: Option<String>,

    /// Persisted per-field access policies. Survives serialization (unlike runtime_fields).
    /// When set, these are copied onto runtime fields during populate_runtime_fields().
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub field_access_policies: HashMap<String, crate::access::types::FieldAccessPolicy>,

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
            && self.field_classifications == other.field_classifications
            && self.field_data_classifications == other.field_data_classifications
            && self.field_interest_categories == other.field_interest_categories
            && self.ref_fields == other.ref_fields
            && self.field_types == other.field_types
            && self.identity_hash == other.identity_hash
            && self.superseded_by == other.superseded_by
            && self.org_hash == other.org_hash
            && self.field_access_policies == other.field_access_policies
        // Exclude runtime_fields, inputs_schema_fields, source_schemas, and hash mappings
        // These are derived/runtime state and don't affect schema identity
    }
}

impl DeclarativeSchemaDefinition {
    /// Populates runtime_fields from declarative schema definition
    /// This is called after deserializing from database to ensure runtime state is initialized
    /// Also regenerates transform metadata (hash mappings, inputs, source schemas) which are not persisted
    pub fn populate_runtime_fields(&mut self) -> Result<(), crate::schema::SchemaError> {
        use crate::schema::types::field::{
            FieldVariant, HashField, HashRangeField, RangeField, SingleField,
        };
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
                SchemaType::Hash => {
                    let hash_field = HashField::new(default_field_mappers.clone(), None);
                    runtime_fields.insert(field_name, FieldVariant::Hash(hash_field));
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

        // Restore persisted molecule UUIDs from field_molecule_uuids if available,
        // otherwise derive deterministically from schema name + field name.
        let persisted = self.field_molecule_uuids.clone().unwrap_or_default();
        for (field_name, field) in self.runtime_fields.iter_mut() {
            let mol_uuid = if let Some(uuid) = persisted.get(field_name) {
                uuid.clone()
            } else {
                crate::atom::deterministic_molecule_uuid(&self.name, field_name)
            };
            field.common_mut().set_molecule_uuid(mol_uuid);
        }

        // Propagate org_hash from the schema to each field's FieldCommon
        // so that field-level storage key construction includes the org prefix.
        if self.org_hash.is_some() {
            for field in self.runtime_fields.values_mut() {
                field.common_mut().set_org_hash(self.org_hash.clone());
            }
        }

        // Fields without policies remain None (legacy = no access checks).
        // Policies are set explicitly via set_field_access_policy or
        // apply_classification_defaults, and persisted in field_access_policies.

        // Apply persisted field access policies.
        // These are stored in field_access_policies and survive serialization,
        // unlike the policies on runtime_fields which are #[serde(skip)].
        for (field_name, policy) in &self.field_access_policies {
            if let Some(field) = self.runtime_fields.get_mut(field_name) {
                field.common_mut().access_policy = Some(policy.clone());
            }
        }

        // Regenerate transform metadata that isn't persisted (marked with #[serde(skip)])
        // This is needed when schemas are loaded from the database
        self.regenerate_metadata();

        Ok(())
    }

    /// Regenerate all derived transform metadata (hash mappings, inputs, source schemas).
    /// Called after construction and after deserialization from database.
    fn regenerate_metadata(&mut self) {
        self.generate_hash_to_code_mappings();
        self.generate_inputs();
        self.fetch_source_schemas();
    }

    /// Copies molecule UUIDs from runtime_fields into the persisted field_molecule_uuids map.
    /// Called after mutations so that the UUIDs survive serialization to DB.
    pub fn sync_molecule_uuids(&mut self) {
        let mut uuids = HashMap::new();
        for (field_name, field) in &self.runtime_fields {
            if let Some(uuid) = field.common().molecule_uuid() {
                uuids.insert(field_name.clone(), uuid.clone());
            }
        }
        if !uuids.is_empty() {
            self.field_molecule_uuids = Some(uuids);
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
            field_classifications: HashMap::new(),
            field_descriptions: HashMap::new(),
            field_data_classifications: HashMap::new(),
            field_interest_categories: HashMap::new(),
            ref_fields: HashMap::new(),
            field_types: HashMap::new(),
            identity_hash: None,
            superseded_by: None,
            org_hash: None,
            trust_domain: None,
            field_access_policies: HashMap::new(),
            runtime_fields: HashMap::new(),
            inputs_schema_fields: Vec::new(),
            source_schemas: Vec::new(),
            field_to_hash_code: HashMap::new(),
            hash_to_code: HashMap::new(),
        };

        schema.regenerate_metadata();
        schema
    }

    /// Get the declared type for a field. Returns `Any` if no type is declared.
    pub fn get_field_type(
        &self,
        field_name: &str,
    ) -> &crate::schema::types::field_value_type::FieldValueType {
        static ANY: crate::schema::types::field_value_type::FieldValueType =
            crate::schema::types::field_value_type::FieldValueType::Any;
        self.field_types.get(field_name).unwrap_or(&ANY)
    }

    pub fn field_mappers(&self) -> Option<&HashMap<String, FieldMapper>> {
        self.field_mappers.as_ref()
    }

    /// Get classifications for a specific field
    pub fn get_field_classifications(&self, field_name: &str) -> Option<&Vec<String>> {
        self.field_classifications.get(field_name)
    }

    /// Compute identity hash from the readable name (descriptive_name) + sorted,
    /// deduplicated field names (SHA256).
    ///
    /// Uses descriptive_name (the human-readable label) rather than schema.name
    /// because schema.name may already be a hash from a previous expansion.
    /// descriptive_name stays stable across expansions, so:
    /// - Same readable name + same fields = same hash = dedup
    /// - Same readable name + different fields = different hash = separate schemas
    /// - Different readable name + same fields = different hash = separate schemas
    ///
    /// Falls back to schema.name if descriptive_name is not set.
    pub fn compute_identity_hash(&mut self) {
        let mut field_names: Vec<&str> = self
            .fields
            .as_ref()
            .map(|f| f.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();
        field_names.sort();
        field_names.dedup();
        let combined = field_names.join(",");
        let mut hasher = Sha256::new();
        // Use the readable name (descriptive_name preferred, falls back to name)
        let readable_name = self
            .descriptive_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.name);
        if !readable_name.is_empty() {
            hasher.update(readable_name.as_bytes());
            hasher.update(b":");
        }
        hasher.update(combined.as_bytes());
        self.identity_hash = Some(format!("{:x}", hasher.finalize()));
    }

    /// Deduplicate the fields list in-place, preserving order.
    pub fn dedup_fields(&mut self) {
        if let Some(ref mut fields) = self.fields {
            let mut seen = std::collections::HashSet::new();
            fields.retain(|f| seen.insert(f.clone()));
        }
    }

    /// Get the identity hash
    pub fn get_identity_hash(&self) -> Option<&String> {
        self.identity_hash.as_ref()
    }

    /// Parse a single transform expression into a "Schema.field" input reference.
    fn parse_expression_input(expression: &str) -> Option<String> {
        // Split by "." and filter out method calls containing "(" or ")"
        let parts: Vec<&str> = expression
            .split(".")
            .filter(|part| !part.contains("(") && !part.contains(")"))
            .collect();

        if parts.len() >= 2 {
            Some(format!("{}.{}", parts[0], parts[1]))
        } else if parts.len() == 1 {
            Some(parts[0].to_string())
        } else {
            None
        }
    }

    fn generate_inputs(&mut self) {
        let mut inputs_schema_fields: Vec<String> = self
            .hash_to_code
            .values()
            .filter_map(|expr| Self::parse_expression_input(expr))
            .collect();
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

    /// Generates hash-to-code mappings for all keys and fields in the declarative schema.
    ///
    /// This function hashes every line from the keys (hash_field and range_field) and every
    /// field expression (atom_uuid expressions) and stores them in the hash_to_code HashMap.
    ///
    /// # Adds mappings to field_to_hash_code and hash_to_code.
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

    /// Gets a reference to the hash-to-code mapping.
    pub fn hash_to_code(&self) -> &HashMap<String, String> {
        &self.hash_to_code
    }

    /// Extract input fields from a single transform expression.
    /// Example: "BlogPost.content.split_by_word()" -> ["BlogPost.content"]
    pub fn extract_inputs_from_expression(expression: &str) -> Vec<String> {
        Self::parse_expression_input(expression)
            .into_iter()
            .collect()
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

    #[test]
    fn test_field_access_policies_deserialization() {
        use crate::access::types::FieldAccessPolicy;

        // Build JSON with field_access_policies
        let json = serde_json::json!({
            "name": "SecureNotes",
            "schema_type": "Single",
            "fields": ["title", "body"],
            "field_access_policies": {
                "body": {
                    "trust_domain": "personal",
                    "trust_distance": { "read_max": 0, "write_max": 0 },
                    "capabilities": [],
                    "security_label": null
                }
            }
        });

        let mut schema: DeclarativeSchemaDefinition =
            serde_json::from_value(json).expect("should deserialize with field_access_policies");

        // The declarative field should be present before populate
        assert_eq!(schema.field_access_policies.len(), 1);
        assert!(schema.field_access_policies.contains_key("body"));

        // Populate runtime fields — this should copy policies onto runtime fields
        schema.populate_runtime_fields().unwrap();

        // Verify the runtime field got the policy
        let body_field = schema.runtime_fields.get("body").expect("body field should exist");
        let policy = body_field
            .common()
            .access_policy
            .as_ref()
            .expect("body should have an access policy after populate");
        assert_eq!(policy.trust_domain, "personal");
        assert_eq!(policy.trust_distance.read_max, 0);
        assert_eq!(policy.trust_distance.write_max, 0);

        // title should NOT have a policy (not in field_access_policies)
        let title_field = schema.runtime_fields.get("title").expect("title field should exist");
        assert!(
            title_field.common().access_policy.is_none(),
            "title should have no access policy"
        );

        // Verify PartialEq includes field_access_policies
        let mut schema2 = schema.clone();
        assert_eq!(schema, schema2, "cloned schemas should be equal");
        schema2.field_access_policies.insert(
            "title".to_string(),
            FieldAccessPolicy::default(),
        );
        assert_ne!(
            schema, schema2,
            "schemas with different field_access_policies should not be equal"
        );
    }
}
