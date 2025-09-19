use crate::schema::types::SchemaError;
use log::{error, info, warn};
use serde_json::Value as JsonValue;

use super::TransformUtils;

impl TransformUtils {
    /// Convert JsonValue to Value with unified error handling
    pub fn json_to_value(json_value: JsonValue) -> Result<JsonValue, SchemaError> {
        Ok(json_value)
    }

    /// Validate and convert JsonValue with type checking
    pub fn validate_and_convert(
        json_value: JsonValue,
        expected_type: &str,
        field_name: &str,
    ) -> Result<JsonValue, SchemaError> {
        info!(
            "🔄 Converting field '{}' (expected type: {})",
            field_name, expected_type
        );

        let is_valid = match expected_type.to_lowercase().as_str() {
            "string" | "str" => json_value.is_string(),
            "number" | "integer" | "int" => json_value.is_number() && json_value.as_i64().is_some(),
            "float" | "double" => json_value.is_number() && json_value.as_f64().is_some(),
            "boolean" | "bool" => json_value.is_boolean(),
            "array" => json_value.is_array(),
            "object" => json_value.is_object(),
            "null" => json_value.is_null(),
            _ => {
                warn!(
                    "⚠️ Unknown expected type '{}' for field '{}', allowing any type",
                    expected_type, field_name
                );
                true
            }
        };

        if !is_valid {
            let error_msg = format!(
                "Type validation failed for field '{}': expected '{}', got '{:?}'",
                field_name, expected_type, json_value
            );
            error!("❌ {}", error_msg);
            return Err(SchemaError::InvalidData(error_msg));
        }

        info!(
            "✅ Successfully validated and converted field '{}'",
            field_name
        );
        Ok(json_value)
    }

    /// Convert string to JsonValue with type inference
    pub fn string_to_json_value(value_str: &str, infer_type: bool) -> JsonValue {
        if !infer_type {
            return JsonValue::String(value_str.to_string());
        }

        if value_str.eq_ignore_ascii_case("true") {
            return JsonValue::Bool(true);
        }
        if value_str.eq_ignore_ascii_case("false") {
            return JsonValue::Bool(false);
        }
        if value_str.eq_ignore_ascii_case("null") {
            return JsonValue::Null;
        }
        if let Ok(int_val) = value_str.parse::<i64>() {
            return JsonValue::Number(serde_json::Number::from(int_val));
        }
        if let Ok(float_val) = value_str.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(float_val) {
                return JsonValue::Number(num);
            }
        }
        JsonValue::String(value_str.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_to_value() {
        let json_val = json!({"key": "value"});
        let result = TransformUtils::json_to_value(json_val.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json_val);
    }

    #[test]
    fn test_validate_and_convert_string() {
        let json_val = JsonValue::String("test".to_string());
        let result = TransformUtils::validate_and_convert(json_val.clone(), "string", "test_field");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), json_val);
    }

    #[test]
    fn test_string_to_json_value_with_inference() {
        assert_eq!(
            TransformUtils::string_to_json_value("true", true),
            JsonValue::Bool(true)
        );
        assert_eq!(
            TransformUtils::string_to_json_value("42", true),
            JsonValue::Number(serde_json::Number::from(42))
        );
        assert_eq!(
            TransformUtils::string_to_json_value("hello", true),
            JsonValue::String("hello".to_string())
        );
    }
}
