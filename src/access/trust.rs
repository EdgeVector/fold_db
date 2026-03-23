use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};


/// Directed weighted graph for trust distance resolution.
///
/// Each edge (A → B, d) means "A trusts B at distance d."
/// `resolve(user, owner)` returns the shortest-path distance from owner to user,
/// or the override distance if one is set.
///
/// Owner's distance to self is always 0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustGraph {
    /// user → [(peer, distance)]
    adjacency: HashMap<String, Vec<(String, u64)>>,
    /// "owner\0user" → forced distance — overrides take precedence over shortest path.
    /// Uses a string key (owner + NUL + user) because JSON can't serialize tuple keys.
    overrides: HashMap<String, u64>,
}

impl TrustGraph {
    pub fn new() -> Self {
        Self {
            adjacency: HashMap::new(),
            overrides: HashMap::new(),
        }
    }

    /// Assign trust: `owner` trusts `user` at `distance`.
    /// Replaces any existing edge from owner to user.
    pub fn assign_trust(&mut self, owner: &str, user: &str, distance: u64) {
        let edges = self.adjacency.entry(owner.to_string()).or_default();
        if let Some(existing) = edges.iter_mut().find(|(peer, _)| peer == user) {
            existing.1 = distance;
        } else {
            edges.push((user.to_string(), distance));
        }
    }

    /// Revoke trust: remove the edge from owner to user and set override to u64::MAX.
    pub fn revoke_trust(&mut self, owner: &str, user: &str) {
        if let Some(edges) = self.adjacency.get_mut(owner) {
            edges.retain(|(peer, _)| peer != user);
        }
        self.overrides
            .insert(Self::override_key(owner, user), u64::MAX);
    }

    /// Set an explicit distance override. Overrides take precedence over graph shortest path.
    pub fn set_override(&mut self, owner: &str, user: &str, distance: u64) {
        self.overrides
            .insert(Self::override_key(owner, user), distance);
    }

    /// Remove an override, falling back to graph-derived distance.
    pub fn remove_override(&mut self, owner: &str, user: &str) {
        self.overrides.remove(&Self::override_key(owner, user));
    }

    /// Resolve the trust distance from `owner` to `user`.
    /// Returns `Some(0)` if user == owner.
    /// Returns `Some(override)` if an explicit override exists.
    /// Otherwise returns the shortest-path distance via Dijkstra, or `None` if unreachable.
    pub fn resolve(&self, user: &str, owner: &str) -> Option<u64> {
        if user == owner {
            return Some(0);
        }

        // Check for explicit override
        let key = Self::override_key(owner, user);
        if let Some(&dist) = self.overrides.get(&key) {
            return if dist == u64::MAX { None } else { Some(dist) };
        }

        self.shortest_path(owner, user)
    }

    fn override_key(owner: &str, user: &str) -> String {
        format!("{}\0{}", owner, user)
    }

    /// Dijkstra's shortest path from `start` to `target`.
    fn shortest_path(&self, start: &str, target: &str) -> Option<u64> {
        let mut distances_owned: HashMap<String, u64> = HashMap::new();
        let mut heap_owned: BinaryHeap<Reverse<(u64, String)>> = BinaryHeap::new();

        distances_owned.insert(start.to_string(), 0);
        heap_owned.push(Reverse((0, start.to_string())));

        while let Some(Reverse((cost, node))) = heap_owned.pop() {
            if node == target {
                return Some(cost);
            }

            if let Some(&best) = distances_owned.get(&node) {
                if cost > best {
                    continue;
                }
            }

            if let Some(edges) = self.adjacency.get(&node) {
                for (neighbor, weight) in edges {
                    let new_cost = cost.saturating_add(*weight);
                    let current = distances_owned.get(neighbor).copied().unwrap_or(u64::MAX);
                    if new_cost < current {
                        distances_owned.insert(neighbor.clone(), new_cost);
                        heap_owned.push(Reverse((new_cost, neighbor.clone())));
                    }
                }
            }
        }

        None
    }

    /// List all trust assignments originating from `owner`.
    pub fn assignments_from(&self, owner: &str) -> Vec<(String, u64)> {
        self.adjacency
            .get(owner)
            .cloned()
            .unwrap_or_default()
    }

