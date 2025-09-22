use super::errors::RegistryError;
use super::schema::NativeSchema;
use crate::transform::native::FieldDefinition;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Concurrent registry storing native schema definitions.
#[derive(Debug, Clone, Default)]
pub struct NativeSchemaRegistry {
    schemas: Arc<RwLock<HashMap<String, NativeSchema>>>,
}

impl NativeSchemaRegistry {
    /// Create an empty registry instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a schema. Fails if another schema with the same name exists or
    /// if the schema fails validation.
    pub async fn register_schema(&self, schema: NativeSchema) -> Result<(), RegistryError> {
        schema
            .validate_integrity()
            .map_err(|source| RegistryError::InvalidSchema {
                name: schema.name().to_string(),
                source,
            })?;

        let mut guard = self.schemas.write().await;
        let name = schema.name().to_string();
        if guard.contains_key(&name) {
            return Err(RegistryError::SchemaExists { name });
        }

        guard.insert(name, schema);
        Ok(())
    }

    /// Replace an existing schema with a new definition, returning the previous
    /// version. Validation runs on the new schema before replacement.
    pub async fn replace_schema(
        &self,
        schema: NativeSchema,
    ) -> Result<Option<NativeSchema>, RegistryError> {
        schema
            .validate_integrity()
            .map_err(|source| RegistryError::InvalidSchema {
                name: schema.name().to_string(),
                source,
            })?;

        let mut guard = self.schemas.write().await;
        let name = schema.name().to_string();
        Ok(guard.insert(name, schema))
    }

    /// Remove a schema by name.
    pub async fn remove_schema(&self, name: &str) -> Result<NativeSchema, RegistryError> {
        let mut guard = self.schemas.write().await;
        guard
            .remove(name)
            .ok_or_else(|| RegistryError::SchemaNotFound {
                name: name.to_string(),
            })
    }

    /// Retrieve a schema by name.
    pub async fn get_schema(&self, name: &str) -> Option<NativeSchema> {
        let guard = self.schemas.read().await;
        guard.get(name).cloned()
    }

    /// Retrieve a single field definition by schema and field name.
    pub async fn get_field(&self, schema_name: &str, field_name: &str) -> Option<FieldDefinition> {
        let guard = self.schemas.read().await;
        guard
            .get(schema_name)
            .and_then(|schema| schema.get_field(field_name))
            .cloned()
    }

    /// Determine whether a schema exists in the registry.
    pub async fn contains_schema(&self, name: &str) -> bool {
        let guard = self.schemas.read().await;
        guard.contains_key(name)
    }

    /// Count registered schemas.
    pub async fn len(&self) -> usize {
        let guard = self.schemas.read().await;
        guard.len()
    }

    /// Returns true if no schemas are registered.
    pub async fn is_empty(&self) -> bool {
        let guard = self.schemas.read().await;
        guard.is_empty()
    }

    /// Return schema names in deterministic order for predictable testing.
    pub async fn list_schemas(&self) -> Vec<String> {
        let guard = self.schemas.read().await;
        let mut names: Vec<String> = guard.keys().cloned().collect();
        names.sort();
        names
    }

    /// Remove all schemas from the registry.
    pub async fn clear(&self) {
        let mut guard = self.schemas.write().await;
        guard.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::native::{FieldDefinition, FieldType};

    fn build_schema(name: &str) -> NativeSchema {
        let mut schema = NativeSchema::new(
            name,
            KeyConfig::HashRange {
                hash_field: "hash".into(),
                range_field: "range".into(),
            },
        )
        .expect("schema");
        schema
            .add_fields([
                FieldDefinition::new("hash", FieldType::String),
                FieldDefinition::new("range", FieldType::Integer),
                FieldDefinition::new("value", FieldType::String).with_required(false),
            ])
            .expect("fields");
        schema
    }

    use super::super::schema::KeyConfig;

    #[tokio::test]
    async fn register_and_fetch_schema() {
        let registry = NativeSchemaRegistry::new();
        let schema = build_schema("Blog");

        registry
            .register_schema(schema.clone())
            .await
            .expect("register");
        let fetched = registry.get_schema("Blog").await.expect("fetched");

        assert_eq!(fetched, schema);
    }

    #[tokio::test]
    async fn duplicate_registration_is_rejected() {
        let registry = NativeSchemaRegistry::new();
        let schema = build_schema("Blog");

        registry
            .register_schema(schema.clone())
            .await
            .expect("register");
        let err = registry
            .register_schema(schema)
            .await
            .expect_err("duplicate");
        assert!(matches!(err, RegistryError::SchemaExists { name } if name == "Blog"));
    }

    #[tokio::test]
    async fn list_schemas_returns_sorted_names() {
        let registry = NativeSchemaRegistry::new();
        registry
            .register_schema(build_schema("beta"))
            .await
            .expect("beta");
        registry
            .register_schema(build_schema("alpha"))
            .await
            .expect("alpha");

        let names = registry.list_schemas().await;
        assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[tokio::test]
    async fn get_field_returns_cloned_definition() {
        let registry = NativeSchemaRegistry::new();
        registry
            .register_schema(build_schema("Blog"))
            .await
            .expect("register");

        let field = registry.get_field("Blog", "value").await.expect("field");
        assert_eq!(field.name, "value");
    }

    #[tokio::test]
    async fn is_empty_reflects_registry_state() {
        let registry = NativeSchemaRegistry::new();
        assert!(registry.is_empty().await);

        registry
            .register_schema(build_schema("Blog"))
            .await
            .expect("register");

        assert!(!registry.is_empty().await);
    }

    #[tokio::test]
    async fn remove_schema_returns_previous_value() {
        let registry = NativeSchemaRegistry::new();
        registry
            .register_schema(build_schema("Blog"))
            .await
            .expect("register");

        let removed = registry.remove_schema("Blog").await.expect("removed");
        assert_eq!(removed.name(), "Blog");
        assert!(registry.get_schema("Blog").await.is_none());
    }

    #[tokio::test]
    async fn invalid_schema_is_rejected() {
        let registry = NativeSchemaRegistry::new();
        let schema = NativeSchema::new(
            "Broken",
            KeyConfig::Single {
                key_field: "missing".into(),
            },
        )
        .expect("schema");

        let err = registry.register_schema(schema).await.expect_err("invalid");
        assert!(matches!(err, RegistryError::InvalidSchema { .. }));
    }
}
