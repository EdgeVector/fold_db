use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::AccessDecision;

/// What kind of action was audited
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    Read {
        schema_name: String,
        fields: Vec<String>,
    },
    Write {
        schema_name: String,
        fields: Vec<String>,
    },
    AccessDenied {
        schema_name: String,
        reason: String,
    },
    TrustGrant {
        user_id: String,
        distance: u64,
    },
    TrustRevoke {
        user_id: String,
    },
}

/// A single audit event recording an access control decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub action: AuditAction,
    pub trust_distance: Option<u64>,
    pub decision_granted: bool,
}

impl AuditEvent {
    pub fn new(
        user_id: impl Into<String>,
        action: AuditAction,
        trust_distance: Option<u64>,
        decision: &AccessDecision,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            user_id: user_id.into(),
            action,
            trust_distance,
            decision_granted: decision.is_granted(),
        }
    }

    /// Create a trust management event (always granted — it's the owner doing it)
    pub fn trust_event(user_id: impl Into<String>, action: AuditAction) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            user_id: user_id.into(),
            action,
            trust_distance: Some(0),
            decision_granted: true,
        }
    }
}

/// Append-only audit log. Persisted to Sled via `DbOperations`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditLog {
    events: Vec<AuditEvent>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record(&mut self, event: AuditEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    pub fn events_for_user(&self, user_id: &str) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.user_id == user_id)
            .collect()
    }

    pub fn events_for_schema(&self, schema_name: &str) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| match &e.action {
                AuditAction::Read { schema_name: s, .. } => s == schema_name,
                AuditAction::Write { schema_name: s, .. } => s == schema_name,
                AuditAction::AccessDenied { schema_name: s, .. } => s == schema_name,
                AuditAction::TrustGrant { .. } | AuditAction::TrustRevoke { .. } => false,
            })
            .collect()
    }

    pub fn total_events(&self) -> usize {
        self.events.len()
    }

    /// Get the most recent N events
    pub fn recent(&self, n: usize) -> &[AuditEvent] {
        let start = self.events.len().saturating_sub(n);
        &self.events[start..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_record_and_query() {
        let mut log = AuditLog::new();

        log.record(AuditEvent::new(
            "alice",
            AuditAction::Read {
                schema_name: "contacts".into(),
                fields: vec!["name".into(), "email".into()],
            },
            Some(0),
            &AccessDecision::Granted,
        ));

        log.record(AuditEvent::new(
            "bob",
            AuditAction::AccessDenied {
                schema_name: "contacts".into(),
                reason: "trust distance too high".into(),
            },
            Some(5),
            &AccessDecision::Denied(super::super::types::AccessDenialReason::TrustDistance {
                required: 2,
                actual: 5,
            }),
        ));

        assert_eq!(log.total_events(), 2);
        assert_eq!(log.events_for_user("alice").len(), 1);
        assert_eq!(log.events_for_user("bob").len(), 1);
        assert_eq!(log.events_for_schema("contacts").len(), 2);
        assert!(log.events_for_user("alice")[0].decision_granted);
        assert!(!log.events_for_user("bob")[0].decision_granted);
    }

    #[test]
    fn test_trust_event() {
        let event = AuditEvent::trust_event(
            "alice",
            AuditAction::TrustGrant {
                user_id: "bob".into(),
                distance: 2,
            },
        );
        assert!(event.decision_granted);
        assert_eq!(event.trust_distance, Some(0));
    }

    #[test]
    fn test_recent() {
        let mut log = AuditLog::new();
        for i in 0..10 {
            log.record(AuditEvent::new(
                format!("user_{}", i),
                AuditAction::Read {
                    schema_name: "test".into(),
                    fields: vec![],
                },
                Some(0),
                &AccessDecision::Granted,
            ));
        }
        assert_eq!(log.recent(3).len(), 3);
        assert_eq!(log.recent(100).len(), 10);
    }

    #[test]
    fn test_serialization() {
        let mut log = AuditLog::new();
        log.record(AuditEvent::new(
            "alice",
            AuditAction::Write {
                schema_name: "notes".into(),
                fields: vec!["content".into()],
            },
            Some(0),
            &AccessDecision::Granted,
        ));

        let json = serde_json::to_string(&log).unwrap();
        let deserialized: AuditLog = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_events(), 1);
    }
}
