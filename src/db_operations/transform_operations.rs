use super::core::DbOperations;
use crate::schema::types::transform::Transform;
use crate::schema::SchemaError;
use std::collections::{BTreeMap, HashMap, HashSet};

const SCHEMA_FIELD_TO_TRANSFORMS_KEY: &str = "map_schema_field_to_transforms";

type TransformMap = HashMap<String, Transform>;
type FieldMappings = BTreeMap<String, HashSet<String>>;
type SyncResult = (TransformMap, FieldMappings);

impl DbOperations {
    /// Syncs transform state bidirectionally: merges in-memory and storage, then persists to both
    pub fn sync_transform_state(
        &self,
        in_memory_transforms: &TransformMap,
        in_memory_field_mappings: &FieldMappings,
    ) -> Result<SyncResult, SchemaError> {
        // 1. Load current state from storage
        let mut storage_transforms = HashMap::new();
        
        // Load all transforms from storage (inline list_transforms + get_transform)
        for result in self.transforms_tree.iter() {
            let (key, _) = result.map_err(|e| {
                SchemaError::InvalidData(format!("Failed to iterate transforms: {}", e))
            })?;
            let transform_id = String::from_utf8_lossy(&key).to_string();

            // Skip metadata keys
            if transform_id == SCHEMA_FIELD_TO_TRANSFORMS_KEY {
                continue;
            }

            if let Some(transform) = self.get_from_tree::<Transform>(&self.transforms_tree, &transform_id)? {
                storage_transforms.insert(transform_id, transform);
            }
        }

        // Load field-to-transforms mapping (inline load_field_to_transforms_mapping)
        let bytes = self.transforms_tree
            .get(SCHEMA_FIELD_TO_TRANSFORMS_KEY.as_bytes())
            .map_err(|e| SchemaError::InvalidData(format!("Failed to get transform mapping: {}", e)))?;

        let storage_field_mappings = match bytes {
            Some(data) => {
                deserialize_mapping(&data, "field_to_transforms")?
            }
            None => BTreeMap::new(),
        };

        // 2. Merge: union of keys from both sources, in-memory takes precedence for conflicts
        let mut merged_transforms = storage_transforms.clone();
        for (id, transform) in in_memory_transforms {
            merged_transforms.insert(id.clone(), transform.clone());
        }

        let mut merged_field_mappings = storage_field_mappings.clone();
        for (field, transforms) in in_memory_field_mappings {
            merged_field_mappings
                .entry(field.clone())
                .or_insert_with(HashSet::new)
                .extend(transforms.clone());
        }

        // 3. Persist merged state to storage
        for (transform_id, transform) in &merged_transforms {
            self.store_in_tree(&self.transforms_tree, transform_id, transform)?;
        }

        let mapping_bytes = serde_json::to_vec(&merged_field_mappings)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to serialize field mapping: {}", e)))?;

        self.transforms_tree
            .insert(SCHEMA_FIELD_TO_TRANSFORMS_KEY.as_bytes(), mapping_bytes)
            .map_err(|e| SchemaError::InvalidData(format!("Failed to store field mapping: {}", e)))?;

        self.transforms_tree
            .flush()
            .map_err(|e| SchemaError::InvalidData(format!("Failed to flush transforms tree: {}", e)))?;

        // 4. Return merged state for caller to update in-memory structures
        Ok((merged_transforms, merged_field_mappings))
    }
}

