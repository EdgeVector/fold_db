# `tokio::spawn` instrument lint (Phase 5 / T2)

Static guard that fails CI if a `tokio::spawn(async ... { ... })` site in
`crates/*/src/` does not chain `.instrument(...)` / `.in_current_span()`
on the spawned future, and is not explicitly marked as an intentional
bare spawn.

This pairs with the runtime test in
`crates/observability/src/layers/ring.rs::instrument_propagates_trace_id_across_tokio_spawn`,
which proves that *with* `.instrument(Span::current())` the spawned task's
events join the parent's trace. The lint enforces the call-site
discipline so the runtime invariant doesn't silently regress.

## Why it matters

`tokio::spawn` runs the future on a worker thread that has no thread-local
default subscriber state — the spawned task starts with an **empty span
stack**. Concretely, this means:

- The parent's `trace_id` does not propagate. Logs emitted from the
  spawned task get no `trace_id` (or, worse, a fresh one), so they cannot
  be stitched back to the originating request in Honeycomb / Sentry.
- Span fields the parent carried (`user.hash`, `schema.name`,
  `request.id`, etc.) vanish.
- Sampling decisions made at the parent are ignored — a child task may
  emit on a request the operator chose not to sample.

`fut.instrument(tracing::Span::current())` (or the `.in_current_span()`
shorthand) attaches the caller's current span to the future *as data*,
so when the worker polls the future it re-enters that span before
running the body. Trace stitching survives the spawn boundary.

`Span::current()` is always safe: when there is no enclosing span it
resolves to a disabled root that the OTel layer simply skips. The
choice to leave a site bare is documentation, not correctness.

## The pattern the lint enforces

The lint walks every `tokio::spawn(` occurrence in `crates/*/src/`. For
each one whose body is an `async` block (single-line or multi-line, with
or without `move`), it scans the call's full parenthesised body —
balancing parentheses character-by-character through the awk scanner so
strings, char literals, line comments, and block comments don't poison
the depth count — and looks for one of:

- `.instrument(`           — the canonical form, usually
  `.instrument(tracing::Span::current())`.
- `.in_current_span()`     — the no-arg shorthand from `tracing` for the
  same thing.
- `// lint:spawn-bare-ok <reason>` — the explicit override marker. May
  live on the spawn line itself, the line immediately preceding it, or
  anywhere inside the spawn call's body.

If none of those is present anywhere in the spawn's parenthesised body
(or in the line immediately preceding it, for the override case), the
lint fails the build and points at the spawn line.

### Canonical good shapes

Single-line:

```rust
tokio::spawn(async move { do_work().await }.in_current_span());
```

Multi-line — closing brace then `.instrument(...)` before the matching
`)`, the form rustfmt prefers when the body is more than a few lines:

```rust
tokio::spawn(
    async move {
        runner
            .run_fire_with_refire_loop(&view_name, trigger_index, &rt)
            .await;
        rt.dispatch_in_flight.store(false, Ordering::SeqCst);
    }
    .instrument(tracing::Span::current()),
);
```

Both forms pass the lint because `.instrument(...)` lives inside the
`tokio::spawn(...)` call's parens.

`use tracing::Instrument;` (or `use tracing::instrument::Instrument;`)
must be in scope at the file level for `.instrument(...)` to resolve.

## The override marker

Use `// lint:spawn-bare-ok <reason>` for spawns that genuinely have no
parent context to propagate. Two categories qualify, both pre-classified
in `docs/observability/tokio-spawn-instrument-notes.md`:

1. **Boot-time perpetual workers.** Spawned once at process startup
   from `init_*` / constructors. The current span at boot is either the
   binary's startup span or no span at all — tagging every event with it
   would be misleading on dashboards. Per-event spans are created by
   downstream code as work flows through. Examples:
   - `SledPool::start_idle_reaper` — the idle reaper.
   - `EventMonitor::new` — five subscriber loops.
   - `MutationManager::start_event_listener`.
   - `ProcessResultsSubscriber::start_event_listener`.
   - `FoldDB::new` — the `TriggerRunner` scheduler loop.
   - `SyncCoordinator::start_background_sync`.

2. **`#[cfg(test)]` scaffolding.** Test driver tasks that advance a
   `MockClock` or block on a gate so the test can exercise concurrent
   behaviour. There is no parent request span; tagging would be
   misleading.

Always include a short reason after the marker:

```rust
// lint:spawn-bare-ok boot-time idle reaper — perpetual worker, no per-request parent span.
tokio::spawn(async move { /* ... */ });
```

The marker may live on the spawn line, the preceding line, or anywhere
inside the spawn call's body. The two-line window survives `rustfmt`
lifting a long trailing comment onto its own line.

If you find yourself reaching for the override in code that runs on the
request path (anything reachable from a handler, ingestion pipeline, or
mutation), step back: the right fix is almost always
`.instrument(tracing::Span::current())`. The override is for code that
runs *outside* any request context, not for code that runs inside one
and finds propagation inconvenient.

## Running locally

```sh
bash scripts/lint-spawn-instrument.sh
```

Exit code is `0` when every spawn site is instrumented or marked, `1`
otherwise. The CI step `Spawn Instrument Lint` inside the `Rust Tests`
job (`.github/workflows/ci-tests.yml`) runs the same invocation on
every PR and `push` to `main`.

## Scope and limits

- `crates/*/src/` only. Top-level integration tests under
  `crates/*/tests/` are out of scope, mirroring `lint-tracing-egress.sh`.
- `tokio::spawn(some_future)` (where the argument is a *named future*,
  not an `async` block literal) is intentionally not flagged — the
  caller of that helper is expected to have wrapped the future already.
  If a future-returning helper grows callers that bypass instrumentation,
  prefer fixing the helper signature over widening the lint.
- `tokio::task::spawn_blocking(...)` takes a synchronous closure, so
  `.instrument()` does not apply; that case needs a separate helper
  (capture `Span::current()`, `let _e = span.enter()` inside the
  closure) and is deferred.
- `Span::current().context()` propagation across non-`spawn` task
  boundaries (e.g. mpsc message hand-offs that re-enter on the receiver)
  is deferred to a follow-up task.
- Sibling repos (`fold_db_node`, `schema_service`, `exemem-infra`) each
  ship their own copy of the same lint as a follow-up.
