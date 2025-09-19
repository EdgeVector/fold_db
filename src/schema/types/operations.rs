use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub schema_name: String,
    pub fields: Vec<String>,
    pub pub_key: String,
    pub trust_distance: u32,
    pub filter: Option<Value>,
}

impl Query {
    #[must_use]
    pub fn new(
        schema_name: String,
        fields: Vec<String>,
        pub_key: String,
        trust_distance: u32,
    ) -> Self {
        Self {
            schema_name,
            fields,
            pub_key,
            trust_distance,
            filter: None,
        }
    }

    #[must_use]
    pub fn new_with_filter(
        schema_name: String,
        fields: Vec<String>,
        pub_key: String,
        trust_distance: u32,
        filter: Option<Value>,
    ) -> Self {
        Self {
            schema_name,
            fields,
            pub_key,
            trust_distance,
            filter,
        }
    }
}

#[derive(Debug, Clone, Serialize, ValueEnum)]
pub enum MutationType {
    Create,
    Update,
    Delete,
    #[clap(skip)]
    AddToCollection(String),
    #[clap(skip)]
    UpdateToCollection(String),
    #[clap(skip)]
    DeleteFromCollection(String),
}

impl<'de> Deserialize<'de> for MutationType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "create" => Ok(MutationType::Create),
            "update" => Ok(MutationType::Update),
            "delete" => Ok(MutationType::Delete),
            s if s.starts_with("add_to_collection:") => Err(serde::de::Error::custom(format!(
                "Collection operations are no longer supported: {}",
                s
            ))),
            s if s.starts_with("update_to_collection:") => {
                let id = s.split(':').nth(1).unwrap_or_default().to_string();
                Ok(MutationType::UpdateToCollection(id))
            }
            s if s.starts_with("delete_from_collection:") => {
                let id = s.split(':').nth(1).unwrap_or_default().to_string();
                Ok(MutationType::DeleteFromCollection(id))
            }
            _ => Err(serde::de::Error::custom("unknown mutation type")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mutation {
    pub schema_name: String,
    pub fields_and_values: HashMap<String, Value>,
    pub pub_key: String,
    pub trust_distance: u32,
    pub mutation_type: MutationType,
    pub synchronous: Option<bool>,
}

impl Mutation {
    #[must_use]
    pub const fn new(
        schema_name: String,
        fields_and_values: HashMap<String, Value>,
        pub_key: String,
        trust_distance: u32,
        mutation_type: MutationType,
    ) -> Self {
        Self {
            schema_name,
            fields_and_values,
            pub_key,
            trust_distance,
            mutation_type,
            synchronous: None,
        }
    }

    /// **RANGE SCHEMA MUTATION FIX: MoleculeRange Key Standardization**
    ///
    /// This method fixes a critical bug in range schema processing where MoleculeRange keys
    /// were inconsistent - sometimes using field names, sometimes using range_key values.
    ///
    /// ## The Problem That Was Solved
    ///
    /// Before this fix, range schema mutations created inconsistent MoleculeRange keys:
    /// - Range key field ("abc") would create MoleculeRange with key = field name
    /// - Non-range key field ({"test_id": "abc", "value": "123"}) would create MoleculeRange with key = field name
    ///
    /// This caused major issues:
    /// - Queries couldn't find data because keys were field names instead of range_key values
    /// - Range filtering failed because it expected keys to be range_key values
    /// - Data isolation was broken - different range_key values weren't properly separated
    ///
    /// ## The Solution: Standardize All Keys to Range Key Values
    ///
    /// This method transforms ALL fields so their MoleculeRange keys will ALWAYS be the
    /// range_key VALUE ("abc"), never field names. This ensures:
    /// - Consistent key structure across all range fields
    /// - Proper data isolation by range_key value
    /// - Correct query and filtering behavior
    ///
    /// ## Transformation Examples
    ///
    /// **Range key field transformation:**
    /// ```text
    /// Input:  "user_id": "abc"
    /// Output: "user_id": {"abc": "abc"}
    /// Result: MoleculeRange key = "abc" (the range_key VALUE)
    /// ```
    ///
    /// **Non-range key field transformation:**
    /// ```text
    /// Input:  "score": {"test_id": "abc", "value": "123"}
    /// Output: "score": {"abc": {"test_id": "abc", "value": "123"}}
    /// Result: MoleculeRange key = "abc" (the range_key VALUE, not "score")
    /// ```
    ///
    /// This transformation happens BEFORE the field_manager processes the mutation,
    /// ensuring that field_manager remains completely agnostic about range schema logic
    /// and only needs to handle standard field type processing.
    ///
    /// ## Validation
    ///
    /// This method ensures that:
    /// - The mutation contains the required range_key field
    /// - The range_key value is valid (not null or empty)
    /// - All fields get properly transformed for consistent MoleculeRange key structure
    pub fn to_range_schema_mutation(
        &self,
        schema: &crate::schema::types::Schema,
    ) -> Result<Self, crate::schema::types::SchemaError> {
        use serde_json::Value;
        if let Some(range_key) = schema.range_key() {
            let (range_key_value, range_key_str) =
                Self::validate_range_key(&self.schema_name, range_key, &self.fields_and_values)?;

            let mut new_fields_and_values = self.fields_and_values.clone();

            for (field_name, value) in new_fields_and_values.iter_mut() {
                let original_value = value.clone();
                if field_name == range_key {
                    *value = Self::transform_range_key_field(&original_value, &range_key_str);
                } else {
                    *value = Self::map_non_range_field(
                        &original_value,
                        range_key,
                        &range_key_str,
                        range_key_value,
                    );
                }
            }

            Ok(Self {
                schema_name: self.schema_name.clone(),
                fields_and_values: new_fields_and_values,
                pub_key: self.pub_key.clone(),
                trust_distance: self.trust_distance,
                mutation_type: self.mutation_type.clone(),
                synchronous: self.synchronous,
            })
        } else {
            Err(crate::schema::types::SchemaError::InvalidData(format!(
                "Schema '{}' is not a RangeSchema. Only range schemas support range_key propagation.",
                self.schema_name
            )))
        }
    }

    fn validate_range_key<'a>(
        schema_name: &str,
        range_key: &str,
        fields_and_values: &'a HashMap<String, Value>,
    ) -> Result<(&'a Value, String), crate::schema::types::SchemaError> {
        let range_key_value = fields_and_values.get(range_key).ok_or_else(|| {
            crate::schema::types::SchemaError::InvalidData(format!(
                "Range schema mutation for '{}' is missing required range_key field '{}'. All range schema mutations must provide a value for the range_key.",
                schema_name, range_key
            ))
        })?;

        if range_key_value.is_null() {
            return Err(crate::schema::types::SchemaError::InvalidData(format!(
                "Range schema mutation for '{}' has null value for range_key field '{}'. Range key must have a valid value.",
                schema_name, range_key
            )));
        }

        if let Some(str_value) = range_key_value.as_str() {
            if str_value.trim().is_empty() {
                return Err(crate::schema::types::SchemaError::InvalidData(format!(
                    "Range schema mutation for '{}' has empty string value for range_key field '{}'. Range key must have a non-empty value.",
                    schema_name, range_key
                )));
            }
        }

        let range_key_str = match range_key_value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => serde_json::to_string(range_key_value)
                .map_err(|e| crate::schema::types::SchemaError::InvalidData(e.to_string()))?
                .trim_matches('"')
                .to_string(),
        };

        Ok((range_key_value, range_key_str))
    }

    fn transform_range_key_field(original_value: &Value, range_key_str: &str) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert(range_key_str.to_string(), original_value.clone());
        Value::Object(obj)
    }

    fn map_non_range_field(
        original_value: &Value,
        range_key: &str,
        range_key_str: &str,
        range_key_value: &Value,
    ) -> Value {
        let field_content = if let Some(obj) = original_value.as_object() {
            if obj.contains_key(range_key) {
                let mut extracted_content = obj.clone();
                extracted_content.remove(range_key);
                if extracted_content.len() == 1 {
                    extracted_content.values().next().unwrap().clone()
                } else if extracted_content.is_empty() {
                    range_key_value.clone()
                } else {
                    serde_json::Value::Object(extracted_content)
                }
            } else {
                original_value.clone()
            }
        } else {
            original_value.clone()
        };

        let mut obj = serde_json::Map::new();
        obj.insert(range_key_str.to_string(), field_content);
        serde_json::Value::Object(obj)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fees::types::config::FieldPaymentConfig;
    use crate::permissions::types::policy::PermissionsPolicy;
    use crate::schema::types::field::{FieldVariant, RangeField};
    use crate::schema::types::operations::MutationType;
    use crate::schema::types::Schema;
    use crate::schema::types::SchemaError;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_to_range_schema_mutation_populates_range_key() {
        // Create a RangeSchema with range_key "user_id" and two fields
        let mut schema = Schema::new_range("TestRange".to_string(), "user_id".to_string());
        let rf = RangeField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        schema
            .fields
            .insert("user_id".to_string(), FieldVariant::Range(rf.clone()));
        schema
            .fields
            .insert("score".to_string(), FieldVariant::Range(rf));

        // Create a mutation with user_id and score fields
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("user_id".to_string(), json!(123));
        fields_and_values.insert("score".to_string(), json!({"points": 42}));

        let mutation = Mutation {
            schema_name: "TestRange".to_string(),
            fields_and_values,
            pub_key: "test".to_string(),
            trust_distance: 0,
            mutation_type: MutationType::Create,
            synchronous: None,
        };

        let result = mutation.to_range_schema_mutation(&schema).unwrap();
        // The score field should now be wrapped with the range_key value as key: {"123": {"points": 42}}
        let score_val = result.fields_and_values.get("score").unwrap();
        assert_eq!(score_val.get("123").unwrap(), &json!({"points": 42}));
        // The user_id field should now be wrapped: {"123": 123}
        let user_id_val = result.fields_and_values.get("user_id").unwrap();
        assert_eq!(user_id_val.get("123").unwrap(), &json!(123));
    }

    #[test]
    fn test_to_range_schema_mutation_missing_key() {
        // Create a RangeSchema with range_key "user_id"
        let mut schema = Schema::new_range("TestRange".to_string(), "user_id".to_string());
        let rf = RangeField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        schema
            .fields
            .insert("user_id".to_string(), FieldVariant::Range(rf.clone()));
        schema
            .fields
            .insert("score".to_string(), FieldVariant::Range(rf));

        // Create a mutation missing the user_id field
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("score".to_string(), json!({"points": 42}));

        let mutation = Mutation {
            schema_name: "TestRange".to_string(),
            fields_and_values,
            pub_key: "test".to_string(),
            trust_distance: 0,
            mutation_type: MutationType::Create,
            synchronous: None,
        };

        let result = mutation.to_range_schema_mutation(&schema);
        assert!(matches!(result, Err(SchemaError::InvalidData(_))));

        // Verify the error message mentions the missing range_key requirement
        if let Err(SchemaError::InvalidData(msg)) = result {
            assert!(msg.contains("missing required range_key field"));
            assert!(msg.contains("user_id"));
        }
    }

    #[test]
    fn test_to_range_schema_mutation_null_range_key() {
        // Create a RangeSchema with range_key "user_id"
        let mut schema = Schema::new_range("TestRange".to_string(), "user_id".to_string());
        let rf = RangeField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        schema
            .fields
            .insert("user_id".to_string(), FieldVariant::Range(rf.clone()));
        schema
            .fields
            .insert("score".to_string(), FieldVariant::Range(rf));

        // Create a mutation with null user_id field
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("user_id".to_string(), json!(null));
        fields_and_values.insert("score".to_string(), json!({"points": 42}));

        let mutation = Mutation {
            schema_name: "TestRange".to_string(),
            fields_and_values,
            pub_key: "test".to_string(),
            trust_distance: 0,
            mutation_type: MutationType::Create,
            synchronous: None,
        };

        let result = mutation.to_range_schema_mutation(&schema);
        assert!(matches!(result, Err(SchemaError::InvalidData(_))));

        // Verify the error message mentions null value
        if let Err(SchemaError::InvalidData(msg)) = result {
            assert!(msg.contains("null value for range_key field"));
        }
    }

    #[test]
    fn test_to_range_schema_mutation_empty_string_range_key() {
        // Create a RangeSchema with range_key "user_id"
        let mut schema = Schema::new_range("TestRange".to_string(), "user_id".to_string());
        let rf = RangeField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        );
        schema
            .fields
            .insert("user_id".to_string(), FieldVariant::Range(rf.clone()));
        schema
            .fields
            .insert("score".to_string(), FieldVariant::Range(rf));

        // Create a mutation with empty string user_id field
        let mut fields_and_values = HashMap::new();
        fields_and_values.insert("user_id".to_string(), json!(""));
        fields_and_values.insert("score".to_string(), json!({"points": 42}));

        let mutation = Mutation {
            schema_name: "TestRange".to_string(),
            fields_and_values,
            pub_key: "test".to_string(),
            trust_distance: 0,
            mutation_type: MutationType::Create,
            synchronous: None,
        };

        let result = mutation.to_range_schema_mutation(&schema);
        assert!(matches!(result, Err(SchemaError::InvalidData(_))));

        // Verify the error message mentions empty string value
        if let Err(SchemaError::InvalidData(msg)) = result {
            assert!(msg.contains("empty string value for range_key field"));
        }
    }

    #[test]
    fn test_validate_range_key_helpers() {
        let mut map = HashMap::new();
        map.insert("user_id".to_string(), json!(42));
        let (val, key_str) = Mutation::validate_range_key("Test", "user_id", &map).unwrap();
        assert_eq!(val, &json!(42));
        assert_eq!(key_str, "42");

        let res = Mutation::validate_range_key("Test", "missing", &map);
        assert!(matches!(res, Err(SchemaError::InvalidData(_))));
    }

    #[test]
    fn test_transform_range_key_field_helper() {
        let val = json!(7);
        let transformed = Mutation::transform_range_key_field(&val, "abc");
        assert_eq!(transformed, json!({"abc": 7}));
    }

    #[test]
    fn test_map_non_range_field_helper() {
        let value = json!({"user_id": "abc", "val": 5});
        let mapped = Mutation::map_non_range_field(&value, "user_id", "abc", &json!("abc"));
        assert_eq!(mapped, json!({"abc": 5}));
    }
}
