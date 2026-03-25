use crate::view::types::TransformView;
use std::collections::HashMap;
use std::collections::HashSet;

/// Tracks which views depend on which source schema fields.
/// Used for cache invalidation when source data changes.
///
/// Dependencies are derived from input_queries: for each query,
/// (query.schema_name, field_name) for all fields in query.fields.
#[derive(Debug, Default)]
pub struct DependencyTracker {
    /// (source_schema, source_field) -> [view_name]
    deps: HashMap<(String, String), Vec<String>>,
}

impl DependencyTracker {
    pub fn new() -> Self {
        Self {
            deps: HashMap::new(),
        }
    }

    /// Register all dependencies for a view based on its input queries.
    pub fn register(&mut self, view: &TransformView) {
        for (schema_name, field_name) in view.source_dependencies() {
            self.deps
                .entry((schema_name, field_name))
                .or_default()
                .push(view.name.clone());
        }
    }

    /// Remove all dependency entries for a given view.
    pub fn unregister(&mut self, view_name: &str) {
        self.deps.retain(|_key, dependents| {
            dependents.retain(|vn| vn != view_name);
            !dependents.is_empty()
        });
    }

    /// Get all view names that depend on a given source schema field.
    pub fn get_dependents(&self, schema: &str, field: &str) -> &[String] {
        let key = (schema.to_string(), field.to_string());
        self.deps.get(&key).map_or(&[], |v| v.as_slice())
    }

    /// Get all view names that depend on ANY field from a given source schema.
    /// Used for cascade invalidation when a view's output changes.
    pub fn get_all_dependents_of_schema(&self, schema: &str) -> Vec<String> {
        let mut views = HashSet::new();
        for ((dep_schema, _), view_names) in &self.deps {
            if dep_schema == schema {
                for view_name in view_names {
                    views.insert(view_name.clone());
                }
            }
        }
        views.into_iter().collect()
    }

    /// Rebuild the tracker from a set of views (used on startup).
    pub fn rebuild(&mut self, views: &[TransformView]) {
        self.deps.clear();
        for view in views {
            self.register(view);
        }
    }

    /// Maximum allowed depth for view dependency chains.
    /// Prevents runaway recursion in cascade invalidation and precomputation.
    pub const MAX_DEPTH: usize = 16;

    /// Check if adding a view would create a dependency cycle or exceed the
    /// maximum chain depth.
    ///
    /// A cycle exists if the new view reads from a source that (transitively)
    /// reads from the new view. Since views can be sources for other views,
    /// we perform DFS through the dependency graph. The depth of the new view
    /// in the dependency chain must also not exceed `MAX_DEPTH`.
    pub fn would_create_cycle(
        &self,
        new_view_name: &str,
        new_view: &TransformView,
        existing_views: &HashMap<String, TransformView>,
    ) -> bool {
        // Collect all source schemas the new view reads from
        let sources: HashSet<String> = new_view.source_schemas().into_iter().collect();

        // DFS: check if any source schema transitively depends on new_view_name
        let mut visited = HashSet::new();
        // (view_name, depth)
        let mut stack: Vec<(String, usize)> = sources.into_iter().map(|s| (s, 1)).collect();

        while let Some((current, depth)) = stack.pop() {
            if current == new_view_name {
                return true; // Cycle detected
            }
            if depth > Self::MAX_DEPTH {
                return true; // Depth limit exceeded
            }
            if !visited.insert(current.clone()) {
                continue; // Already visited
            }
            // If `current` is itself a view, check what it reads from
            if let Some(view) = existing_views.get(&current) {
                for schema_name in view.source_schemas() {
                    stack.push((schema_name, depth + 1));
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::operations::Query;
    use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;

    fn make_view(name: &str, queries: Vec<(&str, Vec<&str>)>) -> TransformView {
        let input_queries = queries
            .into_iter()
            .map(|(schema, fields)| {
                Query::new(
                    schema.to_string(),
                    fields.into_iter().map(|f| f.to_string()).collect(),
                )
            })
            .collect();
        TransformView::new(
            name,
            SchemaType::Single,
            None,
            input_queries,
            None,
            HashMap::new(),
        )
    }

    #[test]
    fn test_register_and_lookup() {
        let mut tracker = DependencyTracker::new();
        let view = make_view(
            "Analytics",
            vec![("BlogPost", vec!["content"]), ("Weather", vec!["temp_c"])],
        );
        tracker.register(&view);

        let deps = tracker.get_dependents("BlogPost", "content");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], "Analytics");

        let deps = tracker.get_dependents("Weather", "temp_c");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], "Analytics");

        assert!(tracker.get_dependents("Other", "field").is_empty());
    }

