use crate::view::types::{TransformFieldDef, TransformView};
use std::collections::HashMap;
use std::collections::HashSet;

/// Tracks which view fields depend on which source schema fields.
/// Used for cache invalidation when source data changes.
#[derive(Debug, Default)]
pub struct DependencyTracker {
    /// (source_schema, source_field) -> [(view_name, view_field_name)]
    deps: HashMap<(String, String), Vec<(String, String)>>,
}

impl DependencyTracker {
    pub fn new() -> Self {
        Self {
            deps: HashMap::new(),
        }
    }

    /// Register all field dependencies for a view.
    pub fn register(&mut self, view_name: &str, fields: &HashMap<String, TransformFieldDef>) {
        for (field_name, field_def) in fields {
            let key = (
                field_def.source.schema.clone(),
                field_def.source.field.clone(),
            );
            self.deps
                .entry(key)
                .or_default()
                .push((view_name.to_string(), field_name.clone()));
        }
    }

    /// Remove all dependency entries for a given view.
    pub fn unregister(&mut self, view_name: &str) {
        self.deps.retain(|_key, dependents| {
            dependents.retain(|(vn, _)| vn != view_name);
            !dependents.is_empty()
        });
    }

    /// Get all view fields that depend on a given source schema field.
    pub fn get_dependents(&self, schema: &str, field: &str) -> &[(String, String)] {
        let key = (schema.to_string(), field.to_string());
        self.deps.get(&key).map_or(&[], |v| v.as_slice())
    }

    /// Rebuild the tracker from a set of views (used on startup).
    pub fn rebuild(&mut self, views: &[TransformView]) {
        self.deps.clear();
        for view in views {
            self.register(&view.name, &view.fields);
        }
    }

    /// Check if adding a view with the given fields would create a dependency cycle.
    ///
    /// A cycle exists if the new view reads from a source that (transitively) reads
    /// from the new view. Since views can be sources for other views, we perform DFS
    /// through the dependency graph.
    ///
    /// `view_fields_map` provides all registered views' fields for traversal.
    pub fn would_create_cycle(
        &self,
        new_view_name: &str,
        new_fields: &HashMap<String, TransformFieldDef>,
        view_fields_map: &HashMap<String, HashMap<String, TransformFieldDef>>,
    ) -> bool {
        // Collect all source schemas the new view reads from
        let sources: HashSet<String> = new_fields
            .values()
            .map(|f| f.source.schema.clone())
            .collect();

        // DFS: check if any source schema transitively depends on new_view_name
        let mut visited = HashSet::new();
        let mut stack: Vec<String> = sources.into_iter().collect();

        while let Some(current) = stack.pop() {
            if current == new_view_name {
                return true; // Cycle detected
            }
            if !visited.insert(current.clone()) {
                continue; // Already visited
            }
            // If `current` is itself a view, check what it reads from
            if let Some(fields) = view_fields_map.get(&current) {
                for field_def in fields.values() {
                    stack.push(field_def.source.schema.clone());
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
    use crate::view::types::FieldRef;

    fn make_field(schema: &str, field: &str) -> TransformFieldDef {
        TransformFieldDef {
            source: FieldRef::new(schema, field),
            wasm_forward: None,
            wasm_inverse: None,
        }
    }

    #[test]
    fn test_register_and_lookup() {
        let mut tracker = DependencyTracker::new();
        let mut fields = HashMap::new();
        fields.insert("words".into(), make_field("BlogPost", "content"));
        fields.insert("temp_f".into(), make_field("Weather", "temp_c"));

        tracker.register("Analytics", &fields);

        let deps = tracker.get_dependents("BlogPost", "content");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], ("Analytics".into(), "words".into()));

        let deps = tracker.get_dependents("Weather", "temp_c");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], ("Analytics".into(), "temp_f".into()));

        // No dependents for unregistered source
        assert!(tracker.get_dependents("Other", "field").is_empty());
    }

