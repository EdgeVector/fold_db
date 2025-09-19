//! Common Statistics Framework for Event-Driven Components
//!
//! This module provides a unified statistics framework that can be shared
//! across all event-driven components in FoldDB.

use std::time::Instant;

/// Base statistics that all event-driven components share
#[derive(Debug, Clone, Default)]
pub struct EventDrivenBaseStats {
    pub requests_processed: u64,
    pub requests_failed: u64,
    pub last_activity: Option<Instant>,
}

impl EventDrivenBaseStats {
    /// Create new base stats with current timestamp
    pub fn new() -> Self {
        Self {
            requests_processed: 0,
            requests_failed: 0,
            last_activity: Some(Instant::now()),
        }
    }

    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Some(Instant::now());
    }

    /// Increment successful request counter
    pub fn increment_processed(&mut self) {
        self.requests_processed += 1;
        self.update_activity();
    }

    /// Increment failed request counter
    pub fn increment_failed(&mut self) {
        self.requests_failed += 1;
        self.update_activity();
    }
}

/// Unified statistics for AtomManager operations
#[derive(Debug, Clone, Default)]
pub struct EventDrivenAtomStats {
    pub atoms_created: u64,
    pub atoms_updated: u64,
    pub molecules_created: u64,
    pub molecules_updated: u64,
    pub requests_processed: u64,
    pub requests_failed: u64,
    pub last_activity: Option<Instant>,
}

impl EventDrivenAtomStats {
    pub fn new() -> Self {
        Self {
            last_activity: Some(Instant::now()),
            ..Default::default()
        }
    }
}

/// Unified statistics for FieldManager operations
#[derive(Debug, Clone, Default)]
pub struct EventDrivenFieldStats {
    pub field_sets_processed: u64,
    pub field_updates_processed: u64,
    pub requests_sent: u64,
    pub responses_received: u64,
    pub timeouts: u64,
    pub errors: u64,
    pub requests_processed: u64,
    pub requests_failed: u64,
    pub last_activity: Option<Instant>,
}

impl EventDrivenFieldStats {
    pub fn new() -> Self {
        Self {
            last_activity: Some(Instant::now()),
            ..Default::default()
        }
    }
}

/// Unified statistics for SchemaManager operations
#[derive(Debug, Clone, Default)]
pub struct EventDrivenSchemaStats {
    pub schemas_loaded: u64,
    pub schemas_approved: u64,
    pub schemas_blocked: u64,
    pub requests_processed: u64,
    pub requests_failed: u64,
    pub last_activity: Option<Instant>,
}

impl EventDrivenSchemaStats {
    pub fn new() -> Self {
        Self {
            last_activity: Some(Instant::now()),
            ..Default::default()
        }
    }
}

/// Unified statistics for FoldDB operations
#[derive(Debug, Clone, Default)]
pub struct EventDrivenFoldDBStats {
    pub mutations_processed: u64,
    pub queries_processed: u64,
    pub schema_operations: u64,
    pub event_requests_sent: u64,
    pub event_responses_received: u64,
    pub timeouts: u64,
    pub errors: u64,
    pub requests_processed: u64,
    pub requests_failed: u64,
    pub last_activity: Option<Instant>,
}

impl EventDrivenFoldDBStats {
    pub fn new() -> Self {
        Self {
            last_activity: Some(Instant::now()),
            ..Default::default()
        }
    }
}

/// Unified statistics for TransformManager operations
#[derive(Debug, Clone, Default)]
pub struct EventDrivenTransformStats {
    pub transforms_stored: u64,
    pub transforms_loaded: u64,
    pub transforms_deleted: u64,
    pub transforms_executed: u64,
    pub transform_lists_generated: u64,
    pub transform_mappings_stored: u64,
    pub transform_mappings_retrieved: u64,
    pub transform_triggers_processed: u64,
    pub requests_processed: u64,
    pub requests_failed: u64,
    pub last_activity: Option<Instant>,
}

impl EventDrivenTransformStats {
    pub fn new() -> Self {
        Self {
            last_activity: Some(Instant::now()),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_atom_stats() {
        let mut stats = EventDrivenAtomStats::new();
        assert_eq!(stats.requests_processed, 0);
        assert_eq!(stats.atoms_created, 0);

        stats.requests_processed += 1;
        stats.atoms_created += 1;

        assert_eq!(stats.requests_processed, 1);
        assert_eq!(stats.atoms_created, 1);
    }

    #[test]
    fn test_field_stats() {
        let mut stats = EventDrivenFieldStats::new();
        assert_eq!(stats.requests_processed, 0);
        assert_eq!(stats.field_sets_processed, 0);

        stats.requests_processed += 1;
        stats.field_sets_processed += 1;

        assert_eq!(stats.requests_processed, 1);
        assert_eq!(stats.field_sets_processed, 1);
    }

    #[test]
    fn test_schema_stats() {
        let mut stats = EventDrivenSchemaStats::new();
        assert_eq!(stats.requests_processed, 0);
        assert_eq!(stats.schemas_loaded, 0);

        stats.requests_processed += 1;
        stats.schemas_loaded += 1;

        assert_eq!(stats.requests_processed, 1);
        assert_eq!(stats.schemas_loaded, 1);
    }

    #[test]
    fn test_folddb_stats() {
        let mut stats = EventDrivenFoldDBStats::new();
        assert_eq!(stats.requests_processed, 0);
        assert_eq!(stats.mutations_processed, 0);

        stats.requests_processed += 1;
        stats.mutations_processed += 1;

        assert_eq!(stats.requests_processed, 1);
        assert_eq!(stats.mutations_processed, 1);
    }

    #[test]
    fn test_activity_timestamp() {
        let mut stats = EventDrivenAtomStats::new();
        let initial_time = stats.last_activity.unwrap();

        thread::sleep(Duration::from_millis(10));
        stats.last_activity = Some(Instant::now());

        let updated_time = stats.last_activity.unwrap();
        assert!(updated_time > initial_time);
    }

    #[test]
    fn test_transform_stats() {
        let mut stats = EventDrivenTransformStats::new();
        assert_eq!(stats.requests_processed, 0);
        assert_eq!(stats.transforms_stored, 0);
        assert_eq!(stats.transforms_executed, 0);

        stats.requests_processed += 1;
        stats.transforms_stored += 1;
        stats.transforms_executed += 1;

        assert_eq!(stats.requests_processed, 1);
        assert_eq!(stats.transforms_stored, 1);
        assert_eq!(stats.transforms_executed, 1);
    }

    #[test]
    fn test_transform_stats_comprehensive() {
        let mut stats = EventDrivenTransformStats::new();

        // Test all transform-specific counters
        stats.transforms_loaded += 2;
        stats.transforms_deleted += 1;
        stats.transform_lists_generated += 3;
        stats.transform_mappings_stored += 4;
        stats.transform_mappings_retrieved += 5;
        stats.transform_triggers_processed += 6;

        assert_eq!(stats.transforms_loaded, 2);
        assert_eq!(stats.transforms_deleted, 1);
        assert_eq!(stats.transform_lists_generated, 3);
        assert_eq!(stats.transform_mappings_stored, 4);
        assert_eq!(stats.transform_mappings_retrieved, 5);
        assert_eq!(stats.transform_triggers_processed, 6);
    }
}
