use crate::schema::types::errors::SchemaError;
use crate::view::dependency_tracker::DependencyTracker;
use crate::view::invertibility::verify_roundtrip;
use crate::view::types::{TransformFieldDef, TransformView, TransformWriteMode};
use crate::view::wasm_engine::WasmTransformEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Lifecycle state for a view (mirrors SchemaState).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewState {
    Available,
    Approved,
    Blocked,
}

/// Registry for transform views — stores definitions, manages lifecycle, owns WASM engine.
#[derive(Debug)]
pub struct ViewRegistry {
    views: HashMap<String, TransformView>,
    view_states: HashMap<String, ViewState>,
    pub(crate) dependency_tracker: DependencyTracker,
    wasm_engine: Arc<WasmTransformEngine>,
}

impl ViewRegistry {
    pub fn new(wasm_engine: Arc<WasmTransformEngine>) -> Self {
        Self {
            views: HashMap::new(),
            view_states: HashMap::new(),
            dependency_tracker: DependencyTracker::new(),
            wasm_engine,
        }
    }

    /// Load views from storage on startup.
    pub fn load(
        views: Vec<TransformView>,
        view_states: HashMap<String, ViewState>,
        wasm_engine: Arc<WasmTransformEngine>,
    ) -> Self {
        let mut dependency_tracker = DependencyTracker::new();
        dependency_tracker.rebuild(&views);

        let views_map: HashMap<String, TransformView> =
            views.into_iter().map(|v| (v.name.clone(), v)).collect();

        Self {
            views: views_map,
            view_states,
            dependency_tracker,
            wasm_engine,
        }
    }

    /// Register a new view. Validates source references, determines write modes,
    /// and checks for cycles.
    ///
    /// `schema_exists_fn` is called to verify that source schemas exist.
    /// This avoids a direct dependency on SchemaCore.
    pub fn register_view<F>(
        &mut self,
        mut view: TransformView,
        schema_exists_fn: F,
    ) -> Result<(), SchemaError>
    where
        F: Fn(&str) -> bool,
    {
        // Check for duplicate name
        if self.views.contains_key(&view.name) {
            return Err(SchemaError::InvalidData(format!(
                "View '{}' already exists",
                view.name
            )));
        }

        // Validate all source references exist (either as schemas or as other views)
        for (field_name, field_def) in &view.fields {
            let source_schema = &field_def.source.schema;
            if !schema_exists_fn(source_schema) && !self.views.contains_key(source_schema) {
                return Err(SchemaError::NotFound(format!(
                    "Source '{}' for view field '{}' not found as schema or view",
                    field_def.source, field_name
                )));
            }
        }

        // Check for cycles
        let view_fields_map: HashMap<String, HashMap<String, TransformFieldDef>> = self
            .views
            .iter()
            .map(|(name, v)| (name.clone(), v.fields.clone()))
            .collect();

        if self
            .dependency_tracker
            .would_create_cycle(&view.name, &view.fields, &view_fields_map)
        {
            return Err(SchemaError::InvalidData(format!(
                "View '{}' would create a dependency cycle",
                view.name
            )));
        }

        // Determine write modes for each field
        let mut write_modes = HashMap::new();
        for (field_name, field_def) in &view.fields {
            let mode = self.determine_write_mode(field_def)?;
            write_modes.insert(field_name.clone(), mode);
        }
        view.write_modes = write_modes;

        // Register dependencies and store
        self.dependency_tracker
            .register(&view.name, &view.fields);
        self.view_states
            .insert(view.name.clone(), ViewState::Available);
        self.views.insert(view.name.clone(), view);

        Ok(())
    }

    fn determine_write_mode(
        &self,
        field_def: &TransformFieldDef,
    ) -> Result<TransformWriteMode, SchemaError> {
        match (&field_def.wasm_forward, &field_def.wasm_inverse) {
            (None, None) => Ok(TransformWriteMode::Identity),
            (Some(forward), Some(inverse)) => {
                let reversible = verify_roundtrip(&self.wasm_engine, forward, inverse)?;
                if reversible {
                    Ok(TransformWriteMode::Reversible)
                } else {
                    Ok(TransformWriteMode::Irreversible)
                }
            }
            (Some(_), None) => Ok(TransformWriteMode::Irreversible),
            (None, Some(_)) => Err(SchemaError::InvalidTransform(
                "Inverse WASM provided without forward WASM".to_string(),
            )),
        }
    }

    pub fn get_view(&self, name: &str) -> Option<&TransformView> {
        self.views.get(name)
    }

    pub fn list_views(&self) -> Vec<&TransformView> {
        self.views.values().collect()
    }

    pub fn get_view_state(&self, name: &str) -> Option<ViewState> {
        self.view_states.get(name).copied()
    }

    pub fn get_views_with_states(&self) -> Vec<(&TransformView, ViewState)> {
        self.views
            .values()
            .filter_map(|v| {
                self.view_states
                    .get(&v.name)
                    .map(|state| (v, *state))
            })
            .collect()
    }

    pub fn approve_view(&mut self, name: &str) -> Result<(), SchemaError> {
        if !self.views.contains_key(name) {
            return Err(SchemaError::NotFound(format!("View '{}' not found", name)));
        }
        self.view_states.insert(name.to_string(), ViewState::Approved);
        Ok(())
    }