    #[test]
    fn test_unregister() {
        let mut tracker = DependencyTracker::new();
        let view1 = make_view("View1", vec![("S1", vec!["f1"])]);
        let view2 = make_view("View2", vec![("S1", vec!["f1"])]);
        tracker.register(&view1);
        tracker.register(&view2);

        assert_eq!(tracker.get_dependents("S1", "f1").len(), 2);

        tracker.unregister("View1");
        let deps = tracker.get_dependents("S1", "f1");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], "View2");
    }

    #[test]
    fn test_rebuild() {
        let mut tracker = DependencyTracker::new();
        let old_view = make_view("OldView", vec![("S1", vec!["f1"])]);
        tracker.register(&old_view);

        let new_view = make_view("NewView", vec![("S2", vec!["f2"])]);
        tracker.rebuild(&[new_view]);

        assert!(tracker.get_dependents("S1", "f1").is_empty());
        assert_eq!(tracker.get_dependents("S2", "f2").len(), 1);
    }

    #[test]
    fn test_no_cycle_with_plain_schemas() {
        let tracker = DependencyTracker::new();
        let view = make_view("MyView", vec![("PlainSchema", vec!["f1"])]);
        let existing = HashMap::new();
        assert!(!tracker.would_create_cycle("MyView", &view, &existing));
    }

    #[test]
    fn test_direct_cycle() {
        let tracker = DependencyTracker::new();

        // ViewA reads from ViewB
        let view_a = make_view("ViewA", vec![("ViewB", vec!["x"])]);
        let mut existing = HashMap::new();
        existing.insert("ViewA".to_string(), view_a);

        // ViewB wants to read from ViewA — should detect cycle
        let view_b = make_view("ViewB", vec![("ViewA", vec!["y"])]);
        assert!(tracker.would_create_cycle("ViewB", &view_b, &existing));
    }

    #[test]
    fn test_transitive_cycle() {
        let tracker = DependencyTracker::new();

        let view_a = make_view("ViewA", vec![("ViewB", vec!["x"])]);
        let view_b = make_view("ViewB", vec![("ViewC", vec!["y"])]);

        let mut existing = HashMap::new();
        existing.insert("ViewA".to_string(), view_a);
        existing.insert("ViewB".to_string(), view_b);

        // ViewC wants to read from ViewA — transitive cycle: C -> A -> B -> C
        let view_c = make_view("ViewC", vec![("ViewA", vec!["z"])]);
        assert!(tracker.would_create_cycle("ViewC", &view_c, &existing));
    }

    #[test]
    fn test_no_cycle_diamond() {
        let tracker = DependencyTracker::new();

        let view_a = make_view("ViewA", vec![("S1", vec!["f1"])]);
        let view_b = make_view("ViewB", vec![("S1", vec!["f1"])]);

        let mut existing = HashMap::new();
        existing.insert("ViewA".to_string(), view_a);
        existing.insert("ViewB".to_string(), view_b);

        // ViewC reads from ViewA and ViewB — no cycle
        let view_c = make_view("ViewC", vec![("ViewA", vec!["a"]), ("ViewB", vec!["b"])]);
        assert!(!tracker.would_create_cycle("ViewC", &view_c, &existing));
    }

    #[test]
    fn test_get_all_dependents_of_schema() {
        let mut tracker = DependencyTracker::new();
        let view1 = make_view("View1", vec![("S1", vec!["f1", "f2"])]);
        let view2 = make_view("View2", vec![("S1", vec!["f3"])]);
        let view3 = make_view("View3", vec![("S2", vec!["g1"])]);
        tracker.register(&view1);
        tracker.register(&view2);
        tracker.register(&view3);

        let mut deps = tracker.get_all_dependents_of_schema("S1");
        deps.sort();
        assert_eq!(deps, vec!["View1", "View2"]);

        let deps2 = tracker.get_all_dependents_of_schema("S2");
        assert_eq!(deps2, vec!["View3"]);

        assert!(tracker.get_all_dependents_of_schema("S99").is_empty());
    }

    #[test]
    fn test_multi_field_query_dependencies() {
        let mut tracker = DependencyTracker::new();
        let view = make_view(
            "Dashboard",
            vec![
                ("BlogPost", vec!["title", "content"]),
                ("Author", vec!["name"]),
            ],
        );
        tracker.register(&view);

        assert_eq!(tracker.get_dependents("BlogPost", "title").len(), 1);
        assert_eq!(tracker.get_dependents("BlogPost", "content").len(), 1);
        assert_eq!(tracker.get_dependents("Author", "name").len(), 1);
    }

    #[test]
    fn test_depth_limit_exceeded() {
        let tracker = DependencyTracker::new();

        // Build a chain of MAX_DEPTH + 1 views: V0 → V1 → V2 → ... → V(MAX_DEPTH)
        let mut existing = HashMap::new();
        for i in 0..DependencyTracker::MAX_DEPTH {
            let source = if i == 0 {
                "BaseSchema".to_string()
            } else {
                format!("V{}", i - 1)
            };
            let view = make_view(&format!("V{}", i), vec![(&source, vec!["f1"])]);
            existing.insert(format!("V{}", i), view);
        }

        // Adding one more layer should be rejected (depth limit)
        let last = format!("V{}", DependencyTracker::MAX_DEPTH - 1);
        let new_view = make_view("VTooDeep", vec![(&last, vec!["f1"])]);
        assert!(
            tracker.would_create_cycle("VTooDeep", &new_view, &existing),
            "Should reject view chain exceeding MAX_DEPTH"
        );
    }

    #[test]
    fn test_depth_at_limit_is_allowed() {
        let tracker = DependencyTracker::new();

        // Build a chain of exactly MAX_DEPTH - 1 views (so the new view is at depth MAX_DEPTH)
        let mut existing = HashMap::new();
        for i in 0..(DependencyTracker::MAX_DEPTH - 1) {
            let source = if i == 0 {
                "BaseSchema".to_string()
            } else {
                format!("V{}", i - 1)
            };
            let view = make_view(&format!("V{}", i), vec![(&source, vec!["f1"])]);
            existing.insert(format!("V{}", i), view);
        }

        // Adding at exactly MAX_DEPTH should be allowed
        let last = format!("V{}", DependencyTracker::MAX_DEPTH - 2);
        let new_view = make_view("VAtLimit", vec![(&last, vec!["f1"])]);
        assert!(
            !tracker.would_create_cycle("VAtLimit", &new_view, &existing),
            "Should allow view chain at exactly MAX_DEPTH"
        );
    }
}
