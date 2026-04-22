//! Pure helpers shared between the live `TriggerRunner` and dry-run
//! simulators that preview trigger behavior (e.g. schema_service's
//! `POST /v1/triggers/simulate`).
//!
//! These are deliberately stateless functions with primitive-only
//! signatures so downstream crates can reuse them without taking a
//! dependency on fold_db's internal `PersistedViewState` type.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;

/// Compute the next cron occurrence strictly after `now_ms` in the given
/// IANA timezone. Returns the fire time as Unix epoch milliseconds (UTC),
/// or `None` if the cron expression or timezone fail to parse.
///
/// DST handling follows croner: `find_next_occurrence(…, false)` advances
/// to the first valid tick after a spring-forward gap, and fires once per
/// local clock-time even during a fall-back overlap (croner walks UTC
/// under the hood, so the ambiguous hour doesn't double-fire).
pub fn next_fire_from_cron(cron_expr: &str, tz_str: &str, now_ms: i64) -> Option<i64> {
    let cron = croner::Cron::new(cron_expr).parse().ok()?;
    let tz: Tz = tz_str.parse().ok()?;
    let now_utc = DateTime::<Utc>::from_timestamp_millis(now_ms)?;
    let now_in_tz = now_utc.with_timezone(&tz);
    let next = cron.find_next_occurrence(&now_in_tz, false).ok()?;
    Some(next.with_timezone(&Utc).timestamp_millis())
}

/// Shared fire predicate for `OnWriteCoalesced` triggers.
///
/// Fires when we have a full batch past the debounce window, OR when
/// `max_wait_ms` has elapsed since the first pending event. Returns
/// `false` on an empty batch — both the mutation-notified and
/// scheduler-tick paths rely on this gate to avoid dispatching with
/// nothing pending.
///
/// Parameterized on raw values (not `&PersistedViewState`) so dry-run
/// simulators outside fold_db can call it directly.
pub fn should_coalesce_fire(
    pending: u32,
    first_event_ms: i64,
    last_event_ms: i64,
    now_ms: i64,
    min_batch: u32,
    debounce_ms: u64,
    max_wait_ms: u64,
) -> bool {
    if pending == 0 {
        return false;
    }
    let batch_ok = pending >= min_batch;
    let debounce_ok = (now_ms - last_event_ms) >= debounce_ms as i64 && batch_ok;
    let max_wait_ok = (now_ms - first_event_ms) >= max_wait_ms as i64;
    (batch_ok && debounce_ok) || max_wait_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_fire_from_cron_parses_and_steps_forward() {
        // 1970-01-01 00:00:00 UTC → next "0 2 * * *" is 1970-01-01 02:00 UTC.
        let next = next_fire_from_cron("0 2 * * *", "UTC", 0).expect("cron parse");
        assert_eq!(next, 2 * 3_600 * 1_000);
    }

    #[test]
    fn next_fire_from_cron_returns_none_on_invalid_cron() {
        assert!(next_fire_from_cron("not a cron", "UTC", 0).is_none());
    }

    #[test]
    fn next_fire_from_cron_returns_none_on_invalid_tz() {
        assert!(next_fire_from_cron("0 2 * * *", "Not/AZone", 0).is_none());
    }

    #[test]
    fn should_coalesce_fire_empty_batch_never_fires() {
        // The empty-batch guard is global — even with max_wait elapsed,
        // pending==0 must return false or the runtime would dispatch on
        // a batch that has nothing to coalesce.
        assert!(!should_coalesce_fire(
            0, 1_000, 1_000, 99_999, 3, 100, 10_000
        ));
    }

    #[test]
    fn should_coalesce_fire_batch_met_but_no_debounce() {
        // 3 events (min_batch=3), last event at 1_050, now at 1_100:
        // only 50ms of quiet — debounce window of 100ms not satisfied.
        assert!(!should_coalesce_fire(
            3, 1_000, 1_050, 1_100, 3, 100, 10_000
        ));
    }

    #[test]
    fn should_coalesce_fire_batch_and_debounce() {
        // Same batch, now at 1_200: 150ms of quiet past min_batch → fire.
        assert!(should_coalesce_fire(3, 1_000, 1_050, 1_200, 3, 100, 10_000));
    }

    #[test]
    fn should_coalesce_fire_max_wait_alone() {
        // 1 event, min_batch=3 (not met), but max_wait (10s) elapsed from
        // the first event → fire anyway to bound latency.
        assert!(should_coalesce_fire(
            1, 1_000, 1_050, 12_000, 3, 100, 10_000
        ));
    }

    #[test]
    fn should_coalesce_fire_pending_below_min_batch_inside_max_wait() {
        // 1 event at 1_000, now at 5_000 (<max_wait of 10s, <debounce from
        // last_event of 100ms with batch not met): neither predicate trips.
        assert!(!should_coalesce_fire(
            1, 1_000, 1_000, 5_000, 3, 100, 10_000
        ));
    }
}
