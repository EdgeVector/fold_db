//! Trigger configuration attached to a view.
//!
//! A view declares zero or more triggers. `TriggerRunner` consults these to
//! decide when to fire the view's materialization. Phase 1 supports:
//!
//! - `OnWrite`: fire immediately on any mutation against a source schema.
//! - `OnWriteCoalesced { min_batch, debounce_ms, max_wait_ms }`: batch
//!   mutations; fire once thresholds are crossed.
//! - `Scheduled { interval_ms }`: fire every `interval_ms`, regardless of
//!   mutation activity.
//! - `ScheduledIfDirty { interval_ms }`: at each interval tick, fire only
//!   if a source mutation has set the dirty flag since the last fire.
//! - `Manual`: only fires via the explicit run endpoint (Phase 2+).
//!
//! If a view is registered with an empty `triggers` vec, it is treated as
//! `[Trigger::OnWrite]` (backwards-compatible default — every pre-trigger
//! view will continue to invalidate on mutation).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Trigger {
    OnWrite,
    OnWriteCoalesced {
        min_batch: u32,
        debounce_ms: u64,
        max_wait_ms: u64,
    },
    Scheduled {
        interval_ms: u64,
    },
    ScheduledIfDirty {
        interval_ms: u64,
    },
    Manual,
}

impl Trigger {
    /// True when a mutation against a source schema should route to this
    /// trigger's runtime path (OnWrite + OnWriteCoalesced). Scheduled and
    /// ScheduledIfDirty only consume mutation notifications to flip the
    /// dirty flag — the scheduler tick does the actual firing.
    pub fn is_write_triggered(&self) -> bool {
        matches!(
            self,
            Trigger::OnWrite | Trigger::OnWriteCoalesced { .. } | Trigger::ScheduledIfDirty { .. }
        )
    }

    /// True when the scheduler tracks this trigger in its min-heap.
    pub fn is_scheduled(&self) -> bool {
        matches!(
            self,
            Trigger::Scheduled { .. } | Trigger::ScheduledIfDirty { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_write_is_write_triggered_not_scheduled() {
        let t = Trigger::OnWrite;
        assert!(t.is_write_triggered());
        assert!(!t.is_scheduled());
    }

    #[test]
    fn scheduled_if_dirty_is_both() {
        let t = Trigger::ScheduledIfDirty { interval_ms: 1000 };
        assert!(t.is_write_triggered());
        assert!(t.is_scheduled());
    }

    #[test]
    fn scheduled_only_scheduled() {
        let t = Trigger::Scheduled { interval_ms: 5000 };
        assert!(!t.is_write_triggered());
        assert!(t.is_scheduled());
    }

    #[test]
    fn manual_is_neither() {
        let t = Trigger::Manual;
        assert!(!t.is_write_triggered());
        assert!(!t.is_scheduled());
    }

    #[test]
    fn serializes_with_kind_tag() {
        let t = Trigger::OnWriteCoalesced {
            min_batch: 10,
            debounce_ms: 500,
            max_wait_ms: 5000,
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains(r#""kind":"on_write_coalesced""#));
        let round: Trigger = serde_json::from_str(&json).unwrap();
        assert_eq!(round, t);
    }
}
