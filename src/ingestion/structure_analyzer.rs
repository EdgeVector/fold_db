//! Structure analysis utilities for JSON ingestion
//!
//! This module provides utilities to analyze JSON structure and create supersets
//! that capture all possible fields across multiple objects.

use serde_json::Value;
use std::collections::HashMap;

/// Analyzes JSON structure and creates a superset representation
/// that includes all fields found across all top-level elements
pub struct StructureAnalyzer;

impl StructureAnalyzer {
    /// Analyze JSON data and create a superset structure
    /// 
    /// For arrays, this loops through all elements and creates a superset
    /// that includes all fields found across all objects.
    /// For single objects, it returns the object as-is.
    /// 
    /// # Arguments
    /// * `json_data` - The JSON data to analyze
    /// 
    /// # Returns
    /// * `Value` - A superset structure representing all possible fields
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use serde_json::json;
    /// use datafold::ingestion::structure_analyzer::StructureAnalyzer;
    /// 
    /// let data = json!([
    ///     {"name": "Alice", "age": 30},
    ///     {"name": "Bob", "email": "bob@example.com"}
    /// ]);
    /// 
    /// let superset = StructureAnalyzer::create_superset_structure(&data);
    /// // Result: {"name": "string", "age": "number", "email": "string"}
    /// ```
    pub fn create_superset_structure(json_data: &Value) -> Value {
        match json_data {
            Value::Array(array) => {
                if array.is_empty() {
                    return Value::Object(serde_json::Map::new());
                }
                
                // Collect all unique fields and their types from all objects
                let mut field_types: HashMap<String, Vec<String>> = HashMap::new();
                
                for item in array {
                    if let Some(obj) = item.as_object() {
                        for (key, value) in obj {
                            let type_name = Self::get_value_type(value);
                            field_types
                                .entry(key.clone())
                                .or_default()
                                .push(type_name);
                        }
                    }
                }
                
                // Create superset object with all fields
                let mut superset = serde_json::Map::new();
                for (field_name, types) in field_types {
                    // Use the most specific type if there are multiple types
                    let representative_type = Self::get_representative_type(&types);
                    
                    // If the field is an object or array, analyze its nested structure
                    if representative_type == "object" || representative_type == "array" {
                        // Collect all instances of this field to analyze nested structure
                        let mut nested_values = Vec::new();
                        for item in array {
                            if let Some(obj) = item.as_object() {
                                if let Some(value) = obj.get(&field_name) {
                                    nested_values.push(value.clone());
                                }
                            }
                        }
                        
                        // Create superset structure for nested data
                        let nested_structure = Self::create_nested_superset(&nested_values);
                        superset.insert(field_name, nested_structure);
                    } else {
                        superset.insert(field_name, Value::String(representative_type));
                    }
                }
                
                Value::Object(superset)
            }
            Value::Object(obj) => {
                // For single objects, create a structure with field types
                let mut structure = serde_json::Map::new();
                for (key, value) in obj {
                    let type_name = Self::get_value_type(value);
                    structure.insert(key.clone(), Value::String(type_name));
                }
                Value::Object(structure)
            }
            _ => {
                // For primitive values, return a simple type representation
                Value::Object({
                    let mut map = serde_json::Map::new();
                    map.insert("value".to_string(), Value::String(Self::get_value_type(json_data)));
                    map
                })
            }
        }
    }
    