    #[test]
    fn test_unregister() {
        let mut tracker = DependencyTracker::new();
        let mut fields = HashMap::new();
        fields.insert("a".into(), make_field("S1", "f1"));
        tracker.register("View1", &fields);

        let mut fields2 = HashMap::new();
        fields2.insert("b".into(), make_field("S1", "f1"));
        tracker.register("View2", &fields2);

        assert_eq!(tracker.get_dependents("S1", "f1").len(), 2);

        tracker.unregister("View1");
        let deps = tracker.get_dependents("S1", "f1");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "View2");
    }

    #[test]
    fn test_rebuild() {
        let mut tracker = DependencyTracker::new();
        let mut fields = HashMap::new();
        fields.insert("x".into(), make_field("S1", "f1"));
        tracker.register("OldView", &fields);

        let views = vec![TransformView::new(
            "NewView",
            SchemaType::Single,
            None,
            {
                let mut f = HashMap::new();
                f.insert("y".into(), make_field("S2", "f2"));
                f
            },
        )];
        tracker.rebuild(&views);

        assert!(tracker.get_dependents("S1", "f1").is_empty());
        assert_eq!(tracker.get_dependents("S2", "f2").len(), 1);
    }

    #[test]
    fn test_no_cycle_with_plain_schemas() {
        let tracker = DependencyTracker::new();
        let mut fields = HashMap::new();
        fields.insert("a".into(), make_field("PlainSchema", "f1"));

        // PlainSchema is not a view, so no cycle possible
        let view_fields_map = HashMap::new();
        assert!(!tracker.would_create_cycle("MyView", &fields, &view_fields_map));
    }

    #[test]
    fn test_direct_cycle() {
        let tracker = DependencyTracker::new();

        // ViewA reads from ViewB, and we're checking if ViewB reading from ViewA creates a cycle
        let mut view_a_fields = HashMap::new();
        view_a_fields.insert("a".into(), make_field("ViewB", "x"));

        let mut view_fields_map = HashMap::new();
        view_fields_map.insert("ViewA".to_string(), view_a_fields);

        // ViewB wants to read from ViewA — should detect cycle
        let mut new_fields = HashMap::new();
        new_fields.insert("b".into(), make_field("ViewA", "y"));

        assert!(tracker.would_create_cycle("ViewB", &new_fields, &view_fields_map));
    }

    #[test]
    fn test_transitive_cycle() {
        let tracker = DependencyTracker::new();

        // ViewA reads from ViewB, ViewB reads from ViewC
        let mut view_a_fields = HashMap::new();
        view_a_fields.insert("a".into(), make_field("ViewB", "x"));

        let mut view_b_fields = HashMap::new();
        view_b_fields.insert("b".into(), make_field("ViewC", "y"));

        let mut view_fields_map = HashMap::new();
        view_fields_map.insert("ViewA".to_string(), view_a_fields);
        view_fields_map.insert("ViewB".to_string(), view_b_fields);

        // ViewC wants to read from ViewA — transitive cycle: C -> A -> B -> C
        let mut new_fields = HashMap::new();
        new_fields.insert("c".into(), make_field("ViewA", "z"));

        assert!(tracker.would_create_cycle("ViewC", &new_fields, &view_fields_map));
    }

    #[test]
    fn test_no_cycle_diamond() {
        let tracker = DependencyTracker::new();

        // ViewA reads from S1, ViewB reads from S1 — diamond, no cycle
        let mut view_a_fields = HashMap::new();
        view_a_fields.insert("a".into(), make_field("S1", "f1"));

        let mut view_b_fields = HashMap::new();
        view_b_fields.insert("b".into(), make_field("S1", "f1"));

        let mut view_fields_map = HashMap::new();
        view_fields_map.insert("ViewA".to_string(), view_a_fields);
        view_fields_map.insert("ViewB".to_string(), view_b_fields);

        // ViewC reads from ViewA and ViewB — no cycle
        let mut new_fields = HashMap::new();
        new_fields.insert("x".into(), make_field("ViewA", "a"));
        new_fields.insert("y".into(), make_field("ViewB", "b"));

        assert!(!tracker.would_create_cycle("ViewC", &new_fields, &view_fields_map));
    }
}
