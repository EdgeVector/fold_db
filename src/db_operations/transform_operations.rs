use super::core::DbOperations;
use crate::schema::types::transform::{Transform, TransformRegistration};
use crate::schema::SchemaError;
use crate::logging::features::{log_feature, LogFeature};

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
        match self.get_from_tree(&self.transforms_tree, transform_id) {
            Ok(result) => Ok(result),
            Err(e) => {
                // Enhanced error logging for transform deserialization issues
                if let Ok(Some(bytes)) = self.transforms_tree.get(transform_id.as_bytes()) {
                    let raw_data = String::from_utf8_lossy(&bytes);
                    log_feature!(LogFeature::Database, error, "Failed to deserialize transform '{}': {}", transform_id, e);
                    log_feature!(LogFeature::Database, error, "Raw transform data: {}", raw_data);
                    Err(SchemaError::InvalidData(format!(
                        "Failed to deserialize transform '{}': {}. Raw data: {}",
                        transform_id, e, raw_data
                    )))
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Lists all transform IDs (excludes metadata keys)
    pub fn list_transforms(&self) -> Result<Vec<String>, SchemaError> {
        let mut transforms = Vec::new();

        // Metadata keys that should be excluded from transform listing
        let metadata_keys = [
            "map_molecule_to_transforms",
            "map_transform_to_molecules",
            "map_transform_input_names",
            "map_field_to_transforms",
            "map_transform_to_fields",
            "map_transform_outputs",
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
}
