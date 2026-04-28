# Redaction lint (Phase 5 / T1)

Static guard that fails CI if a `tracing` macro emits a sensitive field as a
raw value instead of routing it through the redaction macros.

This is the second of two layers of defence:

1. **Format-time deny-list** in the `RedactingFormat` JSON formatter
   (`crates/observability/src/layers/fmt.rs`) — replaces values for the
   denied keys with `<redacted>` even if a call site forgets.
2. **CI lint** (this doc) — catches the mistake at PR time so the deny-list
   never has to fire in production.

## Guarded fields

The lint pattern matches the literal field name on the left of `=` inside a
`tracing::{info,warn,debug,error,trace}!(...)` invocation:

```
password
token
api_key
secret
auth_token
email
phone
ssn
```

These mirror the static deny-list inside `RedactingFormat`. To extend the
list, edit the `PATTERN` regex in `scripts/lint-redaction.sh` **and** the
deny-list in `crates/observability/src/layers/fmt.rs` together so the two
layers stay in sync.

## What "redacted" means

A value is considered redacted if the right-hand side of the `=` (up to the
next `,`) goes through one of:

- `observability::redact!(...)` — opaque, returns the literal `<redacted>`.
  Use for values you never want back: passwords, raw API keys, encrypted
  blob contents.
- `observability::redact_id!(...)` — correlatable, returns
  `<id:HHHHHHHH>` (low 32 bits of xxhash64, lowercase hex). Use for
  identifiers you need to follow across log lines without exposing the
  underlying string.

Either is recognised whether prefixed with `%` (the `tracing` `Display` form)
or used bare. Example:

```rust
tracing::info!(
    api_key = %observability::redact!(&api_key),
    user.hash = %observability::redact_id!(&user_hash),
    "request received",
);
```

## Override

For an intentional exception — for example, a unit test that has to feed a
raw value to the FMT layer to verify the deny-list redacts it — add a
comment containing the literal `lint:redaction-ok <reason>` either on the
violating line itself or on the line directly above it:

```rust
// lint:redaction-ok FMT-layer test must emit raw value to verify deny-list redaction
tracing::info!(password = "hunter2", "login");
```

The two-line window exists so the override survives `rustfmt`, which will
lift a long trailing `//` comment onto its own line. The script honours
any comment style (`//`, `/* */`, `#`); the only thing it grep's for is the
marker substring. Always include a short reason after the marker so the
next reviewer can tell at a glance whether the override is still
load-bearing.

Use the override sparingly. A single override per intentional case is
fine; if you find yourself overriding in production code, it almost
certainly should have been wrapped in `redact!()` or `redact_id!()`
instead.

## Running locally

```sh
sh scripts/lint-redaction.sh
```

The script walks `crates/*/src/` only — generated tests under
`crates/*/tests/` are out of scope. Exit code is `0` if every match is
wrapped or overridden, `1` otherwise. The CI job
`Redaction Lint` in `.github/workflows/ci-tests.yml` runs the same
invocation on every PR and `push` to `main`.

## Out of scope

- `crates/*/tests/` integration tests. Top-level integration tests run
  against the public API; they should not be emitting raw secrets either,
  but the lint scope deliberately mirrors `lint-tracing-egress.sh` for
  consistency. Revisit if a violation slips through.
- Sibling repos (`fold_db_node`, `schema_service`, `exemem-infra`). Each
  ships its own copy of the same lint as a follow-up task.
