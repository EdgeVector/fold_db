# Structured fields lint (Phase 5 / T4)

Educational, **warn-only** CI lint that highlights `tracing` call sites
whose message string carries positional `{}` / `{:?}` / `{name}`
placeholders instead of emitting structured fields. The script always
exits 0; the CI job always passes. The point is to nudge new code
toward the structured form, not to block PRs.

## Why it matters

`tracing` events have two kinds of payload:

1. **The message string** — a flat, formatted line like
   `"upload to 'edgevector-prod' failed: connection reset"`.
2. **Structured fields** — typed key/value pairs attached to the event
   (`target = "edgevector-prod"`, `error = "connection reset"`).

In Honeycomb, Sentry, and our local query tooling, you can pivot, group,
and filter on **fields**. You cannot pivot on the contents of the message
string. So this:

```rust
tracing::warn!("upload to '{}' failed: {}", target.label, err);
```

produces a log line where `target.label` is unrecoverable as a field —
you can grep for it, but you cannot ask "show me the upload error rate
broken down by target". This:

```rust
tracing::warn!(
    target = %target.label,
    error = %err,
    "upload failed",
);
```

emits the same human-readable text plus two queryable fields. Same
ergonomics for the author, dramatically more useful in the dashboard.

## What the lint flags

The script greps `crates/*/src/` for the regex

```
tracing::(info|warn|debug|error|trace)!(...,"...{...}...", ...)
```

i.e. a `tracing` macro whose message string contains a `{...}`
placeholder and is followed by a comma (so at least one positional arg
trails it). Both the legacy `"...{}", x` form and the Rust-2021
`"...{x}", x = thing` form are flagged.

Pure-message calls without placeholders (`tracing::info!("started")`)
and already-structured calls (`tracing::info!(field = %x, "msg")`) are
ignored. Calls that mix structured fields *and* a positional message
arg (`tracing::info!(field = %x, "msg {}", y)`) **are** flagged — the
positional `y` is still unqueryable even though `field` is fine.

## Output and CI surface

Run locally:

```sh
sh scripts/lint-structured-fields.sh
```

The script exits 0 always. When there are hits, it prints a short
explainer, a before/after example, and the list of flagged sites with
file path, line number, and the source line. Sample:

```
lint-structured-fields: 39 site(s) use positional/inline message args
instead of structured fields. This is warn-only — no PR is blocked.

...

Sites flagged:
  crates/core/src/sync/engine.rs:732
      tracing::warn!("upload to '{}' failed (auth): {e}", target.label);
  ...
```

In CI (`.github/workflows/ci-tests.yml`), the
`Structured Fields Lint (warn-only)` job runs the same invocation and
republishes the output as a **job summary** on the PR's Checks tab.
Because the script exits 0 the check is always green; the summary is
where reviewers see the suggestions.

## Why warn-only (and not block)

Two reasons.

1. **There is a backlog.** At time of landing, the lint flags ~39
   pre-existing sites in `crates/core/src/`. Migrating them is a
   separate ergonomics PR; we did not want to gate that work on every
   downstream change.
2. **The structured form is a preference, not a correctness invariant.**
   Unlike redaction (which protects PII) or `tokio::spawn` instrumentation
   (which preserves trace stitching), a positional log arg is *worse for
   query ergonomics* but doesn't break anything. A nudge is the right
   shape of feedback.

If the structured-fields rate stops improving — or if the lint is being
ignored — we can revisit and promote it to blocking with an inline
override marker, mirroring `lint:redaction-ok` and
`lint:spawn-bare-ok`.

## Migrating a flagged site

Replace each `{}` / `{name}` in the message with a structured field on
the macro argument list, and shorten the message to the human-readable
event name:

```rust
// Before
tracing::info!("Loaded {} embeddings from store", entries.len());

// After
tracing::info!(count = entries.len(), "loaded embeddings from store");
```

For errors, prefer `error = %err` (the `%` is `tracing`'s `Display`
form) over `error = ?err` (Debug) unless you specifically want the
debug rendering:

```rust
// Before
tracing::warn!("Failed to embed face: {}", err);

// After
tracing::warn!(error = %err, "failed to embed face");
```

## Scope and limits

- `crates/*/src/` only, mirroring `lint-redaction.sh` and
  `lint-spawn-instrument.sh`. Top-level integration tests under
  `crates/*/tests/` are out of scope.
- `println!`, `eprintln!`, `log::*!`, and other non-`tracing` macros are
  not in scope. The lint is specifically about events that flow into
  Honeycomb / OTel.
- Sibling repos (`fold_db_node`, `schema_service`, `exemem-infra`) each
  own their own copy of this lint as a follow-up — same as the rest of
  the Phase 5 lint set.