    /// List all overrides for `owner`.
    pub fn overrides_for(&self, owner: &str) -> Vec<(String, u64)> {
        let prefix = format!("{}\0", owner);
        self.overrides
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, &d)| {
                let user = k.strip_prefix(&prefix).unwrap_or("").to_string();
                (user, d)
            })
            .collect()
    }
}

impl Default for TrustGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_distance_is_zero() {
        let graph = TrustGraph::new();
        assert_eq!(graph.resolve("alice", "alice"), Some(0));
    }

    #[test]
    fn test_direct_trust() {
        let mut graph = TrustGraph::new();
        graph.assign_trust("alice", "bob", 1);
        assert_eq!(graph.resolve("bob", "alice"), Some(1));
    }

    #[test]
    fn test_unreachable() {
        let graph = TrustGraph::new();
        assert_eq!(graph.resolve("bob", "alice"), None);
    }

    #[test]
    fn test_transitive_trust() {
        let mut graph = TrustGraph::new();
        graph.assign_trust("alice", "bob", 1);
        graph.assign_trust("bob", "charlie", 2);
        // alice → bob (1) → charlie (2) = 3
        assert_eq!(graph.resolve("charlie", "alice"), Some(3));
    }

    #[test]
    fn test_shortest_path_preferred() {
        let mut graph = TrustGraph::new();
        // Long path: alice → bob (5) → charlie (5) = 10
        graph.assign_trust("alice", "bob", 5);
        graph.assign_trust("bob", "charlie", 5);
        // Short path: alice → charlie (3)
        graph.assign_trust("alice", "charlie", 3);
        assert_eq!(graph.resolve("charlie", "alice"), Some(3));
    }

    #[test]
    fn test_override_takes_precedence() {
        let mut graph = TrustGraph::new();
        graph.assign_trust("alice", "bob", 5);
        graph.set_override("alice", "bob", 1);
        assert_eq!(graph.resolve("bob", "alice"), Some(1));
    }

    #[test]
    fn test_remove_override_falls_back_to_graph() {
        let mut graph = TrustGraph::new();
        graph.assign_trust("alice", "bob", 5);
        graph.set_override("alice", "bob", 1);
        assert_eq!(graph.resolve("bob", "alice"), Some(1));
        graph.remove_override("alice", "bob");
        assert_eq!(graph.resolve("bob", "alice"), Some(5));
    }

    #[test]
    fn test_revoke_trust() {
        let mut graph = TrustGraph::new();
        graph.assign_trust("alice", "bob", 1);
        assert_eq!(graph.resolve("bob", "alice"), Some(1));
        graph.revoke_trust("alice", "bob");
        assert_eq!(graph.resolve("bob", "alice"), None);
    }

    #[test]
    fn test_update_trust_distance() {
        let mut graph = TrustGraph::new();
        graph.assign_trust("alice", "bob", 5);
        assert_eq!(graph.resolve("bob", "alice"), Some(5));
        graph.assign_trust("alice", "bob", 2);
        assert_eq!(graph.resolve("bob", "alice"), Some(2));
    }

    #[test]
    fn test_assignments_from() {
        let mut graph = TrustGraph::new();
        graph.assign_trust("alice", "bob", 1);
        graph.assign_trust("alice", "charlie", 3);
        let assignments = graph.assignments_from("alice");
        assert_eq!(assignments.len(), 2);
    }

    #[test]
    fn test_serialization() {
        let mut graph = TrustGraph::new();
        graph.assign_trust("alice", "bob", 1);
        graph.set_override("alice", "charlie", 5);

        let json = serde_json::to_string(&graph).unwrap();
        let deserialized: TrustGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.resolve("bob", "alice"), Some(1));
        assert_eq!(deserialized.resolve("charlie", "alice"), Some(5));
    }

    #[test]
    fn test_saturating_add_no_overflow() {
        let mut graph = TrustGraph::new();
        graph.assign_trust("alice", "bob", u64::MAX - 1);
        graph.assign_trust("bob", "charlie", 2);
        // Saturated cost (u64::MAX) can't improve on default u64::MAX,
        // so Dijkstra won't find the path — returns None (unreachable).
        // This is correct: practically infinite distance = unreachable.
        assert_eq!(graph.resolve("charlie", "alice"), None);
    }
}
