//! TriggerFiring schema — internal log of every view trigger firing.
//!
//! TriggerRunner (Phase 1 task 3) writes one row here per attempt, win or
//! lose. Rows are keyed by (trigger_id, fired_at) so callers can list a
//! trigger's history in time order without scanning the whole log.
//!
//! Registered at FoldDB startup via [`register_trigger_firing_schema`].
//! Idempotent: if the schema already exists in the store, load_schema
//! refreshes the in-memory cache without clobbering on-disk state.

pub mod clock;
pub mod simulate;
pub mod types;

pub use clock::{Clock, MockClock, SystemClock};
pub use simulate::{next_fire_from_cron, should_coalesce_fire};
pub use types::Trigger;

use std::sync::Arc;

use crate::schema::types::data_classification::{DataClassification, INTERNAL};
use crate::schema::types::field_value_type::FieldValueType;
use crate::schema::types::key_config::KeyConfig;
use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
use crate::schema::types::Schema;
use crate::schema::{SchemaCore, SchemaError, SchemaState};

pub const TRIGGER_FIRING_SCHEMA_NAME: &str = "TriggerFiring";

/// Field names on the TriggerFiring schema. Exposed so the runner can
/// reference them by identifier instead of stringly-typed literals.
pub mod fields {
    pub const TRIGGER_ID: &str = "trigger_id";
    pub const VIEW_NAME: &str = "view_name";
    pub const FIRED_AT: &str = "fired_at";
    pub const DURATION_MS: &str = "duration_ms";
    pub const STATUS: &str = "status";
    pub const INPUT_ROW_COUNT: &str = "input_row_count";
    pub const OUTPUT_ROW_COUNT: &str = "output_row_count";
    pub const ERROR_MESSAGE: &str = "error_message";
    pub const SKIP_REASON: &str = "skip_reason";
}

/// Status values written to the `status` field.
pub mod status {
    pub const SUCCESS: &str = "success";
    pub const ERROR: &str = "error";
    pub const QUARANTINED: &str = "quarantined";
    /// Scheduler decided not to dispatch this tick. Paired with a
    /// `skip_reason` field so operators can filter by cause without
    /// string-parsing the status column.
    pub const SKIPPED: &str = "skipped";
}

/// Reason values written to the `skip_reason` field when `status = "skipped"`.
///
/// Stable identifiers so operator queries can filter by cause. Values are
/// closed-set: a new skip site requires a new constant here and a matching
/// `SkipReason` variant in the runner.
pub mod skip_reason {
    /// `Scheduled { skip_if_idle: true, .. }` tick with a clean dirty
    /// bit — no mutation since last fire, tick is a no-op.
    pub const SKIP_IF_IDLE: &str = "skip_if_idle";
    /// `ScheduledIfDirty` tick with a clean dirty bit. Same semantic as
    /// `skip_if_idle` but via the explicit ScheduledIfDirty variant, so
    /// operators can distinguish the two trigger shapes in the audit log.
    pub const DIRTY_CLEAN: &str = "dirty_clean";
    /// Scheduled fire slipped further past its nominal time than the
    /// trigger's `max_catch_up_age` allows — dispatch was skipped to
    /// bound fire storms after process downtime.
    pub const CATCH_UP_BUDGET: &str = "catch_up_budget";
}

/// Build the TriggerFiring schema definition.
///
/// Shape: HashRange keyed by (trigger_id, fired_at) so a trigger's
/// firing history is naturally clustered and range-scannable.
pub fn trigger_firing_schema() -> Schema {
    let all_fields = [
        (
            fields::TRIGGER_ID,
            FieldValueType::String,
            "Stable id derived from the trigger config (e.g. `{view_id}:{index}`)",
        ),
        (
            fields::VIEW_NAME,
            FieldValueType::String,
            "Name of the view that fired",
        ),
        (
            fields::FIRED_AT,
            FieldValueType::Integer,
            "Milliseconds since Unix epoch when the firing began",
        ),
        (
            fields::DURATION_MS,
            FieldValueType::Integer,
            "How long the firing took, in milliseconds",
        ),
        (
            fields::STATUS,
            FieldValueType::String,
            "Outcome: \"success\" | \"error\" | \"quarantined\" | \"skipped\"",
        ),
        (
            fields::INPUT_ROW_COUNT,
            FieldValueType::Integer,
            "Rows read from source schemas",
        ),
        (
            fields::OUTPUT_ROW_COUNT,
            FieldValueType::Integer,
            "Rows written to the output schema",
        ),
        (
            fields::ERROR_MESSAGE,
            FieldValueType::OneOf(vec![FieldValueType::String, FieldValueType::Null]),
            "Error detail when status != \"success\"",
        ),
        (
            fields::SKIP_REASON,
            FieldValueType::OneOf(vec![FieldValueType::String, FieldValueType::Null]),
            "Reason when status == \"skipped\": \"skip_if_idle\" | \"dirty_clean\" | \"catch_up_budget\"; Null otherwise",
        ),
    ];

    let field_names: Vec<String> = all_fields.iter().map(|(n, _, _)| n.to_string()).collect();

    let mut schema = Schema::new(
        TRIGGER_FIRING_SCHEMA_NAME.to_string(),
        SchemaType::HashRange,
        Some(KeyConfig::new(
            Some(fields::TRIGGER_ID.to_string()),
            Some(fields::FIRED_AT.to_string()),
        )),
        Some(field_names),
        None,
        None,
    );

    schema.descriptive_name = Some(TRIGGER_FIRING_SCHEMA_NAME.to_string());

    for (name, ty, description) in all_fields {
        schema.field_types.insert(name.to_string(), ty);
        schema
            .field_descriptions
            .insert(name.to_string(), description.to_string());
        schema.field_data_classifications.insert(
            name.to_string(),
            DataClassification {
                sensitivity_level: INTERNAL,
                data_domain: "general".to_string(),
            },
        );
        schema
            .field_classifications
            .insert(name.to_string(), vec!["word".to_string()]);
    }

    schema.compute_identity_hash();
    schema
}

