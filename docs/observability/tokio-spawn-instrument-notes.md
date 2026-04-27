# tokio::spawn `.instrument(Span::current())` audit (Phase 3 / T6)

Cross-Phase 3 notes on which `tokio::spawn(...)` sites in `crates/*/src/`
need `.instrument(tracing::Span::current())` so the spawned task inherits
its caller's span context (and thus `trace_id`, `span_id`, and any
`user.hash` / `schema.name` fields the parent span carries).

Phase 5 will add a CI lint that flags new bare `tokio::spawn(...)` calls —
this doc records why each existing site was classified the way it was so
the lint's allow-list (and reviewers) have a written precedent.

## Heuristic

- **YES (wrap with `.instrument(Span::current())`)**: the spawn lives in a
  function reachable from a request handler, ingestion path, or any caller
  that already has user/schema/request span context. Without `.instrument`,
  the spawned future starts with an empty span stack — the trace breaks at
  the spawn boundary.
- **NO (leave bare)**: the spawn is a perpetual worker started once at
  process boot (`init_*`, constructors), or it lives inside `#[cfg(test)]`
  scaffolding. There is no parent request span to propagate; tagging events
  with the boot-time span would be actively misleading.

`Span::current()` is always safe — when there is no enclosing span it
resolves to a disabled root, which the OTel layer simply skips. The choice
to leave a site bare is documentation, not correctness.

## fold_db (this repo) — Phase 3 / T6 sweep

Fresh `grep -RnE 'tokio::spawn\(' crates/*/src/` enumerated 16 sites. The
codex-flagged paths from the brief (`crates/core/src/ingestion.rs:223`,
`crates/core/src/handlers/auth.rs:276`) do not exist in the current
workspace layout — there is no `handlers/` module under `crates/core/src/`
and the only `ingestion.rs` is the LLM prompt template at
`crates/core/src/llm_registry/prompts/ingestion.rs`, which contains no
spawn. The grep below is authoritative for this PR.

### YES — wrapped with `.instrument(tracing::Span::current())`

- `crates/core/src/fold_db_core/trigger_runner.rs:544` (`dispatch_nonblocking`)
- `crates/core/src/fold_db_core/trigger_runner.rs:597` (`dispatch_inline_once`
  retry hand-off)

Both live inside `dispatch_*` async methods reachable from
`on_mutation_notified` (the per-mutation entry point). The retry path's own
comment — *"Failure: hand off retries to the background so the mutation
path isn't stuck"* — is the giveaway: the caller is the user's mutation
request, so its span carries `user.hash` / `schema.name` / `trace_id`. We
want the deferred fire's logs to join up with the originating mutation in
the trace.

`use tracing::Instrument;` was added to the file once at the top.

### NO — perpetual workers spawned at boot, no parent context

| Site | Why bare |
| --- | --- |
| `crates/core/src/storage/sled_pool.rs:139` | Idle reaper started once by `start_idle_reaper`; no per-request caller. |
| `crates/core/src/fold_db_core/event_monitor.rs:36, 50, 64, 78, 97` | Five subscriber loops created in `EventMonitor::new` at startup. Events flowing through them already carry their own per-emission context if any — the spawn itself does not. |
| `crates/core/src/fold_db_core/process_results_subscriber.rs:38` | `start_event_listener` spawns one subscriber loop at boot. Same reasoning as `event_monitor`. |
| `crates/core/src/fold_db_core/fold_db.rs:537` | `run_scheduler_loop` for `TriggerRunner`, started once at `FoldDB` construction. |
| `crates/core/src/fold_db_core/sync_coordinator.rs:88` | Background sync poll loop started once via `start_background_sync`. |
| `crates/core/src/fold_db_core/mutation_manager.rs:1003` | `start_event_listener` spawns one loop at boot to drain `MutationRequest` events. |

Tagging any of these with `.instrument(Span::current())` would stamp every
log line with whichever boot-time span happened to be current when
`new()` ran — not useful, and actively misleading on dashboards.

### NO — `#[cfg(test)]` scaffolding

| Site | Notes |
| --- | --- |
| `crates/core/src/triggers/clock.rs:192` | `mock_clock_advance_wakes_sleeper` test driver. |
| `crates/core/src/fold_db_core/trigger_runner.rs:1925` | Quarantine test driver. |
| `crates/core/src/fold_db_core/trigger_runner.rs:2014` | Coalesce-refire test driver. |
| `crates/core/src/fold_db_core/trigger_runner.rs:3051` | Backoff/quarantine restart test driver. |

Tests are out of scope for the lint — the Phase 5 enforcement should
exclude `cfg(test)` and `tests/` paths.

## Out of scope (covered elsewhere)

- `tokio::task::spawn_blocking(...)` calls in `crates/core/src/storage/sled_backend.rs`
  and other storage paths. `spawn_blocking` takes a synchronous closure,
  not a future, so `.instrument()` does not apply. If Phase 5 wants context
  propagation across blocking work it needs a separate helper (capture
  `Span::current()`, `let _e = span.enter()` inside the closure).
- Spawns inside `fold_db_node` or `schema_service` — these live in sibling
  workspaces and will be swept independently per the brief.

## Verification

A new unit test —
`crates/observability/src/layers/ring.rs::tests::instrument_propagates_trace_id_across_tokio_spawn`
— drives a `current_thread` runtime under a registry with an OTel layer +
RING layer, opens a parent span to seed a real W3C trace_id, then
`tokio::spawn(... .instrument(parent))` and asserts the spawned task's
event lands in the RING with the parent's trace_id (32 hex chars, exact
match). The test was confirmed to fail when the `.instrument(...)` call
is removed (the spawned event's metadata lacks `trace_id` because the
spawned task has no current span). This is the pattern test the Phase 5
lint should treat as the canonical "correct" shape for context-bearing
spawns.