    /// Get the JSON type of a value
    fn get_value_type(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(_) => "boolean".to_string(),
            Value::Number(_) => "number".to_string(),
            Value::String(_) => "string".to_string(),
            Value::Array(_) => "array".to_string(),
            Value::Object(_) => "object".to_string(),
        }
    }
    
    /// Create superset structure for nested data (objects or arrays)
    fn create_nested_superset(values: &[Value]) -> Value {
        if values.is_empty() {
            return Value::String("null".to_string());
        }
        
        // Check if all values are objects
        if values.iter().all(|v| v.is_object()) {
            // Create superset for nested objects
            let mut field_types: HashMap<String, Vec<String>> = HashMap::new();
            
            for value in values {
                if let Some(obj) = value.as_object() {
                    for (key, val) in obj {
                        let type_name = Self::get_value_type(val);
                        field_types
                            .entry(key.clone())
                            .or_default()
                            .push(type_name);
                    }
                }
            }
            
            // Create superset object with all fields
            let mut superset = serde_json::Map::new();
            for (field_name, types) in field_types {
                let representative_type = Self::get_representative_type(&types);
                
                // If the field is an object, analyze its nested structure
                if representative_type == "object" {
                    // Collect all instances of this field to analyze nested structure
                    let mut nested_values = Vec::new();
                    for value in values {
                        if let Some(obj) = value.as_object() {
                            if let Some(nested_value) = obj.get(&field_name) {
                                nested_values.push(nested_value.clone());
                            }
                        }
                    }
                    
                    // Create superset structure for nested data
                    let nested_structure = Self::create_nested_superset(&nested_values);
                    superset.insert(field_name, nested_structure);
                } else {
                    superset.insert(field_name, Value::String(representative_type));
                }
            }
            
            Value::Object(superset)
        } else if values.iter().all(|v| v.is_array()) {
            // Handle arrays of arrays - analyze element types
            let mut element_types: HashMap<String, usize> = HashMap::new();
            
            for array_value in values {
                if let Some(array) = array_value.as_array() {
                    for item in array {
                        let type_name = Self::get_value_type(item);
                        *element_types.entry(type_name).or_insert(0) += 1;
                    }
                }
            }
            
            let most_common_type = element_types
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(type_name, _)| type_name)
                .unwrap_or_else(|| "string".to_string());
            
            Value::String(format!("array[{}]", most_common_type))
        } else {
            // Mixed types or primitive arrays - use most common type
            let mut type_counts: HashMap<String, usize> = HashMap::new();
            for value in values {
                let type_name = Self::get_value_type(value);
                *type_counts.entry(type_name).or_insert(0) += 1;
            }
            
            let most_common_type = type_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(type_name, _)| type_name)
                .unwrap_or_else(|| "string".to_string());
            
            Value::String(most_common_type)
        }
    }
    
    /// Get the most representative type from a list of types
    /// 
    /// This handles cases where a field has different types across objects.
    /// The priority is: object > array > string > number > boolean > null
    fn get_representative_type(types: &[String]) -> String {
        if types.is_empty() {
            return "null".to_string();
        }
        
        // Count occurrences of each type
        let mut type_counts: HashMap<String, usize> = HashMap::new();
        for type_name in types {
            *type_counts.entry(type_name.clone()).or_insert(0) += 1;
        }
        
        // Priority order for type selection
        let priority_order = vec!["object", "array", "string", "number", "boolean", "null"];
        
        // Find the highest priority type that exists
        for priority_type in priority_order {
            if type_counts.contains_key(priority_type) {
                return priority_type.to_string();
            }
        }
        
        // Fallback to the most common type
        type_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(type_name, _)| type_name)
            .unwrap_or_else(|| "null".to_string())
    }
    
    /// Get statistics about the structure analysis
    /// 
    /// Returns information about the number of elements analyzed,
    /// unique fields found, and type variations.
    pub fn get_analysis_stats(json_data: &Value) -> StructureStats {
        match json_data {
            Value::Array(array) => {
                let mut field_counts: HashMap<String, usize> = HashMap::new();
                let mut type_variations: HashMap<String, HashMap<String, usize>> = HashMap::new();
                
                for item in array {
                    if let Some(obj) = item.as_object() {
                        for (key, value) in obj {
                            let type_name = Self::get_value_type(value);
                            
                            // Count field occurrences
                            *field_counts.entry(key.clone()).or_insert(0) += 1;
                            
                            // Track type variations per field
                            type_variations
                                .entry(key.clone())
                                .or_default()
                                .entry(type_name)
                                .and_modify(|count| *count += 1)
                                .or_insert(1);
                        }
                    }
                }
                
                StructureStats {
                    total_elements: array.len(),
                    unique_fields: field_counts.len(),
                    field_counts,
                    type_variations,
                }
            }
            Value::Object(obj) => {
                let mut field_counts: HashMap<String, usize> = HashMap::new();
                let mut type_variations: HashMap<String, HashMap<String, usize>> = HashMap::new();
                
                for (key, value) in obj {
                    let type_name = Self::get_value_type(value);
                    field_counts.insert(key.clone(), 1);
                    type_variations.insert(key.clone(), {
                        let mut map = HashMap::new();
                        map.insert(type_name, 1);
                        map
                    });
                }
                
                StructureStats {
                    total_elements: 1,
                    unique_fields: obj.len(),
                    field_counts,
                    type_variations,
                }
            }
            _ => StructureStats {
                total_elements: 1,
                unique_fields: 1,
                field_counts: {
                    let mut map = HashMap::new();
                    map.insert("value".to_string(), 1);
                    map
                },
                type_variations: {
                    let mut map = HashMap::new();
                    map.insert("value".to_string(), {
                        let mut type_map = HashMap::new();
                        type_map.insert(Self::get_value_type(json_data), 1);
                        type_map
                    });
                    map
                },
            }
        }
    }
}

