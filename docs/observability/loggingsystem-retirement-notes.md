# LoggingSystem retirement notes (Phase 3 / T7)

Final atomic switchover that deletes `crates/core/src/logging/` and removes
the `log` crate as a direct dependency of `fold_db`. After T1–T6 migrated
call sites and rewired the dashboard endpoints, this PR is the cleanup.

## What landed in this PR

- `crates/core/src/logging/` (config.rs, core.rs, features.rs, mod.rs,
  outputs/) deleted in full.
- `pub mod logging;` removed from `crates/core/src/lib.rs`.
- `log = { version = "0.4" }` removed from `crates/core/Cargo.toml`'s
  `[dependencies]`.
- New module `crates/core/src/user_context.rs` with `run_with_user` and
  `get_current_user_id`. The tokio task-local moved out of `logging::core`
  because user context is a request property, not a logging detail; carrying
  it under `logging::` was incidental to the legacy bridge.
- `tracing_log::LogTracer::init()` install moved BEFORE the
  `set_global_default(...)` call in `init_node` / `init_lambda` /
  `init_tauri` / `init_cli` (was previously called immediately after, in
  `install_globals`). Same effective behavior in steady state, but it now
  also bridges `log::*` calls emitted between subscriber install and the
  current line — including any `log::*!` from third-party crates that fire
  during init itself.
- New unit test `init::tests::log_macros_route_through_tracing_via_log_tracer`
  pins the bridge in place: emits via `log::info!` / `log::warn!` and asserts
  the events reach a tracing `Layer`.

## Things that surprised me

### `LogTracer` rewrites the `target`

Bridged `log::Record`s arrive at the tracing subscriber with `target = "log"`,
not the original `log::*!(target: "...")` value. The original target lives in
a field on the event. The first cut of the test asserted on
`event.metadata().target()` and failed even though the bridge was working —
the captured events were there, just with a different target. Adjusted the
test to filter by message body and added a comment so the next person
chasing this gets it for free.

If we ever want to preserve original `target` values across the bridge,
`LogTracer::builder().with_filter(...)` plus a custom `subscriber::interest`
shim would be needed — out of scope here, leaving as future work.

### `MutationManager` and friends were the only `run_with_user` callers

I expected the task-local user-context propagation to be load-bearing across
many crates. Final inventory:

- `fold_db_core/mutation_manager.rs` — wraps the event-listener task.
- `fold_db_core/process_results_subscriber.rs` — wraps the consumer loop.
- `fold_db_core/orchestration/index_status.rs` — reads via
  `get_current_user_id`.
- `fold_db_core/query/hash_range_query.rs` — reads for tracing context.
- `messaging/constructors.rs` — reads for `MutationExecuted.user_id`.
- `progress.rs` — reads for sled key namespacing.

That's it. Five readers and two writers. Putting it in its own module is
not over-engineering — it's exactly the right size.

### External consumers in `fold_db_node` are pinned by rev

`fold_db_node`'s `Cargo.toml` pins
`fold_db = { git = "...", rev = "a0434b2539..." }`, NOT HEAD. So this PR
does not break `fold_db_node`'s build. The consumers there
(`fold_db::logging::core::run_with_user`, `LoggingSystem::query_logs`,
`LogFeature::*`, etc.) keep resolving against the OLD rev until somebody
bumps it. That bump will need a follow-up migration in `fold_db_node`
itself — call sites translate one-for-one:

- `fold_db::logging::core::run_with_user` → `fold_db::user_context::run_with_user`
- `fold_db::logging::core::get_current_user_id` → `fold_db::user_context::get_current_user_id`
- `fold_db::logging::features::LogFeature::X / log_feature!(X, level, ...)` →
  `tracing::level!(target: "fold_node::x", ...)`
- `fold_db::logging::LoggingSystem::query_logs(...)` →
  `ObsGuard::ring().unwrap().query(...)` (ring buffer powering `/api/logs`)
- `fold_db::logging::LoggingSystem::get_config / get_features / update_feature_level / reload_config_from_file`
  → `ObsGuard::reload()` for runtime filter updates; static config (per-feature
  levels) is now plain `RUST_LOG=fold_node::schema=debug,...` env-filter
  syntax — there is no on-disk `LogConfig` to swap anymore.

### No transitive `log` dep leaked

`cargo check --workspace` after the `log` removal compiled cleanly. None of
`fold_db`'s remaining direct deps re-export `log` macros into
`fold_db`'s own source. `LogTracer` (in `observability`) is the bridge for
transitive `log::*` calls coming from `reqwest` / `hyper` / `sled` / etc.

## Out of scope (filed as follow-ups)

- CI lint enforcing `tracing::*` over `log::*` (Phase 5).
- Trace context propagation into AWS SDK egress — intentionally out of
  scope. Those calls are auth/billing/sync metadata, not on the user-facing
  critical path; a lightweight `#[tracing::instrument]` on Lambda wrapper
  functions is the fallback if span coverage is needed.
- `fold_db_node` consumer migration (separate PR in `fold_db_node` repo).