/// Deserialize a mapping of String -> Set<String> stored as JSON bytes.
fn deserialize_mapping(
    bytes: &[u8],
    context: &str,
) -> Result<BTreeMap<String, HashSet<String>>, SchemaError> {
    let parsed: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| {
        SchemaError::InvalidData(format!(
            "Failed to parse {} mapping as JSON: {}",
            context, e
        ))
    })?;

    // Accept either an object of arrays or object of sets; normalize to BTreeMap<String, HashSet<String>>
    match parsed {
        serde_json::Value::Object(map) => {
            let mut result: BTreeMap<String, HashSet<String>> = BTreeMap::new();
            for (key, value) in map.into_iter() {
                let set: HashSet<String> = match value {
                    serde_json::Value::Array(arr) => arr
                        .into_iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect(),
                    serde_json::Value::Null => HashSet::new(),
                    other => {
                        return Err(SchemaError::InvalidData(format!(
                            "Invalid value for {}.{}: expected array, got {}",
                            context,
                            key,
                            other
                        )))
                    }
                };
                result.insert(key, set);
            }
            Ok(result)
        }
        other => Err(SchemaError::InvalidData(format!(
            "Invalid {} mapping root: expected object, got {}",
            context, other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::DeclarativeSchemaDefinition;
    use std::collections::{BTreeMap, HashMap, HashSet};
    use tempfile::TempDir;

    fn create_test_db() -> (TempDir, DbOperations) {
        let temp_dir = TempDir::new().unwrap();
        let db = sled::open(temp_dir.path()).unwrap();
        let db_ops = DbOperations::new(db).unwrap();
        (temp_dir, db_ops)
    }

    fn create_test_transform(id: &str) -> Transform {
        let mut transform_fields = HashMap::new();
        transform_fields.insert("field1".to_string(), "input.value".to_string());
        
        let schema = DeclarativeSchemaDefinition::new(
            format!("TestSchema_{}", id),
            crate::schema::types::schema::SchemaType::Single,
            None,
            Some(vec!["field1".to_string()]),
            Some(transform_fields),
        );
        Transform::from_declarative_schema(schema)
    }

    #[test]
    fn test_sync_empty_to_empty() {
        let (_temp_dir, db_ops) = create_test_db();
        
        let empty_transforms = HashMap::new();
        let empty_mappings = BTreeMap::new();

        let result = db_ops.sync_transform_state(&empty_transforms, &empty_mappings);
        assert!(result.is_ok());
        
        let (merged_transforms, merged_mappings) = result.unwrap();
        assert_eq!(merged_transforms.len(), 0);
        assert_eq!(merged_mappings.len(), 0);
    }

    #[test]
    fn test_sync_loads_from_storage() {
        let (_temp_dir, db_ops) = create_test_db();
        
        // Pre-populate storage with a transform
        let transform1 = create_test_transform("1");
        db_ops.store_in_tree(&db_ops.transforms_tree, "transform_1", &transform1).unwrap();

        // Create field mapping in storage
        let mut storage_mappings: BTreeMap<String, HashSet<String>> = BTreeMap::new();
        storage_mappings.insert("Schema.field1".to_string(), 
            vec!["transform_1".to_string()].into_iter().collect());
        let mapping_bytes = serde_json::to_vec(&storage_mappings).unwrap();
        db_ops.transforms_tree.insert(b"map_schema_field_to_transforms", mapping_bytes).unwrap();
        db_ops.transforms_tree.flush().unwrap();

        // Sync with empty in-memory state
        let empty_transforms = HashMap::new();
        let empty_mappings = BTreeMap::new();

        let (merged_transforms, merged_mappings) = 
            db_ops.sync_transform_state(&empty_transforms, &empty_mappings).unwrap();

        assert_eq!(merged_transforms.len(), 1);
        assert!(merged_transforms.contains_key("transform_1"));
        assert_eq!(merged_mappings.len(), 1);
        assert!(merged_mappings.contains_key("Schema.field1"));
    }

    #[test]
    fn test_sync_writes_in_memory_to_storage() {
        let (_temp_dir, db_ops) = create_test_db();
        
        // Create in-memory state
        let mut in_memory_transforms = HashMap::new();
        let transform1 = create_test_transform("1");
        in_memory_transforms.insert("transform_1".to_string(), transform1);

        let mut in_memory_mappings = BTreeMap::new();
        let mut transform_set = HashSet::new();
        transform_set.insert("transform_1".to_string());
        in_memory_mappings.insert("Schema.field1".to_string(), transform_set);

        // Sync to empty storage
        let (merged_transforms, merged_mappings) = 
            db_ops.sync_transform_state(&in_memory_transforms, &in_memory_mappings).unwrap();

        assert_eq!(merged_transforms.len(), 1);
        assert_eq!(merged_mappings.len(), 1);

        // Verify it was written to storage by loading with empty in-memory
        let empty_transforms = HashMap::new();
        let empty_mappings = BTreeMap::new();
        let (loaded_transforms, loaded_mappings) = 
            db_ops.sync_transform_state(&empty_transforms, &empty_mappings).unwrap();

        assert_eq!(loaded_transforms.len(), 1);
        assert!(loaded_transforms.contains_key("transform_1"));
        assert_eq!(loaded_mappings.len(), 1);
        assert!(loaded_mappings.contains_key("Schema.field1"));
    }

    #[test]
    fn test_sync_merges_from_both_sources() {
        let (_temp_dir, db_ops) = create_test_db();
        
        // Put transform_1 in storage
        let transform1 = create_test_transform("1");
        db_ops.store_in_tree(&db_ops.transforms_tree, "transform_1", &transform1).unwrap();

        // Put transform_2 in memory
        let mut in_memory_transforms = HashMap::new();
        let transform2 = create_test_transform("2");
        in_memory_transforms.insert("transform_2".to_string(), transform2);

        let empty_mappings = BTreeMap::new();

        let (merged_transforms, _) = 
            db_ops.sync_transform_state(&in_memory_transforms, &empty_mappings).unwrap();

        // Should have both transforms
        assert_eq!(merged_transforms.len(), 2);
        assert!(merged_transforms.contains_key("transform_1"));
        assert!(merged_transforms.contains_key("transform_2"));
    }

    #[test]
    fn test_sync_in_memory_takes_precedence() {
        let (_temp_dir, db_ops) = create_test_db();
        
        // Put transform_1 version A in storage
        let transform_a = create_test_transform("A");
        db_ops.store_in_tree(&db_ops.transforms_tree, "transform_1", &transform_a).unwrap();

        // Put transform_1 version B in memory
        let mut in_memory_transforms = HashMap::new();
        let transform_b = create_test_transform("B");
        in_memory_transforms.insert("transform_1".to_string(), transform_b.clone());

        let empty_mappings = BTreeMap::new();

        let (merged_transforms, _) = 
            db_ops.sync_transform_state(&in_memory_transforms, &empty_mappings).unwrap();

        // Should have in-memory version (B)
        assert_eq!(merged_transforms.len(), 1);
        let merged_transform = merged_transforms.get("transform_1").unwrap();
        
        let merged_schema = merged_transform.get_declarative_schema().unwrap();
        let expected_schema = transform_b.get_declarative_schema().unwrap();
        assert_eq!(merged_schema.name, expected_schema.name);
        assert_eq!(merged_schema.name, "TestSchema_B");
    }

    #[test]
    fn test_sync_merges_field_mappings() {
        let (_temp_dir, db_ops) = create_test_db();
        
        // Storage has field1 -> [transform_1]
        let mut storage_mappings: BTreeMap<String, HashSet<String>> = BTreeMap::new();
        storage_mappings.insert("Schema.field1".to_string(), 
            vec!["transform_1".to_string()].into_iter().collect());
        let mapping_bytes = serde_json::to_vec(&storage_mappings).unwrap();
        db_ops.transforms_tree.insert(b"map_schema_field_to_transforms", mapping_bytes).unwrap();
        db_ops.transforms_tree.flush().unwrap();

        // In-memory has field1 -> [transform_2] and field2 -> [transform_3]
        let mut in_memory_mappings = BTreeMap::new();
        in_memory_mappings.insert("Schema.field1".to_string(), 
            vec!["transform_2".to_string()].into_iter().collect());
        in_memory_mappings.insert("Schema.field2".to_string(), 
            vec!["transform_3".to_string()].into_iter().collect());

        let empty_transforms = HashMap::new();

        let (_, merged_mappings) = 
            db_ops.sync_transform_state(&empty_transforms, &in_memory_mappings).unwrap();

        // field1 should have both transform_1 and transform_2
        assert_eq!(merged_mappings.len(), 2);
        let field1_transforms = merged_mappings.get("Schema.field1").unwrap();
        assert_eq!(field1_transforms.len(), 2);
        assert!(field1_transforms.contains("transform_1"));
        assert!(field1_transforms.contains("transform_2"));

        // field2 should have transform_3
        let field2_transforms = merged_mappings.get("Schema.field2").unwrap();
        assert_eq!(field2_transforms.len(), 1);
        assert!(field2_transforms.contains("transform_3"));
    }

    #[test]
    fn test_sync_round_trip_persistence() {
        let (_temp_dir, db_ops) = create_test_db();
        
        // Create initial state
        let mut transforms = HashMap::new();
        transforms.insert("transform_1".to_string(), create_test_transform("1"));
        transforms.insert("transform_2".to_string(), create_test_transform("2"));

        let mut mappings = BTreeMap::new();
        mappings.insert("Schema.field1".to_string(), 
            vec!["transform_1".to_string()].into_iter().collect());
        mappings.insert("Schema.field2".to_string(), 
            vec!["transform_1".to_string(), "transform_2".to_string()].into_iter().collect());

        // First sync - write to storage
        db_ops.sync_transform_state(&transforms, &mappings).unwrap();

        // Second sync - load from storage with empty in-memory
        let empty_transforms = HashMap::new();
        let empty_mappings = BTreeMap::new();
        let (loaded_transforms, loaded_mappings) = 
            db_ops.sync_transform_state(&empty_transforms, &empty_mappings).unwrap();

        // Should match original
        assert_eq!(loaded_transforms.len(), 2);
        assert!(loaded_transforms.contains_key("transform_1"));
        assert!(loaded_transforms.contains_key("transform_2"));

        assert_eq!(loaded_mappings.len(), 2);
        assert_eq!(loaded_mappings.get("Schema.field1").unwrap().len(), 1);
        assert_eq!(loaded_mappings.get("Schema.field2").unwrap().len(), 2);
    }

    #[test]
    fn test_sync_skips_metadata_keys() {
        let (_temp_dir, db_ops) = create_test_db();
        
        // Manually insert metadata key and a transform
        db_ops.transforms_tree.insert(b"map_schema_field_to_transforms", b"{}").unwrap();
        let transform1 = create_test_transform("1");
        db_ops.store_in_tree(&db_ops.transforms_tree, "transform_1", &transform1).unwrap();

        let empty_transforms = HashMap::new();
        let empty_mappings = BTreeMap::new();

        let (merged_transforms, _) = 
            db_ops.sync_transform_state(&empty_transforms, &empty_mappings).unwrap();

        // Should only have transform_1, not the metadata key
        assert_eq!(merged_transforms.len(), 1);
        assert!(merged_transforms.contains_key("transform_1"));
        assert!(!merged_transforms.contains_key("map_schema_field_to_transforms"));
    }

    #[test]
    fn test_deserialize_mapping_with_arrays() {
        let json = r#"{"field1": ["transform1", "transform2"], "field2": ["transform3"]}"#;
        let result = deserialize_mapping(json.as_bytes(), "test");
        
        assert!(result.is_ok());
        let mapping = result.unwrap();
        assert_eq!(mapping.len(), 2);
        assert_eq!(mapping.get("field1").unwrap().len(), 2);
        assert_eq!(mapping.get("field2").unwrap().len(), 1);
    }

    #[test]
    fn test_deserialize_mapping_with_null() {
        let json = r#"{"field1": null, "field2": ["transform1"]}"#;
        let result = deserialize_mapping(json.as_bytes(), "test");
        
        assert!(result.is_ok());
        let mapping = result.unwrap();
        assert_eq!(mapping.get("field1").unwrap().len(), 0);
        assert_eq!(mapping.get("field2").unwrap().len(), 1);
    }

    #[test]
    fn test_deserialize_mapping_invalid_format() {
        let json = r#"{"field1": "not_an_array"}"#;
        let result = deserialize_mapping(json.as_bytes(), "test");
        
        assert!(result.is_err());
        match result {
            Err(SchemaError::InvalidData(msg)) => {
                assert!(msg.contains("expected array"));
            }
            _ => panic!("Expected InvalidData error"),
        }
    }
}
