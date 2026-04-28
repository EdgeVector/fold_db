//! OTLP-compatible head sampling configuration parsed from the `OBS_SAMPLER`
//! env var.
//!
//! Phase 4 / T5. The OTel spec defines a small set of built-in samplers
//! (`always_on`, `always_off`, `traceidratio`, `parentbased_traceidratio`,
//! …). Honeycomb, the OTel Collector, and the OTel SDKs in other languages
//! all read the same `OTEL_TRACES_SAMPLER` env var with the same syntax;
//! we mirror that under our own `OBS_SAMPLER` name so operators can copy
//! tuning advice across stacks.
//!
//! ## Why parent-based by default
//!
//! `parentbased_traceidratio:1.0` (the default when `OBS_SAMPLER` is unset)
//! keeps 100% of traces in dev — useful when the operator is the only
//! consumer and every trace matters. The `parentbased_` prefix means a
//! distributed parent's sampling decision is honoured: if an upstream
//! service already decided "drop" we drop too, and "keep" we keep too.
//! This is the right default for any service that participates in a
//! W3C-propagated trace graph (which fold_db does — see
//! [`crate::propagation`]).
//!
//! ## Why error-keeping is independent
//!
//! Errors must always reach the SaaS sink so on-call can debug them.
//! Head sampling decides whether spans are *recorded* in the first place;
//! the Sentry / error-routing layer is wired separately and reads
//! `tracing::Event` levels regardless of the trace decision. Dropping a
//! span here does not drop the corresponding error event.

use std::env;

use opentelemetry_sdk::trace::Sampler;

/// Env var name parsed by [`parse_sampler`]. Mirrors OTel's
/// `OTEL_TRACES_SAMPLER` syntax under a fold_db-scoped name.
pub const OBS_SAMPLER_ENV: &str = "OBS_SAMPLER";

/// Default sampler used when `OBS_SAMPLER` is unset. Dev-safe: keeps every
/// trace, but honours an upstream "drop" decision so this service stays
/// consistent inside a propagated trace graph.
pub const DEFAULT_SAMPLER_SPEC: &str = "parentbased_traceidratio:1.0";

/// Failure modes for [`parse_sampler_spec`]. Returned as a structured error
/// so callers can distinguish "your env var is malformed" from "we couldn't
/// reach Honeycomb" at startup time.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SamplerParseError {
    #[error(
        "OBS_SAMPLER ratio for `{kind}` is not a valid f64 between 0.0 and 1.0: got {value:?}"
    )]
    InvalidRatio { kind: &'static str, value: String },
    #[error("OBS_SAMPLER `{kind}` requires a `:<ratio>` suffix (e.g. `{kind}:0.1`); got {raw:?}")]
    MissingRatio { kind: &'static str, raw: String },
    #[error(
        "OBS_SAMPLER value {raw:?} is not a recognized sampler. Expected one of: \
         always_on, always_off, traceidratio:<f>, parentbased_traceidratio:<f>"
    )]
    UnknownSampler { raw: String },
}

/// Resolve the active sampler from the process environment.
///
/// Order of resolution:
/// 1. `$OBS_SAMPLER` if set — parsed via [`parse_sampler_spec`].
/// 2. [`DEFAULT_SAMPLER_SPEC`] — `parentbased_traceidratio:1.0` (100%,
///    parent-honouring).
///
/// A malformed env var is a *programmer-visible* error: we surface it via
/// the `Result` rather than silently falling back to the default, because
/// silent fallback would mask a typo that drops the operator's intended
/// sampling rate.
pub fn parse_sampler() -> Result<Sampler, SamplerParseError> {
    match env::var(OBS_SAMPLER_ENV) {
        Ok(raw) if !raw.trim().is_empty() => parse_sampler_spec(raw.trim()),
        _ => parse_sampler_spec(DEFAULT_SAMPLER_SPEC),
    }
}

