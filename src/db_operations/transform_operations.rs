use super::core::DbOperations;
use crate::schema::types::transform::{Transform, TransformRegistration};
use crate::schema::SchemaError;
use log::info;
use std::collections::{BTreeMap, HashSet};

impl DbOperations {
    /// Stores a transform using generic tree operations
    pub fn store_transform(
        &self,
        transform_id: &str,
        transform: &Transform,
    ) -> Result<(), SchemaError> {
        self.store_in_tree(&self.transforms_tree, transform_id, transform)
    }

    /// Gets a transform with enhanced error logging
    pub fn get_transform(&self, transform_id: &str) -> Result<Option<Transform>, SchemaError> {
        match self.get_from_tree::<Transform>(&self.transforms_tree, transform_id) {
            Ok(Some(transform)) => {
                Ok(Some(transform))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                Err(e)
            }
        }
    }

    /// Lists all transform IDs (excludes metadata keys)
    pub fn list_transforms(&self) -> Result<Vec<String>, SchemaError> {
        let mut transforms = Vec::new();

        // Metadata keys that should be excluded from transform listing
        // Keys reserved for metadata persisted in the transforms tree
        const SCHEMA_FIELD_TO_TRANSFORMS_KEY: &str = "map_schema_field_to_transforms";
        let metadata_keys = [SCHEMA_FIELD_TO_TRANSFORMS_KEY];

        for result in self.transforms_tree.iter() {
            let (key, _) = result.map_err(|e| {
                SchemaError::InvalidData(format!("Failed to iterate transforms: {}", e))
            })?;
            let transform_id = String::from_utf8_lossy(&key).to_string();

            // Skip metadata keys
            if metadata_keys.contains(&transform_id.as_str()) {
                continue;
            }

            transforms.push(transform_id);
        }

        Ok(transforms)
    }

    /// Deletes a transform using generic tree operations
    pub fn delete_transform(&self, transform_id: &str) -> Result<bool, SchemaError> {
        self.delete_from_tree(&self.transforms_tree, transform_id)
    }

    /// Stores a transform registration
    pub fn store_transform_registration(
        &self,
        registration: &TransformRegistration,
    ) -> Result<(), SchemaError> {
        let key = format!("registration:{}", registration.transform_id);
        self.store_item(&key, registration)
    }

    /// Gets a transform registration
    pub fn get_transform_registration(
        &self,
        transform_id: &str,
    ) -> Result<Option<TransformRegistration>, SchemaError> {
        let key = format!("registration:{}", transform_id);
        self.get_item(&key)
    }

    /// Stores a transform mapping (for internal mappings like molecule_to_transforms)
    pub fn store_transform_mapping(&self, key: &str, data: &[u8]) -> Result<(), SchemaError> {
        self.transforms_tree
            .insert(key.as_bytes(), data)
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to store transform mapping: {}", e))
            })?;
        self.transforms_tree.flush().map_err(|e| {
            SchemaError::InvalidData(format!("Failed to flush transform mappings: {}", e))
        })?;
        Ok(())
    }

    /// Gets a transform mapping
    pub fn get_transform_mapping(&self, key: &str) -> Result<Option<Vec<u8>>, SchemaError> {
        if let Some(bytes) = self.transforms_tree.get(key.as_bytes()).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to get transform mapping: {}", e))
        })? {
            Ok(Some(bytes.to_vec()))
        } else {
            Ok(None)
        }
    }

    /// Load persisted field-to-transforms mappings from database
    pub fn load_field_to_transforms_mapping(
        &self,
        key: &str,
    ) -> Result<BTreeMap<String, HashSet<String>>, SchemaError> {

        // Load field_to_transforms with special debug logging
        let schema_field_to_transforms = match self.get_transform_mapping(key)? {
            Some(data) => {
                let loaded_map: BTreeMap<String, HashSet<String>> =
                    deserialize_mapping(&data, "field_to_transforms")?;
                for (field_key, transforms) in &loaded_map {
                    info!("  📋 Loaded '{}' -> {:?}", field_key, transforms);
                }
                loaded_map
            }
            None => BTreeMap::new(),
        };

        Ok(schema_field_to_transforms)
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