    pub fn block_view(&mut self, name: &str) -> Result<(), SchemaError> {
        if !self.views.contains_key(name) {
            return Err(SchemaError::NotFound(format!("View '{}' not found", name)));
        }
        self.view_states.insert(name.to_string(), ViewState::Blocked);
        Ok(())
    }

    pub fn remove_view(&mut self, name: &str) -> Result<TransformView, SchemaError> {
        let view = self
            .views
            .remove(name)
            .ok_or_else(|| SchemaError::NotFound(format!("View '{}' not found", name)))?;
        self.view_states.remove(name);
        self.dependency_tracker.unregister(name);
        Ok(view)
    }

    pub fn wasm_engine(&self) -> &Arc<WasmTransformEngine> {
        &self.wasm_engine
    }

    /// Check if a name is already used by a view (for cross-registry uniqueness).
    pub fn name_exists(&self, name: &str) -> bool {
        self.views.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
    use crate::view::types::{FieldRef, TransformFieldDef};

    fn make_registry() -> ViewRegistry {
        let engine = Arc::new(WasmTransformEngine::new().unwrap());
        ViewRegistry::new(engine)
    }

    fn identity_view(name: &str, source_schema: &str, source_field: &str) -> TransformView {
        let mut fields = HashMap::new();
        fields.insert(
            "out".into(),
            TransformFieldDef {
                source: FieldRef::new(source_schema, source_field),
                wasm_forward: None,
                wasm_inverse: None,
            },
        );
        TransformView::new(name, SchemaType::Single, None, fields)
    }

    #[test]
    fn test_register_identity_view() {
        let mut registry = make_registry();
        let view = identity_view("MyView", "BlogPost", "content");

        // Schema exists
        let result = registry.register_view(view, |name| name == "BlogPost");
        assert!(result.is_ok());

        let stored = registry.get_view("MyView").unwrap();
        assert_eq!(stored.name, "MyView");
        assert_eq!(
            *stored.write_modes.get("out").unwrap(),
            TransformWriteMode::Identity
        );
        assert_eq!(
            registry.get_view_state("MyView"),
            Some(ViewState::Available)
        );
    }

    #[test]
    fn test_register_missing_source() {
        let mut registry = make_registry();
        let view = identity_view("MyView", "NonExistent", "field");

        let result = registry.register_view(view, |_| false);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_duplicate_name() {
        let mut registry = make_registry();
        let view1 = identity_view("MyView", "S1", "f1");
        let view2 = identity_view("MyView", "S2", "f2");

        registry.register_view(view1, |_| true).unwrap();
        let result = registry.register_view(view2, |_| true);
        assert!(result.is_err());
    }

    #[test]
    fn test_approve_and_block() {
        let mut registry = make_registry();
        let view = identity_view("V1", "S1", "f1");
        registry.register_view(view, |_| true).unwrap();

        registry.approve_view("V1").unwrap();
        assert_eq!(registry.get_view_state("V1"), Some(ViewState::Approved));

        registry.block_view("V1").unwrap();
        assert_eq!(registry.get_view_state("V1"), Some(ViewState::Blocked));
    }

    #[test]
    fn test_remove_view() {
        let mut registry = make_registry();
        let view = identity_view("V1", "S1", "f1");
        registry.register_view(view, |_| true).unwrap();

        let removed = registry.remove_view("V1").unwrap();
        assert_eq!(removed.name, "V1");
        assert!(registry.get_view("V1").is_none());
        assert!(registry.get_view_state("V1").is_none());
    }

    #[test]
    fn test_view_as_source_for_another_view() {
        let mut registry = make_registry();

        // Register ViewA reading from schema S1
        let view_a = identity_view("ViewA", "S1", "f1");
        registry.register_view(view_a, |n| n == "S1").unwrap();

        // Register ViewB reading from ViewA (which is a registered view)
        let view_b = identity_view("ViewB", "ViewA", "out");
        registry.register_view(view_b, |_| false).unwrap();

        assert!(registry.get_view("ViewB").is_some());
    }

    #[test]
    fn test_cycle_detection() {
        let mut registry = make_registry();

        // ViewA reads from S1
        let view_a = identity_view("ViewA", "S1", "f1");
        registry.register_view(view_a, |n| n == "S1").unwrap();

        // ViewB reads from ViewA
        let view_b = identity_view("ViewB", "ViewA", "out");
        registry.register_view(view_b, |_| false).unwrap();

        // ViewC tries to read from ViewB, but if ViewA also tried to read from ViewC...
        // For now just test that non-cyclic chains work
        let view_c = identity_view("ViewC", "ViewB", "out");
        let result = registry.register_view(view_c, |_| false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_inverse_without_forward_errors() {
        let mut registry = make_registry();
        let mut fields = HashMap::new();
        fields.insert(
            "bad".into(),
            TransformFieldDef {
                source: FieldRef::new("S1", "f1"),
                wasm_forward: None,
                wasm_inverse: Some(vec![0, 1, 2]),
            },
        );
        let view = TransformView::new("BadView", SchemaType::Single, None, fields);
        let result = registry.register_view(view, |_| true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Inverse WASM"));
    }

    #[test]
    fn test_list_views() {
        let mut registry = make_registry();
        registry
            .register_view(identity_view("V1", "S1", "f1"), |_| true)
            .unwrap();
        registry
            .register_view(identity_view("V2", "S2", "f2"), |_| true)
            .unwrap();

        let views = registry.list_views();
        assert_eq!(views.len(), 2);
    }
}