/// Parse one OTel-spec sampler string. Public so tests can drive it
/// directly without mutating the real environment.
pub fn parse_sampler_spec(raw: &str) -> Result<Sampler, SamplerParseError> {
    // Split on the first `:` so values that include further punctuation in
    // the future (e.g. parentbased delegates) parse without rewrites.
    let (kind, arg) = match raw.split_once(':') {
        Some((k, v)) => (k.trim(), Some(v.trim())),
        None => (raw.trim(), None),
    };

    match (kind, arg) {
        ("always_on", None) => Ok(Sampler::AlwaysOn),
        ("always_off", None) => Ok(Sampler::AlwaysOff),
        ("traceidratio", Some(arg)) => Ok(Sampler::TraceIdRatioBased(parse_ratio(
            "traceidratio",
            arg,
        )?)),
        ("parentbased_traceidratio", Some(arg)) => {
            let ratio = parse_ratio("parentbased_traceidratio", arg)?;
            Ok(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                ratio,
            ))))
        }
        ("traceidratio", None) => Err(SamplerParseError::MissingRatio {
            kind: "traceidratio",
            raw: raw.to_string(),
        }),
        ("parentbased_traceidratio", None) => Err(SamplerParseError::MissingRatio {
            kind: "parentbased_traceidratio",
            raw: raw.to_string(),
        }),
        // `always_on:0.5` and friends — a ratio was supplied where none was
        // expected. Treat as unknown rather than ignoring; the operator
        // probably meant `traceidratio:0.5`.
        _ => Err(SamplerParseError::UnknownSampler {
            raw: raw.to_string(),
        }),
    }
}

