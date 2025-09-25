use super::core::DbOperations;
use crate::transform::manager::transform_manager::SCHEMA_FIELD_TO_TRANSFORMS_KEY;
use crate::schema::types::transform::{Transform, TransformRegistration};
use crate::schema::SchemaError;

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
        let metadata_keys = [
            SCHEMA_FIELD_TO_TRANSFORMS_KEY,
        ];

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
    ) -> Result<std::collections::BTreeMap<String, std::collections::HashSet<String>>, SchemaError> {
        use crate::transform::manager::utils::SerializationHelper;
        use log::info;

        // Load field_to_transforms with special debug logging
        let schema_field_to_transforms = match self.get_transform_mapping(key)? {
            Some(data) => {
                let loaded_map: std::collections::BTreeMap<String, std::collections::HashSet<String>> =
                    SerializationHelper::deserialize_mapping(&data, "field_to_transforms")?;
                info!(
                    "🔍 DEBUG: Loaded field_to_transforms mapping from database with {} entries:",
                    loaded_map.len()
                );
                for (field_key, transforms) in &loaded_map {
                    info!("  📋 Loaded '{}' -> {:?}", field_key, transforms);
                }
                loaded_map
            }
            None => std::collections::BTreeMap::new(),
        };

        Ok(schema_field_to_transforms)
    }
}