/// Statistics about structure analysis
#[derive(Debug, Clone)]
pub struct StructureStats {
    /// Total number of elements analyzed
    pub total_elements: usize,
    /// Number of unique fields found
    pub unique_fields: usize,
    /// Count of occurrences for each field
    pub field_counts: HashMap<String, usize>,
    /// Type variations for each field
    pub type_variations: HashMap<String, HashMap<String, usize>>,
}

impl StructureStats {
    /// Get fields that appear in all elements (100% coverage)
    pub fn get_common_fields(&self) -> Vec<String> {
        self.field_counts
            .iter()
            .filter(|(_, &count)| count == self.total_elements)
            .map(|(field, _)| field.clone())
            .collect()
    }
    
    /// Get fields that appear in some but not all elements (partial coverage)
    pub fn get_partial_fields(&self) -> Vec<String> {
        self.field_counts
            .iter()
            .filter(|(_, &count)| count > 0 && count < self.total_elements)
            .map(|(field, _)| field.clone())
            .collect()
    }
    
    /// Get fields with type variations
    pub fn get_fields_with_type_variations(&self) -> Vec<String> {
        self.type_variations
            .iter()
            .filter(|(_, variations)| variations.len() > 1)
            .map(|(field, _)| field.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_superset_structure_array() {
        let data = json!([
            {"name": "Alice", "age": 30},
            {"name": "Bob", "email": "bob@example.com"},
            {"name": "Charlie", "age": 25, "email": "charlie@example.com"}
        ]);
        
        let superset = StructureAnalyzer::create_superset_structure(&data);
        
        assert!(superset.is_object());
        let obj = superset.as_object().unwrap();
        assert!(obj.contains_key("name"));
        assert!(obj.contains_key("age"));
        assert!(obj.contains_key("email"));
        assert_eq!(obj["name"], "string");
        assert_eq!(obj["age"], "number");
        assert_eq!(obj["email"], "string");
    }

    #[test]
    fn test_create_superset_structure_single_object() {
        let data = json!({"name": "Alice", "age": 30});
        
        let superset = StructureAnalyzer::create_superset_structure(&data);
        
        assert!(superset.is_object());
        let obj = superset.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert_eq!(obj["name"], "string");
        assert_eq!(obj["age"], "number");
    }

    #[test]
    fn test_create_superset_structure_empty_array() {
        let data = json!([]);
        
        let superset = StructureAnalyzer::create_superset_structure(&data);
        
        assert!(superset.is_object());
        assert!(superset.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_type_variations() {
        let data = json!([
            {"id": 1, "name": "Alice"},
            {"id": "2", "name": "Bob"},
            {"id": 3, "name": "Charlie"}
        ]);
        
        let superset = StructureAnalyzer::create_superset_structure(&data);
        let obj = superset.as_object().unwrap();
        
        // Should choose "string" as representative type for "id" field (higher priority than number)
        assert_eq!(obj["id"], "string");
        assert_eq!(obj["name"], "string");
    }

    #[test]
    fn test_nested_structure_detection() {
        let data = json!([
            {"name": "Alice", "profile": {"age": 30, "department": "Engineering"}, "tags": ["senior", "backend"]},
            {"name": "Bob", "profile": {"email": "bob@example.com", "role": "Manager"}, "tags": ["lead"]},
            {"name": "Charlie", "profile": {"age": 25, "email": "charlie@example.com", "department": "Marketing"}, "tags": ["junior", "frontend"]}
        ]);
        
        let superset = StructureAnalyzer::create_superset_structure(&data);
        let obj = superset.as_object().unwrap();
        
        // Check top-level fields
        assert_eq!(obj["name"], "string");
        
        // Check nested object structure
        assert!(obj.contains_key("profile"));
        let profile = obj["profile"].as_object().unwrap();
        assert!(profile.contains_key("age"));
        assert!(profile.contains_key("department"));
        assert!(profile.contains_key("email"));
        assert!(profile.contains_key("role"));
        assert_eq!(profile["age"], "number");
        assert_eq!(profile["department"], "string");
        assert_eq!(profile["email"], "string");
        assert_eq!(profile["role"], "string");
        
        // Check array structure
        assert_eq!(obj["tags"], "array[string]");
    }
}