fn parse_ratio(kind: &'static str, value: &str) -> Result<f64, SamplerParseError> {
    let parsed: f64 = value.parse().map_err(|_| SamplerParseError::InvalidRatio {
        kind,
        value: value.to_string(),
    })?;
    // Reject NaN, negative, or >1 — the OTel SDK clamps these silently to
    // [0,1], but a clamp masks operator typos. Surface the bad value so the
    // boot log shows it.
    if !parsed.is_finite() || !(0.0..=1.0).contains(&parsed) {
        return Err(SamplerParseError::InvalidRatio {
            kind,
            value: value.to_string(),
        });
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Probe a `Sampler` value by asking it whether a given trace id should
    /// be sampled. We don't assert variant identity directly because
    /// `Sampler::ParentBased` wraps a `Box<dyn ShouldSample>` and
    /// `TraceIdRatioBased` carries an `f64` — pattern matching is fragile
    /// across SDK upgrades. Behavioural probes are stable.
    fn record_decision(sampler: &Sampler, trace_id: opentelemetry::trace::TraceId) -> bool {
        use opentelemetry::trace::{SamplingDecision, SpanKind};
        use opentelemetry_sdk::trace::ShouldSample;
        let result = sampler.should_sample(None, trace_id, "probe", &SpanKind::Internal, &[], &[]);
        matches!(
            result.decision,
            SamplingDecision::RecordAndSample | SamplingDecision::RecordOnly
        )
    }

    /// `TraceId::from_u128` was removed from `opentelemetry` somewhere along
    /// the way. The byte-array constructor is the stable path.
    fn trace_id_of(seed: u128) -> opentelemetry::trace::TraceId {
        opentelemetry::trace::TraceId::from_bytes(seed.to_be_bytes())
    }

    #[test]
    fn parses_always_on() {
        let s = parse_sampler_spec("always_on").expect("always_on parses");
        assert!(record_decision(&s, trace_id_of(1)));
    }

    #[test]
    fn parses_always_off() {
        let s = parse_sampler_spec("always_off").expect("always_off parses");
        assert!(!record_decision(&s, trace_id_of(1)));
    }

    #[test]
    fn parses_traceidratio_zero() {
        let s = parse_sampler_spec("traceidratio:0.0").expect("ratio:0 parses");
        // ratio 0 must drop everything
        assert!(!record_decision(&s, trace_id_of(1)));
    }

    #[test]
    fn parses_traceidratio_one() {
        let s = parse_sampler_spec("traceidratio:1.0").expect("ratio:1 parses");
        // ratio 1 must keep everything
        assert!(record_decision(&s, trace_id_of(0xdead_beef),));
    }

    #[test]
    fn parses_parentbased_traceidratio_one() {
        // No parent context → falls back to root sampler (ratio 1) → keep.
        let s =
            parse_sampler_spec("parentbased_traceidratio:1.0").expect("parentbased ratio:1 parses");
        assert!(record_decision(&s, trace_id_of(7)));
    }

    #[test]
    fn parses_parentbased_traceidratio_zero() {
        // No parent context → falls back to root sampler (ratio 0) → drop.
        let s =
            parse_sampler_spec("parentbased_traceidratio:0.0").expect("parentbased ratio:0 parses");
        assert!(!record_decision(&s, trace_id_of(7)));
    }

    #[test]
    fn tolerates_surrounding_whitespace() {
        // Useful when the env var is set via a shell heredoc that leaks
        // trailing whitespace.
        let s = parse_sampler_spec("  always_on  ").expect("trim works");
        assert!(record_decision(&s, trace_id_of(1)));
    }

    #[test]
    fn rejects_malformed_ratio() {
        let err = parse_sampler_spec("traceidratio:abc").expect_err("must error");
        assert!(
            matches!(
                err,
                SamplerParseError::InvalidRatio {
                    kind: "traceidratio",
                    ..
                }
            ),
            "got: {err:?}"
        );
        // The error message must include the bad value so the boot log is
        // self-explanatory.
        assert!(err.to_string().contains("abc"), "msg={}", err);
    }

    #[test]
    fn rejects_out_of_range_ratio() {
        let err = parse_sampler_spec("traceidratio:1.5").expect_err("clamp must surface");
        assert!(matches!(err, SamplerParseError::InvalidRatio { .. }));
    }

    #[test]
    fn rejects_missing_ratio() {
        let err = parse_sampler_spec("traceidratio").expect_err("missing ratio");
        assert!(matches!(
            err,
            SamplerParseError::MissingRatio {
                kind: "traceidratio",
                ..
            }
        ));
        let err = parse_sampler_spec("parentbased_traceidratio").expect_err("missing ratio");
        assert!(matches!(
            err,
            SamplerParseError::MissingRatio {
                kind: "parentbased_traceidratio",
                ..
            }
        ));
    }

    #[test]
    fn rejects_unknown_sampler() {
        let err = parse_sampler_spec("jaeger_remote").expect_err("unknown");
        assert!(matches!(err, SamplerParseError::UnknownSampler { .. }));
    }

    #[test]
    fn rejects_arg_on_argless_sampler() {
        // `always_on:0.5` is a typo — the user probably meant
        // `traceidratio:0.5`. We must not silently accept the arg.
        let err = parse_sampler_spec("always_on:0.5").expect_err("argless+arg → error");
        assert!(matches!(err, SamplerParseError::UnknownSampler { .. }));
    }

    /// Mutating env vars from tests is racy when other tests in the same
    /// binary read the same var concurrently. We serialize this single
    /// env-touching test behind a mutex local to the module.
    #[test]
    fn parse_sampler_falls_back_to_default_when_env_absent() {
        use std::sync::Mutex;
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _g = ENV_LOCK.lock().unwrap();

        let prev = env::var(OBS_SAMPLER_ENV).ok();
        env::remove_var(OBS_SAMPLER_ENV);

        let sampler = parse_sampler().expect("default parses");

        // restore
        if let Some(v) = prev {
            env::set_var(OBS_SAMPLER_ENV, v);
        }

        // Default is parentbased_traceidratio:1.0 → with no parent, root
        // sampler (ratio 1) → keep.
        assert!(record_decision(&sampler, trace_id_of(42),));
    }

    #[test]
    fn parse_sampler_reads_env_when_set() {
        use std::sync::Mutex;
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _g = ENV_LOCK.lock().unwrap();

        let prev = env::var(OBS_SAMPLER_ENV).ok();
        env::set_var(OBS_SAMPLER_ENV, "always_off");

        let sampler = parse_sampler().expect("env parses");

        match prev {
            Some(v) => env::set_var(OBS_SAMPLER_ENV, v),
            None => env::remove_var(OBS_SAMPLER_ENV),
        }

        assert!(!record_decision(&sampler, trace_id_of(99),));
    }

    #[test]
    fn parse_sampler_treats_blank_env_as_unset() {
        use std::sync::Mutex;
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _g = ENV_LOCK.lock().unwrap();

        let prev = env::var(OBS_SAMPLER_ENV).ok();
        env::set_var(OBS_SAMPLER_ENV, "   ");

        let sampler = parse_sampler().expect("blank → default, not error");

        match prev {
            Some(v) => env::set_var(OBS_SAMPLER_ENV, v),
            None => env::remove_var(OBS_SAMPLER_ENV),
        }

        assert!(record_decision(&sampler, trace_id_of(1),));
    }
}
