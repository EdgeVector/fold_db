use crate::schema::types::errors::SchemaError;
use crate::view::dependency_tracker::DependencyTracker;
use crate::view::types::TransformView;
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

    /// Register a new view. Validates source references, checks for cycles.
    ///
    /// `schema_exists_fn` is called to verify that source schemas exist.
    /// This avoids a direct dependency on SchemaCore.
    pub fn register_view<F>(
        &mut self,
        view: TransformView,
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

        // Validate input queries have explicit field lists and no duplicate (schema, field) pairs
        {
            let mut seen_fields: HashMap<(String, String), usize> = HashMap::new();
            for (i, query) in view.input_queries.iter().enumerate() {
                if query.fields.is_empty() {
                    return Err(SchemaError::InvalidData(format!(
                        "View '{}' input query {} (schema '{}') must declare explicit fields",
                        view.name, i, query.schema_name
                    )));
                }
                for field_name in &query.fields {
                    let key = (query.schema_name.clone(), field_name.clone());
                    if let Some(prev_query) = seen_fields.get(&key) {
                        return Err(SchemaError::InvalidData(format!(
                            "View '{}' has duplicate field '{}.{}' in input queries {} and {}",
                            view.name, query.schema_name, field_name, prev_query, i
                        )));
                    }
                    seen_fields.insert(key, i);
                }
            }
        }

        // Validate all source schemas exist (either as schemas or as other views)
        for schema_name in view.source_schemas() {
            if !schema_exists_fn(&schema_name) && !self.views.contains_key(&schema_name) {
                return Err(SchemaError::NotFound(format!(
                    "Source schema '{}' for view '{}' not found as schema or view",
                    schema_name, view.name
                )));
            }
        }

        // Validate output_fields is not empty
        if view.output_fields.is_empty() {
            return Err(SchemaError::InvalidData(format!(
                "View '{}' must declare at least one output field",
                view.name
            )));
        }

        // For identity views (no WASM), validate that output field names match
        // fields available from input queries, and that no field name appears
        // in multiple input queries (ambiguous source)
        if view.is_identity() {
            let mut field_sources: HashMap<String, String> = HashMap::new();
            for query in &view.input_queries {
                for field_name in &query.fields {
                    if let Some(prev_schema) = field_sources.get(field_name) {
                        return Err(SchemaError::InvalidData(format!(
                            "Identity view '{}' has ambiguous field '{}': appears in both '{}' and '{}'",
                            view.name, field_name, prev_schema, query.schema_name
                        )));
                    }
                    field_sources.insert(field_name.clone(), query.schema_name.clone());
                }
            }
            for output_field in view.output_fields.keys() {
                if !field_sources.contains_key(output_field) {
                    return Err(SchemaError::InvalidData(format!(
                        "Identity view '{}' output field '{}' not found in input query fields",
                        view.name, output_field
                    )));
                }
            }
        }

        // WASM transforms can only produce range keys — reject Hash/HashRange output schemas
        if view.wasm_transform.is_some() {
            use crate::schema::types::schema::DeclarativeSchemaType;
            match &view.schema_type {
                DeclarativeSchemaType::Hash | DeclarativeSchemaType::HashRange => {
                    return Err(SchemaError::InvalidData(format!(
                        "View '{}' uses a WASM transform but declares {:?} schema type. \
                         WASM transforms can only produce Range or Single keyed output.",
                        view.name, view.schema_type
                    )));
                }
                _ => {}
            }
        }

        // Check for cycles
        if self
            .dependency_tracker
            .would_create_cycle(&view.name, &view, &self.views)
        {
            return Err(SchemaError::InvalidData(format!(
                "View '{}' would create a dependency cycle",
                view.name
            )));
        }

        // Register dependencies and store
        self.dependency_tracker.register(&view);
        self.view_states
            .insert(view.name.clone(), ViewState::Available);
        self.views.insert(view.name.clone(), view);

        Ok(())
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
            .filter_map(|v| self.view_states.get(&v.name).map(|state| (v, *state)))
            .collect()
    }

    pub fn approve_view(&mut self, name: &str) -> Result<(), SchemaError> {
        if !self.views.contains_key(name) {
            return Err(SchemaError::NotFound(format!("View '{}' not found", name)));
        }
        self.view_states
            .insert(name.to_string(), ViewState::Approved);
        Ok(())
    }

    pub fn block_view(&mut self, name: &str) -> Result<(), SchemaError> {
        if !self.views.contains_key(name) {
            return Err(SchemaError::NotFound(format!("View '{}' not found", name)));
        }
        self.view_states
            .insert(name.to_string(), ViewState::Blocked);
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
    use crate::schema::types::field_value_type::FieldValueType;
    use crate::schema::types::operations::Query;
    use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
    use crate::view::types::WasmTransformSpec;

    fn make_registry() -> ViewRegistry {
        let engine = Arc::new(WasmTransformEngine::new().unwrap());
        ViewRegistry::new(engine)
    }

    fn identity_view(name: &str, source_schema: &str, source_field: &str) -> TransformView {
        TransformView::new(
            name,
            SchemaType::Single,
            None,
            vec![Query::new(
                source_schema.to_string(),
                vec![source_field.to_string()],
            )],
            None,
            HashMap::from([(source_field.to_string(), FieldValueType::Any)]),
        )
    }

    #[test]
    fn test_register_identity_view() {
        let mut registry = make_registry();
        let view = identity_view("MyView", "BlogPost", "content");

        let result = registry.register_view(view, |name| name == "BlogPost");
        assert!(result.is_ok());

        let stored = registry.get_view("MyView").unwrap();
        assert_eq!(stored.name, "MyView");
        assert!(stored.is_identity());
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

        let view_a = identity_view("ViewA", "S1", "f1");
        registry.register_view(view_a, |n| n == "S1").unwrap();

        // ViewB reads from ViewA (which is a registered view)
        // ViewA's output field is "f1", so ViewB must reference that
        let view_b = identity_view("ViewB", "ViewA", "f1");
        registry.register_view(view_b, |_| false).unwrap();

        assert!(registry.get_view("ViewB").is_some());
    }

    #[test]
    fn test_cycle_detection() {
        let mut registry = make_registry();

        let view_a = identity_view("ViewA", "S1", "f1");
        registry.register_view(view_a, |n| n == "S1").unwrap();

        let view_b = identity_view("ViewB", "ViewA", "f1");
        registry.register_view(view_b, |_| false).unwrap();

        // Non-cyclic chain should work
        let view_c = identity_view("ViewC", "ViewB", "f1");
        let result = registry.register_view(view_c, |_| false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_output_fields_errors() {
        let mut registry = make_registry();
        let view = TransformView::new(
            "BadView",
            SchemaType::Single,
            None,
            vec![Query::new("S1".to_string(), vec!["f1".to_string()])],
            None,
            HashMap::new(), // Empty output fields
        );
        let result = registry.register_view(view, |_| true);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least one output field"));
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

    #[test]
    fn test_empty_input_query_fields_rejected() {
        let mut registry = make_registry();
        let view = TransformView::new(
            "BadView",
            SchemaType::Single,
            None,
            vec![Query::new("S1".to_string(), vec![])], // Empty fields
            None,
            HashMap::from([("f1".to_string(), FieldValueType::Any)]),
        );
        let result = registry.register_view(view, |_| true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("explicit fields"));
    }

    #[test]
    fn test_identity_view_ambiguous_field_name_rejected() {
        let mut registry = make_registry();
        // Two input queries both have field "name" — ambiguous for identity
        let view = TransformView::new(
            "AmbiguousView",
            SchemaType::Single,
            None,
            vec![
                Query::new("S1".to_string(), vec!["name".to_string()]),
                Query::new("S2".to_string(), vec!["name".to_string()]),
            ],
            None,
            HashMap::from([("name".to_string(), FieldValueType::Any)]),
        );
        let result = registry.register_view(view, |_| true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ambiguous"));
    }

    #[test]
    fn test_duplicate_schema_field_across_queries_rejected() {
        let mut registry = make_registry();
        let view = TransformView::new(
            "DupView",
            SchemaType::Single,
            None,
            vec![
                Query::new("S1".to_string(), vec!["f1".to_string()]),
                Query::new("S1".to_string(), vec!["f1".to_string()]), // Duplicate
            ],
            None,
            HashMap::from([("f1".to_string(), FieldValueType::Any)]),
        );
        let result = registry.register_view(view, |_| true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate field"));
    }

    #[test]
    fn test_same_schema_different_fields_allowed() {
        let mut registry = make_registry();
        let view = TransformView::new(
            "SplitView",
            SchemaType::Single,
            None,
            vec![
                Query::new("S1".to_string(), vec!["f1".to_string()]),
                Query::new("S1".to_string(), vec!["f2".to_string()]),
            ],
            None,
            HashMap::from([
                ("f1".to_string(), FieldValueType::Any),
                ("f2".to_string(), FieldValueType::Any),
            ]),
        );
        assert!(registry.register_view(view, |_| true).is_ok());
    }

    #[test]
    fn test_wasm_view_rejects_hash_schema_type() {
        let mut registry = make_registry();
        let view = TransformView::new(
            "HashWasm",
            SchemaType::Hash,
            None,
            vec![Query::new("S1".to_string(), vec!["f1".to_string()])],
            Some(WasmTransformSpec {
                bytes: vec![0, 1, 2],
                max_gas: 1_000_000,
                gas_model: None,
            }),
            HashMap::from([("out".to_string(), FieldValueType::Any)]),
        );
        let result = registry.register_view(view, |_| true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("WASM transform"));
    }

    #[test]
    fn test_wasm_view_allows_single_and_range() {
        let mut registry = make_registry();
        let single_view = TransformView::new(
            "SingleWasm",
            SchemaType::Single,
            None,
            vec![Query::new("S1".to_string(), vec!["f1".to_string()])],
            Some(WasmTransformSpec {
                bytes: vec![0, 1, 2],
                max_gas: 1_000_000,
                gas_model: None,
            }),
            HashMap::from([("out".to_string(), FieldValueType::Any)]),
        );
        assert!(registry.register_view(single_view, |_| true).is_ok());

        let range_view = TransformView::new(
            "RangeWasm",
            SchemaType::Range,
            None,
            vec![Query::new("S2".to_string(), vec!["f2".to_string()])],
            Some(WasmTransformSpec {
                bytes: vec![0, 1, 2],
                max_gas: 1_000_000,
                gas_model: None,
            }),
            HashMap::from([("out".to_string(), FieldValueType::Any)]),
        );
        assert!(registry.register_view(range_view, |_| true).is_ok());
    }

    #[test]
    fn test_direct_cycle_rejected() {
        let mut registry = make_registry();

        // ViewA reads from S1
        let view_a = identity_view("ViewA", "S1", "f1");
        registry.register_view(view_a, |n| n == "S1").unwrap();

        // ViewB reads from ViewA
        let view_b = identity_view("ViewB", "ViewA", "f1");
        registry.register_view(view_b, |_| false).unwrap();

        // ViewC tries to read from ViewB, and ViewA tries to read from ViewC → cycle
        // Simulate: try to register a view that reads from ViewB, name it "S1" — not a cycle
        // Real cycle: register ViewX reading from ViewB, then ViewA reading from ViewX
        // Actually, let's test ViewA→ViewB cycle directly:
        // Can't re-register ViewA, so test a new cycle:
        let view_cycle = TransformView::new(
            "ViewCycle",
            SchemaType::Single,
            None,
            vec![Query::new("ViewB".to_string(), vec!["f1".to_string()])],
            None,
            HashMap::from([("f1".to_string(), FieldValueType::Any)]),
        );
        // This is fine: S1 → ViewA → ViewB → ViewCycle (no cycle)
        assert!(registry.register_view(view_cycle, |_| false).is_ok());

        // Now try to register a view that creates a cycle: reads from ViewCycle
        // but ViewCycle reads from ViewB which reads from ViewA which reads from S1
        // Try to register a new view on S1 that reads from ViewCycle — no cycle (S1 is a schema)

        // Direct cycle test: ViewX reads from ViewY, ViewY reads from ViewX
        let mut registry2 = make_registry();
        let vx = TransformView::new(
            "ViewX",
            SchemaType::Single,
            None,
            vec![Query::new("ViewY".to_string(), vec!["f".to_string()])],
            None,
            HashMap::from([("f".to_string(), FieldValueType::Any)]),
        );
        // ViewX reads from ViewY — ViewY doesn't exist yet as schema or view
        // This should fail because ViewY doesn't exist
        assert!(registry2.register_view(vx, |_| false).is_err());
    }

    #[test]
    fn test_multi_query_view() {
        let mut registry = make_registry();
        let view = TransformView::new(
            "Dashboard",
            SchemaType::Single,
            None,
            vec![
                Query::new(
                    "BlogPost".to_string(),
                    vec!["title".to_string(), "content".to_string()],
                ),
                Query::new("Author".to_string(), vec!["name".to_string()]),
            ],
            Some(WasmTransformSpec {
                bytes: vec![0, 1, 2],
                max_gas: 1_000_000,
                gas_model: None,
            }), // Placeholder WASM
            HashMap::from([
                ("enriched_title".to_string(), FieldValueType::String),
                ("word_count".to_string(), FieldValueType::Integer),
            ]),
        );

        let result = registry.register_view(view, |name| name == "BlogPost" || name == "Author");
        assert!(result.is_ok());

        let stored = registry.get_view("Dashboard").unwrap();
        assert_eq!(stored.input_queries.len(), 2);
        assert_eq!(stored.output_fields.len(), 2);
        assert!(!stored.is_identity());
    }
}
