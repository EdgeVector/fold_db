use crate::schema::types::SchemaError;
use log::{error, info};
use std::sync::RwLock;

use super::TransformUtils;

impl TransformUtils {
    /// Serialize a mapping to bytes with consistent error handling
    pub fn serialize_mapping<T>(
        mapping: &RwLock<T>,
        mapping_name: &str,
    ) -> Result<Vec<u8>, SchemaError>
    where
        T: serde::Serialize,
    {
        info!("🔄 Serializing mapping: {}", mapping_name);

        let map = Self::read_lock(mapping, mapping_name)?;
        let json = serde_json::to_vec(&*map).map_err(|e| {
            let error_msg = format!("Failed to serialize {}: {}", mapping_name, e);
            error!("❌ {}", error_msg);
            SchemaError::InvalidData(error_msg)
        })?;

        info!(
            "✅ Successfully serialized mapping: {} ({} bytes)",
            mapping_name,
            json.len()
        );
        Ok(json)
    }

    /// Deserialize mapping data with consistent error handling
    pub fn deserialize_mapping<T>(data: &[u8], mapping_name: &str) -> Result<T, SchemaError>
    where
        T: serde::de::DeserializeOwned + Default,
    {
        info!("🔄 Deserializing mapping: {}", mapping_name);

        match serde_json::from_slice(data) {
            Ok(result) => {
                info!("✅ Successfully deserialized mapping: {}", mapping_name);
                Ok(result)
            }
            Err(e) => {
                let error_msg = format!("Failed to deserialize {}: {}", mapping_name, e);
                error!("❌ {}", error_msg);
                info!(
                    "🔄 Using default value for {} due to deserialization error",
                    mapping_name
                );
                Ok(T::default())
            }
        }
    }

    /// Store mapping to database
    pub fn store_mapping<T>(
        db_ops: &std::sync::Arc<crate::db_operations::DbOperations>,
        mapping: &RwLock<T>,
        key: &str,
        mapping_name: &str,
    ) -> Result<(), SchemaError>
    where
        T: serde::Serialize,
    {
        info!("💾 Storing mapping: {} to key: {}", mapping_name, key);

        let json = Self::serialize_mapping(mapping, mapping_name)?;
        db_ops.store_transform_mapping(key, &json)?;

        info!(
            "✅ Successfully stored mapping: {} to database",
            mapping_name
        );
        Ok(())
    }
}
