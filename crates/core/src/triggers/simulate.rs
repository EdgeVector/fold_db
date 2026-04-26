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

/// Parse a `max_catch_up_age` duration string (e.g. `"30m"`, `"1h"`,
/// `"7d"`, `"2w"`) into a millisecond count. Accepted format is
/// `<positive-int><unit>` with unit in m/h/d/w. Returns `None` if the
/// string doesn't match — input validation is owned by schema_service's
/// `validate_max_catch_up_age` register-time hook; this consumer parses
/// safely (treating an unparseable or zero value as unbounded, via
/// `should_skip_catch_up`) rather than panicking at fire time.
pub fn parse_max_catch_up_age_ms(s: &str) -> Option<i64> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (num_part, unit) = trimmed
        .char_indices()
        .rfind(|(_, c)| c.is_ascii_digit())
        .map(|(last_digit_idx, _)| trimmed.split_at(last_digit_idx + 1))?;
    let n: i64 = num_part.parse().ok()?;
    if n <= 0 {
        return None;
    }
    let ms_per_unit: i64 = match unit {
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        "w" => 7 * 86_400_000,
        _ => return None,
    };
    n.checked_mul(ms_per_unit)
}

/// Staleness-budget predicate for `Scheduled` / `ScheduledIfDirty` catch-up
/// fires. Given the trigger's optional `max_catch_up_age` and the
/// wall-clock lag between `now` and the ideal cron fire time, decide
/// whether to skip this tick (and wait for the next cron occurrence).
///
/// * `None` budget → always returns `false` (unbounded catch-up; legacy
///   behavior).
/// * `Some(budget)` → returns `true` when `lag_ms > budget`. A
///   malformed budget string parses to `None` and returns `false` — it's a
///   warn condition at the caller, not a skip condition, so we err on the
///   side of firing.
/// * `lag_ms <= 0` (fire arrived on time or early) → always returns
///   `false` regardless of budget.
///
/// Contract: malformed budget strings ARE rejected upstream by
/// `schema_service::state::validate_max_catch_up_age` at `PUT /v1/views`
/// register time. The `Some(budget_str) → parse → None → false` branch
/// here is defense in depth for non-HTTP code paths (direct in-process
/// callers, pre-existing sled rows from older schema revisions, and
/// loopback tests that bypass the HTTP validator) — production fires
/// should never exercise it, and the schema_service validator's
/// accept-set is kept a strict subset of `parse_max_catch_up_age_ms`'s
/// `Some` set so a green registration is always a parseable budget here.
///
/// Purpose: bound fire storms after process downtime (laptop sleep, crash
/// restart) when the cron scheduler would otherwise back-fill every missed
/// tick between `last_fire_ms` and `now`.
pub fn should_skip_catch_up(max_catch_up_age: Option<&str>, lag_ms: i64) -> bool {
    let Some(budget_str) = max_catch_up_age else {
        return false;
    };
    let Some(budget_ms) = parse_max_catch_up_age_ms(budget_str) else {
        return false;
    };
    lag_ms > budget_ms
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

    #[test]
    fn parse_max_catch_up_age_ms_accepts_all_units() {
        assert_eq!(parse_max_catch_up_age_ms("30m"), Some(30 * 60_000));
        assert_eq!(parse_max_catch_up_age_ms("1h"), Some(3_600_000));
        assert_eq!(parse_max_catch_up_age_ms("24h"), Some(24 * 3_600_000));
        assert_eq!(parse_max_catch_up_age_ms("7d"), Some(7 * 86_400_000));
        assert_eq!(parse_max_catch_up_age_ms("2w"), Some(14 * 86_400_000));
    }

    #[test]
    fn parse_max_catch_up_age_ms_rejects_malformed() {
        // Empty / whitespace.
        assert!(parse_max_catch_up_age_ms("").is_none());
        assert!(parse_max_catch_up_age_ms("   ").is_none());
        // Missing unit.
        assert!(parse_max_catch_up_age_ms("24").is_none());
        // Unknown unit (seconds intentionally not supported — catch-up
        // budget is a coarse-grained knob).
        assert!(parse_max_catch_up_age_ms("5s").is_none());
        assert!(parse_max_catch_up_age_ms("3y").is_none());
        // Non-numeric prefix.
        assert!(parse_max_catch_up_age_ms("abc").is_none());
        assert!(parse_max_catch_up_age_ms("h").is_none());
        // Zero / negative are rejected to avoid a zero-budget degenerate.
        assert!(parse_max_catch_up_age_ms("0h").is_none());
        assert!(parse_max_catch_up_age_ms("-1h").is_none());
    }

    #[test]
    fn should_skip_catch_up_none_never_skips() {
        // Unbounded catch-up: never skip regardless of lag magnitude.
        assert!(!should_skip_catch_up(None, 0));
        assert!(!should_skip_catch_up(None, 1_000_000_000));
    }

    #[test]
    fn should_skip_catch_up_within_budget_does_not_skip() {
        // 24h budget, 30m lag → well under the budget; fire normally.
        assert!(!should_skip_catch_up(Some("24h"), 30 * 60_000));
        // 1h budget, exactly at the budget → boundary is inclusive of
        // firing (strict `>`), so lag == budget does not skip.
        assert!(!should_skip_catch_up(Some("1h"), 3_600_000));
    }

    #[test]
    fn should_skip_catch_up_over_budget_skips() {
        // 1h budget, 3h lag → skip this catch-up tick.
        assert!(should_skip_catch_up(Some("1h"), 3 * 3_600_000));
        // 1h budget, 1h + 1ms lag → just over; skip.
        assert!(should_skip_catch_up(Some("1h"), 3_600_001));
    }

    #[test]
    fn should_skip_catch_up_malformed_budget_falls_back_to_fire() {
        // Malformed budget → treat as unbounded (return false, don't skip).
        // Input validation lives in schema_service; this consumer must
        // never panic or silently drop a fire on garbage input.
        assert!(!should_skip_catch_up(Some("abc"), 10 * 3_600_000));
        assert!(!should_skip_catch_up(Some(""), 10 * 3_600_000));
        assert!(!should_skip_catch_up(Some("0h"), 10 * 3_600_000));
    }

    #[test]
    fn should_skip_catch_up_non_positive_lag_never_skips() {
        // Fire arrived early or on time: never a catch-up situation.
        assert!(!should_skip_catch_up(Some("1h"), 0));
        assert!(!should_skip_catch_up(Some("1h"), -5_000));
    }
}
