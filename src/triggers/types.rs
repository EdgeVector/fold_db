//! Trigger configuration attached to a view.
//!
//! fold_db is the single source of truth for this type. The schema_service
//! validator (Phase 0) imports it and validates the same shape; the
//! fold_db_node adapter that used to translate canonical→exec is gone once
//! consolidation Tasks B + C land.
//!
//! A view declares zero or more triggers. `TriggerRunner` consults these to
//! decide when to fire the view's materialization.
//!
//! - `OnWrite { schemas }`: fire immediately on any mutation against one of
//!   the named source schemas.
//! - `OnWriteCoalesced { schemas, min_batch, debounce_ms, max_wait_ms }`:
//!   batch mutations; fire once thresholds are crossed.
//! - `Scheduled { cron, timezone, window, skip_if_idle, schemas }`: fire on
//!   a cron schedule in the given IANA timezone. When `window` is set, it
//!   auto-injects a time filter into input queries at fire time (e.g.
//!   `"24h"`, `"7d"`). `skip_if_idle` suppresses the tick when no source
//!   mutations have landed since the last fire.
//! - `ScheduledIfDirty { cron, timezone, window, schemas }`: like
//!   `Scheduled` but only fires when the per-view dirty bit is set.
//! - `Manual`: only fires via explicit run (future endpoint).
//!
//! If a view is registered with an empty `triggers` vec, `effective_triggers`
//! returns `[OnWrite { schemas: <view's source schemas> }]` so every
//! pre-trigger view continues to invalidate on mutation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Trigger {
    OnWrite {
        schemas: Vec<String>,
    },
    OnWriteCoalesced {
        schemas: Vec<String>,
        min_batch: u32,
        debounce_ms: u64,
        max_wait_ms: u64,
    },
    Scheduled {
        cron: String,
        timezone: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window: Option<String>,
        skip_if_idle: bool,
        schemas: Vec<String>,
    },
    ScheduledIfDirty {
        cron: String,
        timezone: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window: Option<String>,
        schemas: Vec<String>,
    },
    Manual,
}

impl Trigger {
    /// True when a mutation against a source schema should route to this
    /// trigger's runtime path (OnWrite + OnWriteCoalesced). ScheduledIfDirty
    /// consumes mutation notifications to flip the dirty flag — the
    /// scheduler tick does the actual firing.
    pub fn is_write_triggered(&self) -> bool {
        matches!(
            self,
            Trigger::OnWrite { .. }
                | Trigger::OnWriteCoalesced { .. }
                | Trigger::ScheduledIfDirty { .. }
        )
    }

    /// True when the scheduler tracks this trigger.
    pub fn is_scheduled(&self) -> bool {
        matches!(
            self,
            Trigger::Scheduled { .. } | Trigger::ScheduledIfDirty { .. }
        )
    }

    /// Source schemas this trigger subscribes to. `Manual` returns an empty
    /// slice — it has no mutation subscription.
    pub fn schemas(&self) -> &[String] {
        match self {
            Trigger::OnWrite { schemas }
            | Trigger::OnWriteCoalesced { schemas, .. }
            | Trigger::Scheduled { schemas, .. }
            | Trigger::ScheduledIfDirty { schemas, .. } => schemas.as_slice(),
            Trigger::Manual => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_write_is_write_triggered_not_scheduled() {
        let t = Trigger::OnWrite {
            schemas: vec!["S".into()],
        };
        assert!(t.is_write_triggered());
        assert!(!t.is_scheduled());
    }

    #[test]
    fn scheduled_if_dirty_is_both() {
        let t = Trigger::ScheduledIfDirty {
            cron: "0 * * * *".into(),
            timezone: "UTC".into(),
            window: None,
            schemas: vec!["S".into()],
        };
        assert!(t.is_write_triggered());
        assert!(t.is_scheduled());
    }

    #[test]
    fn scheduled_only_scheduled() {
        let t = Trigger::Scheduled {
            cron: "0 * * * *".into(),
            timezone: "UTC".into(),
            window: None,
            skip_if_idle: false,
            schemas: vec!["S".into()],
        };
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
    fn schemas_accessor_returns_vec() {
        let t = Trigger::OnWrite {
            schemas: vec!["A".into(), "B".into()],
        };
        assert_eq!(t.schemas(), &["A".to_string(), "B".to_string()]);
        let m = Trigger::Manual;
        assert!(m.schemas().is_empty());
    }

    #[test]
    fn serializes_with_kind_tag_camel_case() {
        let t = Trigger::OnWriteCoalesced {
            schemas: vec!["S1".into()],
            min_batch: 10,
            debounce_ms: 500,
            max_wait_ms: 5000,
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(
            json.contains(r#""kind":"OnWriteCoalesced""#),
            "canonical JSON uses CamelCase tag values: {json}"
        );
        let round: Trigger = serde_json::from_str(&json).unwrap();
        assert_eq!(round, t);
    }

    #[test]
    fn scheduled_roundtrips_with_optional_window() {
        let t = Trigger::Scheduled {
            cron: "0 2 * * *".into(),
            timezone: "America/New_York".into(),
            window: Some("24h".into()),
            skip_if_idle: true,
            schemas: vec!["Source".into()],
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains(r#""kind":"Scheduled""#));
        assert!(json.contains(r#""window":"24h""#));
        let round: Trigger = serde_json::from_str(&json).unwrap();
        assert_eq!(round, t);

        // Window omitted on serialize when None.
        let t_no_window = Trigger::Scheduled {
            cron: "0 2 * * *".into(),
            timezone: "UTC".into(),
            window: None,
            skip_if_idle: false,
            schemas: vec!["S".into()],
        };
        let json = serde_json::to_string(&t_no_window).unwrap();
        assert!(!json.contains("window"), "None window must be omitted: {json}");
    }

    #[test]
    fn manual_serializes_bare() {
        let json = serde_json::to_string(&Trigger::Manual).unwrap();
        assert_eq!(json, r#"{"kind":"Manual"}"#);
    }
}
