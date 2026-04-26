//! Test utilities for building schemas with proper classifications.
//! Gated behind the `test-utils` cargo feature (or `#[cfg(test)]`).

use serde_json::json;
use std::collections::HashMap;

/// Builder for test schemas with automatic field classification.
/// Every field gets a DataClassification so schemas pass validation.
pub struct TestSchemaBuilder {
    name: String,
    descriptive_name: Option<String>,
    fields: Vec<String>,
    hash_field: Option<String>,
    range_field: Option<String>,
    sensitivity: u8,
    data_domain: String,
    field_classifications: HashMap<String, (u8, String)>,
    field_mappers: HashMap<String, String>,
    field_types: HashMap<String, serde_json::Value>,
    ref_fields: HashMap<String, String>,
    org_hash: Option<String>,
}

impl TestSchemaBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            descriptive_name: None,
            fields: Vec::new(),
            hash_field: None,
            range_field: None,
            sensitivity: 0,
            data_domain: "general".to_string(),
            field_classifications: HashMap::new(),
            field_mappers: HashMap::new(),
            field_types: HashMap::new(),
            ref_fields: HashMap::new(),
            org_hash: None,
        }
    }

    pub fn descriptive_name(mut self, name: &str) -> Self {
        self.descriptive_name = Some(name.to_string());
        self
    }

    pub fn field(mut self, name: &str) -> Self {
        if !self.fields.contains(&name.to_string()) {
            self.fields.push(name.to_string());
        }
        self
    }

    pub fn fields(mut self, names: &[&str]) -> Self {
        for name in names {
            if !self.fields.contains(&name.to_string()) {
                self.fields.push(name.to_string());
            }
        }
        self
    }

    pub fn range_key(mut self, field: &str) -> Self {
        self.range_field = Some(field.to_string());
        if !self.fields.contains(&field.to_string()) {
            self.fields.push(field.to_string());
        }
        self
    }

    pub fn hash_key(mut self, field: &str) -> Self {
        self.hash_field = Some(field.to_string());
        if !self.fields.contains(&field.to_string()) {
            self.fields.push(field.to_string());
        }
        self
    }

    /// Set default sensitivity for all fields (0=Public, 4=HighlyRestricted)
    pub fn sensitivity(mut self, level: u8) -> Self {
        self.sensitivity = level;
        self
    }

    /// Set default data domain for all fields
    pub fn domain(mut self, domain: &str) -> Self {
        self.data_domain = domain.to_string();
        self
    }

    /// Override classification for a specific field
    pub fn classify(mut self, field: &str, sensitivity: u8, domain: &str) -> Self {
        self.field_classifications
            .insert(field.to_string(), (sensitivity, domain.to_string()));
        self
    }

    /// Add a field mapper (e.g. "id" -> "User.id")
    pub fn field_mapper(mut self, field: &str, source: &str) -> Self {
        self.field_mappers
            .insert(field.to_string(), source.to_string());
        self
    }

    /// Add a typed field (e.g. "age" -> json!("Integer"))
    pub fn field_type(mut self, field: &str, typ: serde_json::Value) -> Self {
        self.field_types.insert(field.to_string(), typ);
        self
    }

    /// Add a ref field (e.g. "posts" -> "Post")
    pub fn ref_field(mut self, field: &str, target_schema: &str) -> Self {
        self.ref_fields
            .insert(field.to_string(), target_schema.to_string());
        self
    }

    /// Set org_hash for org schemas
    pub fn org_hash(mut self, hash: &str) -> Self {
        self.org_hash = Some(hash.to_string());
        self
    }

    /// Build the schema as a JSON string suitable for load_schema_from_json
    pub fn build_json(&self) -> String {
        let mut classifications = serde_json::Map::new();
        for field in &self.fields {
            let (sens, domain) = self
                .field_classifications
                .get(field)
                .cloned()
                .unwrap_or((self.sensitivity, self.data_domain.clone()));
            classifications.insert(
                field.clone(),
                json!({
                    "sensitivity_level": sens,
                    "data_domain": domain
                }),
            );
        }

        let mut key = serde_json::Map::new();
        if let Some(ref h) = self.hash_field {
            key.insert("hash_field".to_string(), json!(h));
        }
        if let Some(ref r) = self.range_field {
            key.insert("range_field".to_string(), json!(r));
        }

        let mut fields_map = serde_json::Map::new();
        for field in &self.fields {
            fields_map.insert(field.clone(), json!({}));
        }

        let mut schema = json!({
            "name": self.name,
            "fields": fields_map,
            "field_data_classifications": classifications,
        });

        if !key.is_empty() {
            schema
                .as_object_mut()
                .unwrap()
                .insert("key".to_string(), serde_json::Value::Object(key));
        }

        if let Some(ref dn) = self.descriptive_name {
            schema
                .as_object_mut()
                .unwrap()
                .insert("descriptive_name".to_string(), json!(dn));
        }

        if !self.field_mappers.is_empty() {
            schema
                .as_object_mut()
                .unwrap()
                .insert("field_mappers".to_string(), json!(self.field_mappers));
        }

        if !self.field_types.is_empty() {
            schema
                .as_object_mut()
                .unwrap()
                .insert("field_types".to_string(), json!(self.field_types));
        }

        if !self.ref_fields.is_empty() {
            schema
                .as_object_mut()
                .unwrap()
                .insert("ref_fields".to_string(), json!(self.ref_fields));
        }

        if let Some(ref org) = self.org_hash {
            schema
                .as_object_mut()
                .unwrap()
                .insert("org_hash".to_string(), json!(org));
        }

        serde_json::to_string_pretty(&schema).unwrap()
    }
}