/// Register the TriggerFiring schema with the local SchemaCore and
/// auto-approve it so the runner can write rows immediately.
///
/// Idempotent — safe to call on every boot. If the schema is already
/// present in persistent storage, `load_schema_internal` refreshes the
/// in-memory cache and `set_schema_state(Approved)` is a no-op.
pub async fn register_trigger_firing_schema(
    schema_manager: &Arc<SchemaCore>,
) -> Result<(), SchemaError> {
    schema_manager
        .load_schema_internal(trigger_firing_schema())
        .await?;
    schema_manager
        .set_schema_state(TRIGGER_FIRING_SCHEMA_NAME, SchemaState::Approved)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_has_expected_fields_and_shape() {
        let schema = trigger_firing_schema();

        assert_eq!(schema.name, TRIGGER_FIRING_SCHEMA_NAME);
        assert_eq!(schema.schema_type, SchemaType::HashRange);

        let key = schema.key.as_ref().expect("HashRange schema needs a key");
        assert_eq!(key.hash_field.as_deref(), Some(fields::TRIGGER_ID));
        assert_eq!(key.range_field.as_deref(), Some(fields::FIRED_AT));

        let mut declared: Vec<&str> = schema
            .fields
            .as_ref()
            .expect("fields should be populated")
            .iter()
            .map(String::as_str)
            .collect();
        declared.sort();
        let mut expected = vec![
            fields::TRIGGER_ID,
            fields::VIEW_NAME,
            fields::FIRED_AT,
            fields::DURATION_MS,
            fields::STATUS,
            fields::INPUT_ROW_COUNT,
            fields::OUTPUT_ROW_COUNT,
            fields::ERROR_MESSAGE,
            fields::SKIP_REASON,
        ];
        expected.sort();
        assert_eq!(declared, expected);
    }

    #[test]
    fn error_message_is_nullable_string() {
        let schema = trigger_firing_schema();
        let ty = schema.field_types.get(fields::ERROR_MESSAGE).unwrap();
        match ty {
            FieldValueType::OneOf(variants) => {
                assert!(variants.contains(&FieldValueType::String));
                assert!(variants.contains(&FieldValueType::Null));
            }
            other => panic!("expected OneOf(String, Null), got {:?}", other),
        }
    }

    #[test]
    fn skip_reason_is_nullable_string() {
        let schema = trigger_firing_schema();
        let ty = schema.field_types.get(fields::SKIP_REASON).unwrap();
        match ty {
            FieldValueType::OneOf(variants) => {
                assert!(variants.contains(&FieldValueType::String));
                assert!(variants.contains(&FieldValueType::Null));
            }
            other => panic!("expected OneOf(String, Null), got {:?}", other),
        }
    }

    #[test]
    fn count_and_time_fields_are_integer() {
        let schema = trigger_firing_schema();
        for name in [
            fields::FIRED_AT,
            fields::DURATION_MS,
            fields::INPUT_ROW_COUNT,
            fields::OUTPUT_ROW_COUNT,
        ] {
            assert_eq!(
                schema.field_types.get(name),
                Some(&FieldValueType::Integer),
                "{} should be Integer",
                name
            );
        }
    }

    #[test]
    fn every_field_has_classification() {
        let schema = trigger_firing_schema();
        for name in schema.fields.as_ref().unwrap() {
            let cls = schema
                .field_data_classifications
                .get(name)
                .unwrap_or_else(|| panic!("{} missing DataClassification", name));
            assert_eq!(cls.sensitivity_level, INTERNAL);
            assert_eq!(cls.data_domain, "general");
        }
    }
}
