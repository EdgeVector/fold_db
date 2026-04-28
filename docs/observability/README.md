# Observability docs

Operator-facing documentation for the `observability` crate
(`crates/observability`). Designed for someone bringing up a new fold_db
node, debugging trace propagation, or tuning what reaches Honeycomb.

## Setup

- **[Honeycomb dev setup](honeycomb-setup.md)** — point a node at a
  Honeycomb environment, choose an `OBS_SAMPLER` setting, and project
  ingest cost per env. Start here.

## Implementation notes (one per phase / sweep)

These are working notes, not a tutorial. Each documents the decisions
behind a phase of the observability rollout so future contributors can
see *why* something was wired the way it is.

- [Sampling config](sampling-config-notes.md) — Phase 4 / T5. `OBS_SAMPLER`
  parser, default values, follow-ups.
- [tokio::spawn instrumentation](tokio-spawn-instrument-notes.md) — Phase 3 / T6.
  Where `.instrument(Span::current())` was applied to keep trace context
  across spawn boundaries.
- [LoggingSystem retirement](loggingsystem-retirement-notes.md) — Phase 3 / T7.
  Removal of the legacy `LoggingSystem` and the last `log` crate
  references.
- [Egress classification](egress-classification-notes.md) — Phase 2 / T4.
  `// trace-egress: <class>` comments at HTTP call sites and which calls
  do or don't get `inject_w3c` wrapping.
- [Redaction lint](redaction-lint.md) — Phase 5 / T1. CI guard that fails
  if a `tracing` macro emits a sensitive field as a raw value instead of
  through `redact!()` / `redact_id!()`.

## Source-of-truth pointers

- Crate sources: `crates/observability/src/`
- Init helpers: `crates/observability/src/init.rs`
  (`init_node` / `init_lambda` / `init_tauri` / `init_cli`)
- Layers: `crates/observability/src/layers/` (FMT, RELOAD, RING, WEB)
- Sampling: `crates/observability/src/sampling.rs`
- W3C propagation: `crates/observability/src/propagation.rs`
