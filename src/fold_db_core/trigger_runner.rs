//! TriggerRunner — authoritative fire path for views.
//!
//! Phase 1 task 3 rip-out: before this runner, every successful mutation
//! on schema S reran every view transitively dependent on S. That implicit
//! cascade is gone (see `mutation_manager::write_mutations_batch_async`).
//! Views now declare `Trigger`s explicitly, and this runner decides which
//! fires to dispatch for each mutation.
//!
//! ### Dispatch flow
//! ```text
//! mutation on S
//!    │
//!    ▼
//! TriggerRunner::on_mutation(S, fields_affected)
//!    │
//!    │ walk view registry; for each view V sourced from S:
//!    │   OnWrite              → dispatch via per-view mutex (refire flag)
//!    │   OnWriteCoalesced     → bump counters, fire if thresholds crossed
//!    │   ScheduledIfDirty     → set dirty flag, scheduler tick will fire
//!    │   Scheduled / Manual   → mutation is a no-op
//!    ▼
//! FireHandler::fire(view_name)
//!    │
//!    ▼
//! TriggerFiring row written (at-least-once: last_fire_ms does NOT advance
//!                             when the row write fails — next tick retries)
//! ```
//!
//! State lives in two places:
//! - In-memory `HashMap<ViewId, Arc<ViewRuntime>>`: per-view mutex and
//!   refire flag. One-fire-at-a-time is enforced via `Mutex::try_lock` —
//!   if locked, the caller flips `refire_requested` and returns; the
//!   in-flight fire checks that flag on completion and re-dispatches.
//! - Sled tree `"trigger_state"`: persistent fields (pending_count,
//!   first_event_ms, last_event_ms, dirty, fail_streak, last_fire_ms,
//!   quarantined). Re-hydrated at startup so a node restart in the
//!   middle of a coalesce window doesn't drop events.

use async_trait::async_trait;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

use super::view_orchestrator::ViewOrchestrator;
use crate::schema::types::{KeyValue, Mutation};
use crate::schema::{SchemaCore, SchemaError};
use crate::storage::SledPool;
use crate::triggers::clock::Clock;
use crate::triggers::types::Trigger;
use crate::triggers::{fields, status, TRIGGER_FIRING_SCHEMA_NAME};

const TRIGGER_STATE_TREE: &str = "trigger_state";
const BACKOFF_MIN_MS: u64 = 1_000;
const BACKOFF_MAX_MS: u64 = 60_000;
const QUARANTINE_FAIL_STREAK: u32 = 3;

/// Receives notifications for every successful mutation and schedules
/// fires for views whose triggers match. Wired in from `MutationManager`
/// as `Arc<dyn TriggerDispatcher>`.
#[async_trait]
pub trait TriggerDispatcher: Send + Sync {
    async fn on_mutation(
        &self,
        schema_name: &str,
        fields_affected: &[String],
    ) -> Result<(), SchemaError>;
}

/// Executes the actual work of firing one view. Production wires this to
/// `ViewOrchestrator` (cache invalidation + precompute). Tests inject a
/// mock that returns a scripted outcome so backoff and quarantine can be
/// exercised deterministically.
#[async_trait]
pub trait FireHandler: Send + Sync {
    async fn fire(&self, view_name: &str) -> FireOutcome;
}

pub struct FireOutcome {
    pub success: bool,
    pub input_row_count: i64,
    pub output_row_count: i64,
    pub error_message: Option<String>,
}

impl FireOutcome {
    pub fn success(input_rows: i64, output_rows: i64) -> Self {
        Self {
            success: true,
            input_row_count: input_rows,
            output_row_count: output_rows,
            error_message: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            input_row_count: 0,
            output_row_count: 0,
            error_message: Some(msg.into()),
        }
    }
}

/// Produces `Mutation`s for TriggerFiring rows and hands them off to a
/// writer. A trait so the runner isn't coupled to `MutationManager`, and
/// so tests can assert on the exact rows produced without booting a DB.
#[async_trait]
pub trait FiringWriter: Send + Sync {
    async fn write_firing(&self, row: FiringRecord) -> Result<(), SchemaError>;
}

#[derive(Debug, Clone)]
pub struct FiringRecord {
    pub trigger_id: String,
    pub view_name: String,
    pub fired_at_ms: i64,
    pub duration_ms: i64,
    pub status: FiringStatus,
    pub input_row_count: i64,
    pub output_row_count: i64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiringStatus {
    Success,
    Error,
    Quarantined,
}

impl FiringStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FiringStatus::Success => status::SUCCESS,
            FiringStatus::Error => status::ERROR,
            FiringStatus::Quarantined => status::QUARANTINED,
        }
    }
}

impl FiringRecord {
    /// Build the `Mutation` that writes this record into the TriggerFiring
    /// schema. Exposed as a free function so `FiringWriter` impls that
    /// want to go through `MutationManager` can share one construction.
    pub fn into_mutation(self, pub_key: &str) -> Mutation {
        let mut fields_map = HashMap::new();
        fields_map.insert(
            fields::TRIGGER_ID.to_string(),
            serde_json::Value::String(self.trigger_id.clone()),
        );
        fields_map.insert(
            fields::VIEW_NAME.to_string(),
            serde_json::Value::String(self.view_name),
        );
        fields_map.insert(
            fields::FIRED_AT.to_string(),
            serde_json::Value::Number(self.fired_at_ms.into()),
        );
        fields_map.insert(
            fields::DURATION_MS.to_string(),
            serde_json::Value::Number(self.duration_ms.into()),
        );
        fields_map.insert(
            fields::STATUS.to_string(),
            serde_json::Value::String(self.status.as_str().to_string()),
        );
        fields_map.insert(
            fields::INPUT_ROW_COUNT.to_string(),
            serde_json::Value::Number(self.input_row_count.into()),
        );
        fields_map.insert(
            fields::OUTPUT_ROW_COUNT.to_string(),
            serde_json::Value::Number(self.output_row_count.into()),
        );
        fields_map.insert(
            fields::ERROR_MESSAGE.to_string(),
            match self.error_message {
                Some(m) => serde_json::Value::String(m),
                None => serde_json::Value::Null,
            },
        );

        Mutation::new(
            TRIGGER_FIRING_SCHEMA_NAME.to_string(),
            fields_map,
            KeyValue::new(Some(self.trigger_id), Some(self.fired_at_ms.to_string())),
            pub_key.to_string(),
            crate::schema::types::operations::MutationType::Create,
        )
    }
}

/// Persistent per-view counters. Stored under key `view_name` in the
/// `"trigger_state"` sled tree so a restart doesn't lose a half-built
/// coalesce batch or a quarantine decision.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PersistedViewState {
    pending_count: u32,
    first_event_ms: i64,
    last_event_ms: i64,
    dirty: bool,
    fail_streak: u32,
    last_fire_ms: i64,
    quarantined: bool,
}

/// In-memory runtime for one view. An atomic `dispatch_in_flight` flag
/// serialises fires without the races a plain `Mutex::try_lock` would
/// allow (between the dispatch-time probe and the spawned task's lock
/// acquisition, another dispatcher could slip through). The refire flag
/// is picked up by the in-flight task right before it returns.
struct ViewRuntime {
    persisted: Mutex<PersistedViewState>,
    dispatch_in_flight: std::sync::atomic::AtomicBool,
    refire_requested: std::sync::atomic::AtomicBool,
}

impl ViewRuntime {
    fn new(persisted: PersistedViewState) -> Self {
        Self {
            persisted: Mutex::new(persisted),
            dispatch_in_flight: std::sync::atomic::AtomicBool::new(false),
            refire_requested: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ScheduledFire {
    fire_at_ms: i64,
    view_name: String,
    trigger_index: usize,
}

impl Ord for ScheduledFire {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.fire_at_ms
            .cmp(&other.fire_at_ms)
            .then_with(|| self.view_name.cmp(&other.view_name))
            .then_with(|| self.trigger_index.cmp(&other.trigger_index))
    }
}

impl PartialOrd for ScheduledFire {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct TriggerRunner<C: Clock> {
    schema_manager: Arc<SchemaCore>,
    sled_pool: Option<Arc<SledPool>>,
    clock: Arc<C>,
    fire_handler: Arc<dyn FireHandler>,
    firing_writer: Arc<dyn FiringWriter>,

    runtimes: Mutex<HashMap<String, Arc<ViewRuntime>>>,
    scheduler: Mutex<BinaryHeap<Reverse<ScheduledFire>>>,
    scheduler_notify: Arc<Notify>,
}

impl<C: Clock> TriggerRunner<C> {
    pub fn new(
        schema_manager: Arc<SchemaCore>,
        sled_pool: Option<Arc<SledPool>>,
        clock: Arc<C>,
        fire_handler: Arc<dyn FireHandler>,
        firing_writer: Arc<dyn FiringWriter>,
    ) -> Self {
        Self {
            schema_manager,
            sled_pool,
            clock,
            fire_handler,
            firing_writer,
            runtimes: Mutex::new(HashMap::new()),
            scheduler: Mutex::new(BinaryHeap::new()),
            scheduler_notify: Arc::new(Notify::new()),
        }
    }

    /// Build a runner wired to a `ViewOrchestrator` and the internal
    /// `TriggerFiring` writer (production path).
    pub fn new_with_orchestrator(
        schema_manager: Arc<SchemaCore>,
        view_orchestrator: Arc<ViewOrchestrator>,
        sled_pool: Option<Arc<SledPool>>,
        clock: Arc<C>,
        firing_writer: Arc<dyn FiringWriter>,
    ) -> Self {
        let fire_handler: Arc<dyn FireHandler> =
            Arc::new(ViewOrchestratorFireHandler { view_orchestrator });
        Self::new(
            schema_manager,
            sled_pool,
            clock,
            fire_handler,
            firing_writer,
        )
    }

    fn sled_tree(&self) -> Option<sled::Tree> {
        let pool = self.sled_pool.as_ref()?;
        let guard = pool.acquire_arc().ok()?;
        guard.db().open_tree(TRIGGER_STATE_TREE).ok()
    }

    async fn load_persisted(&self, view_name: &str) -> PersistedViewState {
        let Some(tree) = self.sled_tree() else {
            return PersistedViewState::default();
        };
        match tree.get(view_name.as_bytes()) {
            Ok(Some(bytes)) => serde_json::from_slice(&bytes).unwrap_or_default(),
            _ => PersistedViewState::default(),
        }
    }

    async fn persist(&self, view_name: &str, state: &PersistedViewState) {
        let Some(tree) = self.sled_tree() else {
            return;
        };
        if let Ok(bytes) = serde_json::to_vec(state) {
            if let Err(e) = tree.insert(view_name.as_bytes(), bytes) {
                warn!("trigger_state persist failed for '{}': {}", view_name, e);
            }
        }
    }

    async fn runtime_for(&self, view_name: &str) -> Arc<ViewRuntime> {
        let mut map = self.runtimes.lock().await;
        if let Some(rt) = map.get(view_name) {
            return Arc::clone(rt);
        }
        let persisted = self.load_persisted(view_name).await;
        let rt = Arc::new(ViewRuntime::new(persisted));
        map.insert(view_name.to_string(), Arc::clone(&rt));
        rt
    }

    /// Snapshot views in the registry that have a trigger reacting to
    /// mutations on `schema_name`. Phase 1 rebuilds this per mutation
    /// from the live registry — no separate mutation_schema_index cache
    /// because the registry lock is already cheap and AddView races are
    /// impossible with a rebuild-per-call.
    ///
    /// Each Trigger carries its own `schemas` subscription list; only
    /// triggers whose list includes `schema_name` are returned.
    fn views_triggered_by(
        &self,
        schema_name: &str,
    ) -> Result<Vec<(String, Vec<Trigger>)>, SchemaError> {
        let registry = self
            .schema_manager
            .view_registry()
            .lock()
            .map_err(|_| SchemaError::InvalidData("view_registry lock".to_string()))?;

        let mut out = Vec::new();
        for view in registry.list_views() {
            let triggers: Vec<Trigger> = view
                .effective_triggers()
                .into_iter()
                .filter(|t| t.is_write_triggered() && t.schemas().iter().any(|s| s == schema_name))
                .collect();
            if !triggers.is_empty() {
                out.push((view.name.clone(), triggers));
            }
        }
        Ok(out)
    }

    /// Public entry point for mutation notifications. Called by
    /// `MutationManager` after a successful mutation batch commits.
    pub async fn on_mutation_notified(
        self: &Arc<Self>,
        schema_name: &str,
    ) -> Result<(), SchemaError> {
        // The TriggerFiring schema is written BY the runner itself — we
        // must not re-enter here or we'd recurse on our own audit writes.
        if schema_name == TRIGGER_FIRING_SCHEMA_NAME {
            return Ok(());
        }

        let triggered = self.views_triggered_by(schema_name)?;
        if triggered.is_empty() {
            return Ok(());
        }

        let now = self.clock.now_ms();
        for (view_name, triggers) in triggered {
            let rt = self.runtime_for(&view_name).await;

            for (idx, trig) in triggers.iter().enumerate() {
                match trig {
                    Trigger::OnWrite { .. } => {
                        self.dispatch_inline_once(&view_name, idx, &rt).await;
                    }
                    Trigger::OnWriteCoalesced {
                        min_batch,
                        debounce_ms,
                        max_wait_ms,
                        ..
                    } => {
                        let should_fire = {
                            let mut st = rt.persisted.lock().await;
                            if st.pending_count == 0 {
                                st.first_event_ms = now;
                            }
                            st.pending_count = st.pending_count.saturating_add(1);
                            st.last_event_ms = now;

                            let batch_ok = st.pending_count >= *min_batch;
                            let debounce_ok =
                                (now - st.last_event_ms) >= *debounce_ms as i64 && batch_ok;
                            let max_wait_ok = (now - st.first_event_ms) >= *max_wait_ms as i64;
                            let fire = (batch_ok && debounce_ok) || max_wait_ok;

                            // Persist to survive a restart before the fire.
                            let snapshot = st.clone();
                            drop(st);
                            self.persist(&view_name, &snapshot).await;

                            fire
                        };
                        if should_fire {
                            // Coalesced fires stay async — the caller isn't
                            // expecting synchronous invalidation for these.
                            self.dispatch_nonblocking(&view_name, idx, &rt).await;
                        } else {
                            self.schedule_coalesce_check(
                                &view_name,
                                idx,
                                now,
                                *debounce_ms,
                                *max_wait_ms,
                            )
                            .await;
                        }
                    }
                    Trigger::ScheduledIfDirty { .. } => {
                        let mut st = rt.persisted.lock().await;
                        st.dirty = true;
                        let snapshot = st.clone();
                        drop(st);
                        self.persist(&view_name, &snapshot).await;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Push a scheduler entry so the background tick will re-examine the
    /// coalesce state at the earliest possible fire time. Idempotent —
    /// duplicate entries just mean extra wakeups, which is safe.
    async fn schedule_coalesce_check(
        &self,
        view_name: &str,
        trigger_index: usize,
        now_ms: i64,
        debounce_ms: u64,
        max_wait_ms: u64,
    ) {
        let fire_at = now_ms + debounce_ms.min(max_wait_ms) as i64;
        let mut heap = self.scheduler.lock().await;
        heap.push(Reverse(ScheduledFire {
            fire_at_ms: fire_at,
            view_name: view_name.to_string(),
            trigger_index,
        }));
        self.scheduler_notify.notify_one();
    }

    /// Try to claim the in-flight slot for `view_name` and spawn the
    /// fire. If a fire is already in flight, flip the refire flag and
    /// return — the running fire picks it up before releasing the slot.
    async fn dispatch_nonblocking(
        self: &Arc<Self>,
        view_name: &str,
        trigger_index: usize,
        rt: &Arc<ViewRuntime>,
    ) {
        use std::sync::atomic::Ordering;
        if rt.dispatch_in_flight.swap(true, Ordering::SeqCst) {
            // Slot was already held → record a refire for the runner.
            rt.refire_requested.store(true, Ordering::SeqCst);
            return;
        }
        let runner = Arc::clone(self);
        let rt = Arc::clone(rt);
        let view_name = view_name.to_string();
        tokio::spawn(async move {
            runner
                .run_fire_with_refire_loop(&view_name, trigger_index, &rt)
                .await;
            rt.dispatch_in_flight.store(false, Ordering::SeqCst);
        });
    }

    /// Inline dispatch: claim the slot, run ONE fire synchronously,
    /// then release. On failure, hand off to the spawned retry loop so
    /// the caller doesn't block through exponential backoff.
    ///
    /// This path exists specifically for OnWrite triggers where the
    /// caller (the mutation path) must observe the invalidation
    /// synchronously — downstream tests read `ViewCacheState::Computing`
    /// immediately after the mutation returns, and the old implicit
    /// cascade did the invalidation inline too.
    async fn dispatch_inline_once(
        self: &Arc<Self>,
        view_name: &str,
        trigger_index: usize,
        rt: &Arc<ViewRuntime>,
    ) {
        use std::sync::atomic::Ordering;
        if rt.dispatch_in_flight.swap(true, Ordering::SeqCst) {
            rt.refire_requested.store(true, Ordering::SeqCst);
            return;
        }

        // Single synchronous attempt.
        let success = self.fire_once(view_name, trigger_index, rt).await;

        if success {
            // Drain any refire that piled up during the fire. If the
            // caller just set refire, run it inline too so the sync
            // contract holds for the whole burst.
            loop {
                if !rt.refire_requested.swap(false, Ordering::SeqCst) {
                    break;
                }
                let ok = self.fire_once(view_name, trigger_index, rt).await;
                if !ok {
                    break;
                }
            }
            rt.dispatch_in_flight.store(false, Ordering::SeqCst);
            return;
        }

        // Failure: hand off retries to the background so the mutation
        // path isn't stuck through a 60s backoff.
        let runner = Arc::clone(self);
        let rt_clone = Arc::clone(rt);
        let view_name = view_name.to_string();
        tokio::spawn(async move {
            // Sleep before the retry — backoff driven by fail_streak.
            let streak = rt_clone.persisted.lock().await.fail_streak;
            runner.clock.sleep(exp_backoff_ms(streak)).await;
            runner
                .run_fire_with_refire_loop(&view_name, trigger_index, &rt_clone)
                .await;
            rt_clone.dispatch_in_flight.store(false, Ordering::SeqCst);
        });
    }

    /// Execute one fire attempt and persist results. Returns true when
    /// the fire succeeded AND the audit row was written.
    async fn fire_once(
        self: &Arc<Self>,
        view_name: &str,
        trigger_index: usize,
        rt: &Arc<ViewRuntime>,
    ) -> bool {
        {
            let st = rt.persisted.lock().await;
            if st.quarantined {
                return false;
            }
        }

        let fire_start = self.clock.now_ms();
        let outcome = self.fire_handler.fire(view_name).await;
        let fire_end = self.clock.now_ms();

        let (status_enum, _streak, quarantined) = {
            let mut st = rt.persisted.lock().await;
            if outcome.success {
                st.fail_streak = 0;
                st.pending_count = 0;
                st.first_event_ms = 0;
                st.dirty = false;
                (FiringStatus::Success, 0, false)
            } else {
                st.fail_streak = st.fail_streak.saturating_add(1);
                let quarantine = st.fail_streak >= QUARANTINE_FAIL_STREAK;
                if quarantine {
                    st.quarantined = true;
                }
                let streak = st.fail_streak;
                (
                    if quarantine {
                        FiringStatus::Quarantined
                    } else {
                        FiringStatus::Error
                    },
                    streak,
                    quarantine,
                )
            }
        };

        let trigger_id = format!("{}:{}", view_name, trigger_index);
        let record = FiringRecord {
            trigger_id,
            view_name: view_name.to_string(),
            fired_at_ms: fire_start,
            duration_ms: fire_end - fire_start,
            status: status_enum,
            input_row_count: outcome.input_row_count,
            output_row_count: outcome.output_row_count,
            error_message: outcome.error_message.clone(),
        };

        let write_result = self.firing_writer.write_firing(record.clone()).await;
        if write_result.is_ok() {
            let mut st = rt.persisted.lock().await;
            st.last_fire_ms = fire_end;
            let snap = st.clone();
            drop(st);
            self.persist(view_name, &snap).await;
        } else {
            warn!(
                "TriggerFiring write failed for view '{}' — will retry (status was {:?})",
                view_name, record.status
            );
        }

        outcome.success && write_result.is_ok() && !quarantined
    }

    async fn run_fire_with_refire_loop(
        self: Arc<Self>,
        view_name: &str,
        trigger_index: usize,
        rt: &Arc<ViewRuntime>,
    ) {
        loop {
            // Check quarantined before each attempt.
            {
                let st = rt.persisted.lock().await;
                if st.quarantined {
                    debug!("view '{}' is quarantined, skipping fire", view_name);
                    return;
                }
            }

            let fire_start = self.clock.now_ms();
            let outcome = self.fire_handler.fire(view_name).await;
            let fire_end = self.clock.now_ms();

            let (status_enum, new_fail_streak, should_quarantine) = {
                let mut st = rt.persisted.lock().await;
                if outcome.success {
                    st.fail_streak = 0;
                    // Coalesce counters reset on successful fire — the
                    // batch has shipped.
                    st.pending_count = 0;
                    st.first_event_ms = 0;
                    st.dirty = false;
                    (FiringStatus::Success, 0, false)
                } else {
                    st.fail_streak = st.fail_streak.saturating_add(1);
                    let quarantine = st.fail_streak >= QUARANTINE_FAIL_STREAK;
                    if quarantine {
                        st.quarantined = true;
                    }
                    let streak = st.fail_streak;
                    (
                        if quarantine {
                            FiringStatus::Quarantined
                        } else {
                            FiringStatus::Error
                        },
                        streak,
                        quarantine,
                    )
                }
            };

            let trigger_id = format!("{}:{}", view_name, trigger_index);
            let record = FiringRecord {
                trigger_id,
                view_name: view_name.to_string(),
                fired_at_ms: fire_start,
                duration_ms: fire_end - fire_start,
                status: status_enum,
                input_row_count: outcome.input_row_count,
                output_row_count: outcome.output_row_count,
                error_message: outcome.error_message.clone(),
            };

            let write_result = self.firing_writer.write_firing(record.clone()).await;

            // At-least-once: advance last_fire_ms only when the audit row
            // landed. A failed write leaves the previous last_fire_ms in
            // place so the next scheduler tick / mutation will retry.
            if write_result.is_ok() {
                let mut st = rt.persisted.lock().await;
                st.last_fire_ms = fire_end;
                let snap = st.clone();
                drop(st);
                self.persist(view_name, &snap).await;
            } else {
                warn!(
                    "TriggerFiring write failed for view '{}' — will retry (status was {:?})",
                    view_name, record.status
                );
            }

            if outcome.success {
                if !rt
                    .refire_requested
                    .swap(false, std::sync::atomic::Ordering::SeqCst)
                {
                    return;
                }
                // Refire requested — loop.
                continue;
            } else {
                if should_quarantine {
                    // Permanent stop — no further retries.
                    return;
                }
                // Exponential backoff on failure. Sleep before next
                // attempt so we don't spin on transient errors.
                let backoff = exp_backoff_ms(new_fail_streak);
                self.clock.sleep(backoff).await;
                continue;
            }
        }
    }

    /// Tick the scheduler once: fire anything whose fire_at_ms has passed,
    /// then return the earliest remaining fire_at_ms. Caller sleeps until
    /// that time (or until woken by `scheduler_notify`) before calling
    /// tick() again.
    async fn tick_once(self: &Arc<Self>) -> Option<i64> {
        let now = self.clock.now_ms();

        // First: enqueue new Scheduled/ScheduledIfDirty fires that are due
        // based on each view's interval. We rebuild from the registry
        // rather than maintain a cache; views rarely change shape.
        self.populate_scheduled_from_registry(now).await;

        // Pop all due fires. Dedupe by (view, trigger_index) — a coalesce
        // trigger can push one heap entry per mutation, and we only want
        // one dispatch per tick for each (view, trigger_index) pair.
        let due: Vec<ScheduledFire> = {
            let mut heap = self.scheduler.lock().await;
            let mut out = Vec::new();
            let mut seen: std::collections::HashSet<(String, usize)> =
                std::collections::HashSet::new();
            while let Some(Reverse(top)) = heap.peek() {
                if top.fire_at_ms > now {
                    break;
                }
                let Reverse(f) = heap.pop().unwrap();
                if seen.insert((f.view_name.clone(), f.trigger_index)) {
                    out.push(f);
                }
            }
            out
        };

        for fire in due {
            self.process_scheduled_fire(fire, now).await;
        }

        let next = {
            let heap = self.scheduler.lock().await;
            heap.peek().map(|Reverse(f)| f.fire_at_ms)
        };
        next
    }

    /// For every Scheduled or ScheduledIfDirty trigger in the registry,
    /// enqueue the next fire if it's not already scheduled. Each
    /// (view, trigger_index) gets at most one outstanding heap entry at
    /// a time so we don't flood the heap.
    async fn populate_scheduled_from_registry(self: &Arc<Self>, now_ms: i64) {
        let Ok(triggers) = self.list_scheduled_triggers() else {
            return;
        };
        let existing: std::collections::HashSet<(String, usize)> = {
            let heap = self.scheduler.lock().await;
            heap.iter()
                .map(|Reverse(f)| (f.view_name.clone(), f.trigger_index))
                .collect()
        };

        for (view_name, idx, trig) in triggers {
            if existing.contains(&(view_name.clone(), idx)) {
                continue;
            }
            let (cron_expr, tz_str) = match &trig {
                Trigger::Scheduled { cron, timezone, .. }
                | Trigger::ScheduledIfDirty { cron, timezone, .. } => {
                    (cron.as_str(), timezone.as_str())
                }
                _ => continue,
            };
            let Some(next_at) = next_fire_from_cron(cron_expr, tz_str, now_ms) else {
                warn!(
                    "trigger '{}:{}': failed to compute next fire from cron='{}' tz='{}'",
                    view_name, idx, cron_expr, tz_str
                );
                continue;
            };
            let mut heap = self.scheduler.lock().await;
            heap.push(Reverse(ScheduledFire {
                fire_at_ms: next_at,
                view_name,
                trigger_index: idx,
            }));
        }
    }

    fn list_scheduled_triggers(&self) -> Result<Vec<(String, usize, Trigger)>, SchemaError> {
        let registry = self
            .schema_manager
            .view_registry()
            .lock()
            .map_err(|_| SchemaError::InvalidData("view_registry lock".to_string()))?;
        let mut out = Vec::new();
        for view in registry.list_views() {
            for (idx, trig) in view.effective_triggers().iter().enumerate() {
                if trig.is_scheduled() {
                    out.push((view.name.clone(), idx, trig.clone()));
                }
            }
        }
        Ok(out)
    }

    async fn process_scheduled_fire(self: &Arc<Self>, fire: ScheduledFire, now_ms: i64) {
        // Look up the current trigger config — it may have changed since
        // the heap entry was pushed.
        let trigger = {
            let registry = match self.schema_manager.view_registry().lock() {
                Ok(r) => r,
                Err(_) => return,
            };
            let Some(view) = registry.get_view(&fire.view_name) else {
                return;
            };
            let trigs = view.effective_triggers();
            let Some(t) = trigs.get(fire.trigger_index).cloned() else {
                return;
            };
            t
        };

        let rt = self.runtime_for(&fire.view_name).await;

        let should_fire = match &trigger {
            Trigger::Scheduled { skip_if_idle, .. } => {
                if *skip_if_idle {
                    let st = rt.persisted.lock().await;
                    st.dirty
                } else {
                    true
                }
            }
            Trigger::ScheduledIfDirty { .. } => {
                let st = rt.persisted.lock().await;
                st.dirty
            }
            Trigger::OnWriteCoalesced {
                min_batch,
                debounce_ms,
                max_wait_ms,
                ..
            } => {
                // Coalesce scheduler tick: fire if the debounce window has
                // elapsed AND we have a batch, OR if max_wait is hit.
                let st = rt.persisted.lock().await;
                if st.pending_count == 0 {
                    false
                } else {
                    let batch_ok = st.pending_count >= *min_batch;
                    let debounce_ok =
                        (now_ms - st.last_event_ms) >= *debounce_ms as i64 && batch_ok;
                    let max_wait_ok = (now_ms - st.first_event_ms) >= *max_wait_ms as i64;
                    (batch_ok && debounce_ok) || max_wait_ok
                }
            }
            _ => false,
        };

        let reschedule_cron = match &trigger {
            Trigger::Scheduled { cron, timezone, .. }
            | Trigger::ScheduledIfDirty { cron, timezone, .. } => {
                Some((cron.clone(), timezone.clone()))
            }
            _ => None,
        };

        if !should_fire {
            // Re-enqueue the next cron occurrence for scheduled triggers —
            // skip_if_idle / dirty-check skipped this tick but the next
            // cron tick should still be tracked.
            if let Some((cron, tz)) = reschedule_cron {
                if let Some(next_at) = next_fire_from_cron(&cron, &tz, now_ms) {
                    let mut heap = self.scheduler.lock().await;
                    heap.push(Reverse(ScheduledFire {
                        fire_at_ms: next_at,
                        view_name: fire.view_name,
                        trigger_index: fire.trigger_index,
                    }));
                }
            }
            return;
        }

        self.dispatch_nonblocking(&fire.view_name, fire.trigger_index, &rt)
            .await;

        // Re-arm the next cron occurrence.
        if let Some((cron, tz)) = reschedule_cron {
            if let Some(next_at) = next_fire_from_cron(&cron, &tz, now_ms) {
                let mut heap = self.scheduler.lock().await;
                heap.push(Reverse(ScheduledFire {
                    fire_at_ms: next_at,
                    view_name: fire.view_name,
                    trigger_index: fire.trigger_index,
                }));
            }
        }
    }

    /// Run the scheduler loop. Intended to be spawned as a long-lived
    /// tokio task; returns only when the `shutdown` notify is fired.
    pub async fn run_scheduler_loop(self: Arc<Self>, shutdown: Arc<Notify>) {
        loop {
            let next_ms = self.tick_once().await;
            let sleep_ms = match next_ms {
                Some(t) => {
                    let now = self.clock.now_ms();
                    ((t - now).max(0) as u64).max(10)
                }
                None => 60_000, // idle: poll every minute for newly added views
            };
            tokio::select! {
                _ = self.clock.sleep(sleep_ms) => {},
                _ = self.scheduler_notify.notified() => {},
                _ = shutdown.notified() => return,
            }
        }
    }

    #[cfg(test)]
    pub(crate) async fn test_is_quarantined(&self, view_name: &str) -> bool {
        let rt = self.runtime_for(view_name).await;
        let st = rt.persisted.lock().await;
        st.quarantined
    }

    #[cfg(test)]
    pub(crate) async fn test_last_fire_ms(&self, view_name: &str) -> i64 {
        let rt = self.runtime_for(view_name).await;
        let st = rt.persisted.lock().await;
        st.last_fire_ms
    }

    #[cfg(test)]
    pub(crate) async fn test_fail_streak(&self, view_name: &str) -> u32 {
        let rt = self.runtime_for(view_name).await;
        let st = rt.persisted.lock().await;
        st.fail_streak
    }
}

#[async_trait]
impl<C: Clock> TriggerDispatcher for TriggerRunner<C> {
    async fn on_mutation(
        &self,
        schema_name: &str,
        _fields_affected: &[String],
    ) -> Result<(), SchemaError> {
        // Self is not Arc here — the call to on_mutation_notified needs
        // Arc<Self>. This impl is for external dispatch via
        // `Arc<dyn TriggerDispatcher>`; the caller already holds an Arc.
        // We work around the Arc-shape mismatch by going through a
        // dedicated entry that takes &self and defers spawn to the
        // notified variant.
        //
        // In practice MutationManager always holds Arc<dyn TriggerDispatcher>,
        // so this trampoline is invoked through the Arc, but we can't
        // recover the Arc from &self. Production callers should prefer
        // `TriggerRunner::dispatch_for_mutation`.
        if schema_name == TRIGGER_FIRING_SCHEMA_NAME {
            return Ok(());
        }
        warn!(
            "TriggerDispatcher::on_mutation trampoline invoked for '{}' — \
             prefer dispatch_for_mutation with Arc<TriggerRunner>",
            schema_name
        );
        Ok(())
    }
}

/// Dispatcher that holds an `Arc<TriggerRunner>` so it can invoke the
/// `Arc<Self>`-receiver methods. MutationManager depends on this trait
/// object instead of `TriggerRunner` directly to avoid pulling the
/// runner's concrete generic parameter through the type graph.
pub struct ArcTriggerDispatcher<C: Clock> {
    runner: Arc<TriggerRunner<C>>,
}

impl<C: Clock> ArcTriggerDispatcher<C> {
    pub fn new(runner: Arc<TriggerRunner<C>>) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl<C: Clock> TriggerDispatcher for ArcTriggerDispatcher<C> {
    async fn on_mutation(
        &self,
        schema_name: &str,
        _fields_affected: &[String],
    ) -> Result<(), SchemaError> {
        self.runner.on_mutation_notified(schema_name).await
    }
}

/// `FireHandler` that delegates to the existing `ViewOrchestrator` cache
/// invalidation + background precompute. We don't duplicate materialization
/// — this is exactly the path the old implicit cascade used to take, just
/// gated behind explicit triggers now.
pub struct ViewOrchestratorFireHandler {
    view_orchestrator: Arc<ViewOrchestrator>,
}

#[async_trait]
impl FireHandler for ViewOrchestratorFireHandler {
    async fn fire(&self, view_name: &str) -> FireOutcome {
        match self.view_orchestrator.invalidate_view(view_name).await {
            Ok(()) => FireOutcome::success(0, 0),
            Err(e) => FireOutcome::error(e.to_string()),
        }
    }
}

/// `FiringWriter` that persists TriggerFiring rows through the normal
/// mutation pipeline. This does mean a small cycle: runner → writer →
/// MutationManager → back to runner via TriggerDispatcher. The
/// `TriggerRunner::on_mutation_notified` path short-circuits on the
/// TriggerFiring schema name so we never recurse on our own audit writes.
pub struct MutationManagerFiringWriter {
    mutation_manager: std::sync::Weak<super::mutation_manager::MutationManager>,
    pub_key: String,
}

impl MutationManagerFiringWriter {
    pub fn new(
        mutation_manager: std::sync::Weak<super::mutation_manager::MutationManager>,
        pub_key: String,
    ) -> Self {
        Self {
            mutation_manager,
            pub_key,
        }
    }
}

#[async_trait]
impl FiringWriter for MutationManagerFiringWriter {
    async fn write_firing(&self, row: FiringRecord) -> Result<(), SchemaError> {
        let mutation = row.into_mutation(&self.pub_key);
        let mm = self.mutation_manager.upgrade().ok_or_else(|| {
            SchemaError::InvalidData(
                "MutationManager was dropped; cannot write TriggerFiring row".into(),
            )
        })?;
        mm.write_mutations_batch_async(vec![mutation]).await?;
        Ok(())
    }
}

/// Compute the next cron occurrence strictly after `now_ms` in the given
/// IANA timezone. Returns the fire time as Unix epoch milliseconds (UTC),
/// or `None` if the cron expression or timezone fail to parse.
///
/// DST handling follows croner: `find_next_occurrence(…, false)` advances
/// to the first valid tick after a spring-forward gap, and fires once per
/// local clock-time even during a fall-back overlap (croner walks UTC
/// under the hood, so the ambiguous hour doesn't double-fire).
fn next_fire_from_cron(cron_expr: &str, tz_str: &str, now_ms: i64) -> Option<i64> {
    let cron = croner::Cron::new(cron_expr).parse().ok()?;
    let tz: chrono_tz::Tz = tz_str.parse().ok()?;
    let now_utc = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(now_ms)?;
    let now_in_tz = now_utc.with_timezone(&tz);
    let next = cron.find_next_occurrence(&now_in_tz, false).ok()?;
    Some(next.with_timezone(&chrono::Utc).timestamp_millis())
}

fn exp_backoff_ms(fail_streak: u32) -> u64 {
    if fail_streak == 0 {
        return BACKOFF_MIN_MS;
    }
    // 1s, 2s, 4s, 8s, ... capped at 60s
    let shift = fail_streak.saturating_sub(1).min(6);
    (BACKOFF_MIN_MS << shift).min(BACKOFF_MAX_MS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triggers::clock::MockClock;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use tokio::sync::Mutex as TokioMutex;

    struct RecordingFireHandler {
        outcomes: TokioMutex<Vec<FireOutcome>>,
        call_count: AtomicU32,
        fired_views: TokioMutex<Vec<String>>,
    }

    impl RecordingFireHandler {
        fn with_outcomes(outcomes: Vec<FireOutcome>) -> Arc<Self> {
            Arc::new(Self {
                outcomes: TokioMutex::new(outcomes.into_iter().rev().collect()),
                call_count: AtomicU32::new(0),
                fired_views: TokioMutex::new(Vec::new()),
            })
        }

        fn all_success() -> Arc<Self> {
            Self::with_outcomes(
                (0..100)
                    .map(|_| FireOutcome::success(0, 0))
                    .collect::<Vec<_>>(),
            )
        }

        fn all_error() -> Arc<Self> {
            Self::with_outcomes(
                (0..100)
                    .map(|_| FireOutcome::error("mock failure"))
                    .collect::<Vec<_>>(),
            )
        }
    }

    #[async_trait]
    impl FireHandler for RecordingFireHandler {
        async fn fire(&self, view_name: &str) -> FireOutcome {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            self.fired_views.lock().await.push(view_name.to_string());
            self.outcomes
                .lock()
                .await
                .pop()
                .unwrap_or_else(|| FireOutcome::success(0, 0))
        }
    }

    /// Test double for `FiringWriter`. Supports two independent failure
    /// modes used by the at-least-once retry tests:
    ///
    /// * `fail_next` — one-shot: the next call fails, subsequent succeed.
    /// * `fail_count` — N-shot: the next `fail_count` calls fail, then
    ///   successes resume. Decremented per failing call.
    ///
    /// Both failure modes count toward `call_count` and `attempted_rows`
    /// so tests can assert total attempts regardless of outcome.
    struct CountingFiringWriter {
        rows: TokioMutex<Vec<FiringRecord>>,
        fail_next: std::sync::atomic::AtomicBool,
        fail_count: AtomicU32,
        call_count: AtomicU32,
    }

    impl CountingFiringWriter {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                rows: TokioMutex::new(Vec::new()),
                fail_next: std::sync::atomic::AtomicBool::new(false),
                fail_count: AtomicU32::new(0),
                call_count: AtomicU32::new(0),
            })
        }

        fn set_fail_count(&self, n: u32) {
            self.fail_count.store(n, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl FiringWriter for CountingFiringWriter {
        async fn write_firing(&self, row: FiringRecord) -> Result<(), SchemaError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            if self.fail_next.swap(false, Ordering::SeqCst) {
                return Err(SchemaError::InvalidData("mock write fail".into()));
            }
            // fail_count: decrement-and-check. The CAS loop ensures we
            // don't underflow if two calls race while fail_count == 1.
            loop {
                let cur = self.fail_count.load(Ordering::SeqCst);
                if cur == 0 {
                    break;
                }
                if self
                    .fail_count
                    .compare_exchange(cur, cur - 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    return Err(SchemaError::InvalidData("mock write fail".into()));
                }
            }
            self.rows.lock().await.push(row);
            Ok(())
        }
    }

    async fn make_schema_manager() -> Arc<SchemaCore> {
        Arc::new(SchemaCore::new_for_testing().await.unwrap())
    }

    fn register_view(
        schema_manager: &Arc<SchemaCore>,
        name: &str,
        source_schema: &str,
        triggers: Vec<Trigger>,
    ) {
        use crate::schema::types::field_value_type::FieldValueType;
        use crate::schema::types::operations::Query;
        use crate::schema::types::schema::DeclarativeSchemaType as SchemaType;
        use crate::view::types::TransformView;
        use std::collections::HashMap;

        let mut view = TransformView::new(
            name,
            SchemaType::Single,
            None,
            vec![Query::new(
                source_schema.to_string(),
                vec!["f1".to_string()],
            )],
            None,
            HashMap::from([("f1".to_string(), FieldValueType::Any)]),
        );
        view.triggers = triggers;

        let mut reg = schema_manager.view_registry().lock().unwrap();
        reg.register_view(view, |_| true).unwrap();
    }

    fn make_runner<C: Clock>(
        schema_manager: Arc<SchemaCore>,
        clock: Arc<C>,
        fire_handler: Arc<dyn FireHandler>,
        writer: Arc<dyn FiringWriter>,
    ) -> Arc<TriggerRunner<C>> {
        Arc::new(TriggerRunner::new(
            schema_manager,
            None,
            clock,
            fire_handler,
            writer,
        ))
    }

    #[tokio::test]
    async fn on_write_single_mutation_fires_once() {
        let sm = make_schema_manager().await;
        register_view(
            &sm,
            "V1",
            "S1",
            vec![Trigger::OnWrite {
                schemas: vec!["S1".into()],
            }],
        );
        let clock = Arc::new(MockClock::new(1_000));
        let fire = RecordingFireHandler::all_success();
        let writer = CountingFiringWriter::new();

        let runner = make_runner(
            Arc::clone(&sm),
            clock,
            Arc::clone(&fire) as Arc<dyn FireHandler>,
            Arc::clone(&writer) as Arc<dyn FiringWriter>,
        );

        runner.on_mutation_notified("S1").await.unwrap();

        // Let the spawned fire task run. on_mutation_notified spawns;
        // we need to yield until the mutex is released.
        for _ in 0..20 {
            if fire.call_count.load(Ordering::SeqCst) > 0 {
                break;
            }
            tokio::task::yield_now().await;
        }

        assert_eq!(fire.call_count.load(Ordering::SeqCst), 1);
        let rows = writer.rows.lock().await.clone();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].view_name, "V1");
        assert_eq!(rows[0].trigger_id, "V1:0");
        assert_eq!(rows[0].status, FiringStatus::Success);
    }

    #[tokio::test]
    async fn coalesced_fires_at_min_batch() {
        let sm = make_schema_manager().await;
        register_view(
            &sm,
            "V1",
            "S1",
            vec![Trigger::OnWriteCoalesced {
                schemas: vec!["S1".into()],
                min_batch: 3,
                debounce_ms: 0,
                max_wait_ms: 10_000,
            }],
        );
        let clock = Arc::new(MockClock::new(0));
        let fire = RecordingFireHandler::all_success();
        let writer = CountingFiringWriter::new();
        let runner = make_runner(
            Arc::clone(&sm),
            clock,
            Arc::clone(&fire) as Arc<dyn FireHandler>,
            Arc::clone(&writer) as Arc<dyn FiringWriter>,
        );

        runner.on_mutation_notified("S1").await.unwrap();
        runner.on_mutation_notified("S1").await.unwrap();
        assert_eq!(fire.call_count.load(Ordering::SeqCst), 0);

        runner.on_mutation_notified("S1").await.unwrap();
        for _ in 0..30 {
            if fire.call_count.load(Ordering::SeqCst) > 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(fire.call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn coalesced_fires_at_max_wait_even_below_batch() {
        let sm = make_schema_manager().await;
        register_view(
            &sm,
            "V1",
            "S1",
            vec![Trigger::OnWriteCoalesced {
                schemas: vec!["S1".into()],
                min_batch: 100,
                debounce_ms: 10_000,
                max_wait_ms: 500,
            }],
        );
        let clock = Arc::new(MockClock::new(0));
        let fire = RecordingFireHandler::all_success();
        let writer = CountingFiringWriter::new();
        let runner = make_runner(
            Arc::clone(&sm),
            Arc::clone(&clock),
            Arc::clone(&fire) as Arc<dyn FireHandler>,
            Arc::clone(&writer) as Arc<dyn FiringWriter>,
        );

        runner.on_mutation_notified("S1").await.unwrap();
        // Advance past max_wait_ms, arrive with a single mutation.
        clock.advance(600);
        runner.on_mutation_notified("S1").await.unwrap();

        for _ in 0..30 {
            if fire.call_count.load(Ordering::SeqCst) > 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(fire.call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn coalesced_respects_debounce_window() {
        let sm = make_schema_manager().await;
        register_view(
            &sm,
            "V1",
            "S1",
            vec![Trigger::OnWriteCoalesced {
                schemas: vec!["S1".into()],
                min_batch: 2,
                debounce_ms: 100,
                max_wait_ms: 100_000,
            }],
        );
        let clock = Arc::new(MockClock::new(0));
        let fire = RecordingFireHandler::all_success();
        let writer = CountingFiringWriter::new();
        let runner = make_runner(
            Arc::clone(&sm),
            Arc::clone(&clock),
            Arc::clone(&fire) as Arc<dyn FireHandler>,
            Arc::clone(&writer) as Arc<dyn FiringWriter>,
        );

        runner.on_mutation_notified("S1").await.unwrap();
        runner.on_mutation_notified("S1").await.unwrap();
        // Same-instant mutations don't satisfy debounce (last_event == now).
        assert_eq!(fire.call_count.load(Ordering::SeqCst), 0);

        // Advance past debounce and tick the scheduler so it picks up
        // the pending coalesce.
        clock.advance(200);
        runner.tick_once().await;

        for _ in 0..30 {
            if fire.call_count.load(Ordering::SeqCst) > 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(fire.call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn scheduled_if_dirty_only_fires_when_dirty() {
        // Cron "* * * * *" fires every minute on the 0-second boundary.
        // MockClock starts at epoch 0 (1970-01-01 00:00:00 UTC); next fire
        // from t=0 is 00:01:00 = 60_000 ms.
        let sm = make_schema_manager().await;
        register_view(
            &sm,
            "V1",
            "S1",
            vec![Trigger::ScheduledIfDirty {
                cron: "* * * * *".into(),
                timezone: "UTC".into(),
                window: None,
                schemas: vec!["S1".into()],
            }],
        );
        let clock = Arc::new(MockClock::new(0));
        let fire = RecordingFireHandler::all_success();
        let writer = CountingFiringWriter::new();
        let runner = make_runner(
            Arc::clone(&sm),
            Arc::clone(&clock),
            Arc::clone(&fire) as Arc<dyn FireHandler>,
            Arc::clone(&writer) as Arc<dyn FiringWriter>,
        );

        // First tick populates scheduler; not dirty yet.
        runner.tick_once().await;
        clock.advance(60_000);
        runner.tick_once().await;
        tokio::task::yield_now().await;
        assert_eq!(fire.call_count.load(Ordering::SeqCst), 0);

        // Mark dirty via mutation; next cron tick should fire.
        runner.on_mutation_notified("S1").await.unwrap();
        clock.advance(60_000);
        runner.tick_once().await;
        for _ in 0..30 {
            if fire.call_count.load(Ordering::SeqCst) > 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(fire.call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn scheduled_fires_on_cron_tick() {
        // "0 2 * * *" = 02:00 UTC daily. MockClock at epoch 0 → next fire
        // is 1970-01-01 02:00:00 UTC = 2 * 3600 * 1000 = 7_200_000 ms.
        let sm = make_schema_manager().await;
        register_view(
            &sm,
            "V1",
            "S1",
            vec![Trigger::Scheduled {
                cron: "0 2 * * *".into(),
                timezone: "UTC".into(),
                window: None,
                skip_if_idle: false,
                schemas: vec!["S1".into()],
            }],
        );
        let clock = Arc::new(MockClock::new(0));
        let fire = RecordingFireHandler::all_success();
        let writer = CountingFiringWriter::new();
        let runner = make_runner(
            Arc::clone(&sm),
            Arc::clone(&clock),
            Arc::clone(&fire) as Arc<dyn FireHandler>,
            Arc::clone(&writer) as Arc<dyn FiringWriter>,
        );

        runner.tick_once().await;
        // Before cron fires, no-op.
        clock.advance(3_600_000); // 1h: not yet 02:00
        runner.tick_once().await;
        tokio::task::yield_now().await;
        assert_eq!(fire.call_count.load(Ordering::SeqCst), 0);

        // Past 02:00 → should fire.
        clock.advance(24 * 3_600_000); // +24h, well past next 02:00
        runner.tick_once().await;
        for _ in 0..30 {
            if fire.call_count.load(Ordering::SeqCst) >= 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(fire.call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn scheduled_skip_if_idle_suppresses_fire_when_clean() {
        // `skip_if_idle` makes plain `Scheduled` behave like
        // `ScheduledIfDirty` — no fire when no mutation has landed.
        let sm = make_schema_manager().await;
        register_view(
            &sm,
            "V1",
            "S1",
            vec![Trigger::Scheduled {
                cron: "* * * * *".into(),
                timezone: "UTC".into(),
                window: None,
                skip_if_idle: true,
                schemas: vec!["S1".into()],
            }],
        );
        let clock = Arc::new(MockClock::new(0));
        let fire = RecordingFireHandler::all_success();
        let writer = CountingFiringWriter::new();
        let runner = make_runner(
            Arc::clone(&sm),
            Arc::clone(&clock),
            Arc::clone(&fire) as Arc<dyn FireHandler>,
            Arc::clone(&writer) as Arc<dyn FiringWriter>,
        );

        runner.tick_once().await;
        // Clean, no mutations: even past the cron tick, no fire.
        clock.advance(60_000);
        runner.tick_once().await;
        tokio::task::yield_now().await;
        assert_eq!(
            fire.call_count.load(Ordering::SeqCst),
            0,
            "skip_if_idle must suppress fire when dirty bit is clean"
        );
    }

    #[test]
    fn next_fire_from_cron_parses_and_steps_forward() {
        // 1970-01-01 00:00:00 UTC → next "0 2 * * *" is 1970-01-01 02:00 UTC.
        let next = next_fire_from_cron("0 2 * * *", "UTC", 0).expect("cron parse");
        assert_eq!(next, 2 * 3_600 * 1_000);

        // Invalid cron → None.
        assert!(next_fire_from_cron("not a cron", "UTC", 0).is_none());
        // Invalid tz → None.
        assert!(next_fire_from_cron("0 2 * * *", "Not/AZone", 0).is_none());
    }

    // --- DST / leap-year regression tests -----------------------------------
    //
    // These tests pin croner + chrono_tz behavior at the seams where cron
    // semantics get interesting: spring-forward gap, fall-back overlap,
    // Feb 29 across a non-leap year, and IANA timezone alias equivalence.
    // They assert absolute epoch-millis values computed from chrono so the
    // contract is explicit: if croner ever changes behavior in a future
    // release, these tests fail loudly rather than silently shifting fire
    // semantics.

    #[test]
    fn next_fire_from_cron_spring_forward_la_fires_at_first_valid_tick() {
        // 2026-03-08 is US spring-forward: LA local clock jumps
        // 01:59:59 PST → 03:00:00 PDT; the 02:00–02:59 hour does not exist.
        // Cron "0 2 * * *" nominally fires at 02:00 local, which is skipped
        // on that day. croner's find_next_occurrence(_, /*inclusive=*/false)
        // advances to the first valid tick after the gap, so the fire lands
        // at 03:00 PDT (= 10:00 UTC) on the transition day.
        use chrono::TimeZone;
        let tz: chrono_tz::Tz = "America/Los_Angeles".parse().unwrap();
        // now = 2026-03-08 00:00 PST (pre-transition, same local day).
        let now_ms = tz
            .with_ymd_and_hms(2026, 3, 8, 0, 0, 0)
            .single()
            .expect("unambiguous local time")
            .timestamp_millis();
        let next_ms = next_fire_from_cron("0 2 * * *", "America/Los_Angeles", now_ms)
            .expect("cron should parse and step forward");

        // Expected: 2026-03-08 03:00 PDT = 2026-03-08 10:00 UTC.
        let expected = chrono::Utc
            .with_ymd_and_hms(2026, 3, 8, 10, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        assert_eq!(
            next_ms, expected,
            "spring-forward: cron '0 2 * * *' should fire at the first valid \
             local tick after the DST gap (03:00 PDT), not skip the day"
        );
    }

    #[test]
    fn next_fire_from_cron_fall_back_la_fires_exactly_once() {
        // 2026-11-01 is US fall-back: LA local clock repeats
        // 01:00–01:59 (first as PDT, then as PST). A cron "0 1 * * *" must
        // fire exactly once on that date; if croner double-fires the
        // ambiguous hour, the next-fire computed from the first fire would
        // land on the same local date instead of the following day.
        use chrono::TimeZone;

        // now = 2026-11-01 00:30 PDT (pre-transition). Unambiguous.
        let tz: chrono_tz::Tz = "America/Los_Angeles".parse().unwrap();
        let now_ms = tz
            .with_ymd_and_hms(2026, 11, 1, 0, 30, 0)
            .single()
            .expect("unambiguous local time")
            .timestamp_millis();

        let first = next_fire_from_cron("0 1 * * *", "America/Los_Angeles", now_ms)
            .expect("cron should parse and step forward");
        // First fire: 2026-11-01 01:00 PDT = 2026-11-01 08:00 UTC.
        let expected_first = chrono::Utc
            .with_ymd_and_hms(2026, 11, 1, 8, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        assert_eq!(
            first, expected_first,
            "fall-back: first fire should be 01:00 PDT (the first 01:00 \
             of the ambiguous hour), not 00:00 or 02:00"
        );

        // Step again from the first fire. If croner double-fires, we'd get
        // 2026-11-01 01:00 PST = 2026-11-01 09:00 UTC (one hour later).
        // Correct behavior: 2026-11-02 01:00 PST = 2026-11-02 09:00 UTC
        // (25 hours later in UTC because the day has 25 hours).
        let second = next_fire_from_cron("0 1 * * *", "America/Los_Angeles", first)
            .expect("cron should parse and step forward");
        let expected_second = chrono::Utc
            .with_ymd_and_hms(2026, 11, 2, 9, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        assert_eq!(
            second, expected_second,
            "fall-back: cron '0 1 * * *' must not double-fire the ambiguous \
             hour — next fire after 01:00 PDT should be 01:00 PST on the \
             FOLLOWING day (25h later in UTC), not 01:00 PST the same day"
        );
    }

    #[test]
    fn next_fire_from_cron_feb_29_skips_to_next_leap_year() {
        // Cron "0 0 29 2 *" (midnight on Feb 29). now = 2027-02-28 00:00 UTC
        // — a non-leap year with no Feb 29 ahead in 2027. croner should
        // advance to the next leap year's Feb 29: 2028-02-29 00:00 UTC.
        use chrono::TimeZone;
        let now_ms = chrono::Utc
            .with_ymd_and_hms(2027, 2, 28, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        let next_ms = next_fire_from_cron("0 0 29 2 *", "UTC", now_ms)
            .expect("leap-year cron should return Some — croner advances to next matching Feb 29");

        let expected = chrono::Utc
            .with_ymd_and_hms(2028, 2, 29, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        assert_eq!(
            next_ms, expected,
            "Feb 29 cron on a non-leap year should advance to the next \
             leap year's Feb 29 (2028-02-29 00:00 UTC), not return None \
             and not fire on Feb 28 or Mar 1"
        );
    }

    #[test]
    fn next_fire_from_cron_tz_alias_pst8pdt_matches_la() {
        // chrono_tz accepts both the IANA primary name "America/Los_Angeles"
        // and the POSIX-style alias "PST8PDT" (defined as a Zone in the
        // northamerica tz source with US DST rules). Same cron + same
        // now_ms must produce the same next-fire in both, including across
        // DST transitions.
        use chrono::TimeZone;

        let la: chrono_tz::Tz = "America/Los_Angeles".parse().unwrap();
        let midwinter = la
            .with_ymd_and_hms(2026, 1, 15, 6, 30, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        let spring_forward_day = la
            .with_ymd_and_hms(2026, 3, 8, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();

        for (label, now_ms) in &[
            ("midwinter", midwinter),
            ("spring-forward-day", spring_forward_day),
        ] {
            let via_iana = next_fire_from_cron("0 2 * * *", "America/Los_Angeles", *now_ms)
                .expect("IANA name should parse");
            let via_alias = next_fire_from_cron("0 2 * * *", "PST8PDT", *now_ms)
                .expect("PST8PDT alias should parse");
            assert_eq!(
                via_iana, via_alias,
                "tz alias equivalence failed at {}: 'America/Los_Angeles' and \
                 'PST8PDT' should produce identical next-fire times",
                label
            );
        }
    }

    #[tokio::test]
    async fn fail_streak_triggers_quarantine_after_three_errors() {
        let sm = make_schema_manager().await;
        register_view(
            &sm,
            "V1",
            "S1",
            vec![Trigger::OnWrite {
                schemas: vec!["S1".into()],
            }],
        );
        let clock = Arc::new(MockClock::new(0));
        let fire = RecordingFireHandler::all_error();
        let writer = CountingFiringWriter::new();
        let runner = make_runner(
            Arc::clone(&sm),
            Arc::clone(&clock),
            Arc::clone(&fire) as Arc<dyn FireHandler>,
            Arc::clone(&writer) as Arc<dyn FiringWriter>,
        );

        // Kick off a fire; the refire loop will retry with backoff.
        // We drive MockClock forward past each backoff to unstick the
        // sleep between attempts.
        let runner2 = Arc::clone(&runner);
        let clock2 = Arc::clone(&clock);
        let handle = tokio::spawn(async move {
            for _ in 0..20 {
                clock2.advance(1_000);
                tokio::task::yield_now().await;
            }
            let _ = runner2;
        });

        runner.on_mutation_notified("S1").await.unwrap();

        for _ in 0..200 {
            if runner.test_is_quarantined("V1").await {
                break;
            }
            clock.advance(2_000);
            tokio::task::yield_now().await;
        }

        assert!(
            runner.test_is_quarantined("V1").await,
            "view should be quarantined after {} consecutive failures",
            QUARANTINE_FAIL_STREAK
        );

        let _ = handle.await;

        // Further mutations after quarantine must not produce fires.
        let count_before = fire.call_count.load(Ordering::SeqCst);
        runner.on_mutation_notified("S1").await.unwrap();
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
        let count_after = fire.call_count.load(Ordering::SeqCst);
        assert_eq!(count_after, count_before, "quarantine must block new fires");
    }

    #[tokio::test]
    async fn on_write_refire_coalesces_concurrent_second_mutation() {
        // If a second mutation lands while a fire is in flight, the
        // dispatcher must record a refire and the fire loop must re-run
        // exactly once for it — not two additional times, and not zero.
        use std::sync::atomic::AtomicBool;

        struct GatedHandler {
            release: Arc<AtomicBool>,
            count: AtomicU32,
        }

        #[async_trait]
        impl FireHandler for GatedHandler {
            async fn fire(&self, _view_name: &str) -> FireOutcome {
                // On the first call, hold until release is set, so the
                // caller can issue a second mutation while we're mid-fire.
                let was_first = self.count.fetch_add(1, Ordering::SeqCst) == 0;
                if was_first {
                    while !self.release.load(Ordering::SeqCst) {
                        tokio::task::yield_now().await;
                    }
                }
                FireOutcome::success(0, 0)
            }
        }

        let sm = make_schema_manager().await;
        register_view(
            &sm,
            "V1",
            "S1",
            vec![Trigger::OnWrite {
                schemas: vec!["S1".into()],
            }],
        );
        let clock = Arc::new(MockClock::new(0));
        let handler = Arc::new(GatedHandler {
            release: Arc::new(AtomicBool::new(false)),
            count: AtomicU32::new(0),
        });
        let writer = CountingFiringWriter::new();

        let runner = make_runner(
            Arc::clone(&sm),
            clock,
            Arc::clone(&handler) as Arc<dyn FireHandler>,
            Arc::clone(&writer) as Arc<dyn FiringWriter>,
        );

        // OnWrite uses inline dispatch — so spawn the first mutation in
        // a task (it blocks on the gate) and issue the second mutation
        // from the main task while the first fire is held.
        let r1 = Arc::clone(&runner);
        let first = tokio::spawn(async move {
            r1.on_mutation_notified("S1").await.unwrap();
        });

        // Wait until fire is in flight.
        for _ in 0..500 {
            if handler.count.load(Ordering::SeqCst) >= 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(handler.count.load(Ordering::SeqCst), 1);

        // Second mutation: dispatch_in_flight is held by the first, so
        // this call just sets the refire flag and returns immediately.
        runner.on_mutation_notified("S1").await.unwrap();

        // Release the gate — first fire completes, refire flag produces
        // exactly one additional inline fire.
        handler.release.store(true, Ordering::SeqCst);
        first.await.unwrap();

        assert_eq!(
            handler.count.load(Ordering::SeqCst),
            2,
            "refire flag should produce exactly one additional fire"
        );
    }

    #[test]
    fn exp_backoff_caps_at_60s() {
        assert_eq!(exp_backoff_ms(0), 1_000);
        assert_eq!(exp_backoff_ms(1), 1_000);
        assert_eq!(exp_backoff_ms(2), 2_000);
        assert_eq!(exp_backoff_ms(3), 4_000);
        assert_eq!(exp_backoff_ms(4), 8_000);
        assert_eq!(exp_backoff_ms(5), 16_000);
        assert_eq!(exp_backoff_ms(6), 32_000);
        assert_eq!(exp_backoff_ms(7), BACKOFF_MAX_MS);
        assert_eq!(exp_backoff_ms(100), BACKOFF_MAX_MS);
    }

    // Helper: wait up to `max_ticks * 2ms` real wall time for `predicate`
    // to become true. Uses `tokio::time::sleep` (not yield_now) because
    // the retry path spawns a task onto another worker thread, and bare
    // yields don't give the other worker a chance to make progress.
    // Keeps the retry tests tight (<<2s) while still deterministic via
    // MockClock for the trigger-internal logic.
    async fn wait_for(mut predicate: impl FnMut() -> bool, max_ticks: usize) -> bool {
        for _ in 0..max_ticks {
            if predicate() {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        }
        predicate()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn at_least_once_single_write_failure_retries_and_advances() {
        // INVARIANT UNDER TEST (see module docstring, trigger_runner.rs:25):
        // if the TriggerFiring audit-row write fails, the runner MUST NOT
        // advance `last_fire_ms`. A retry happens via the spawned retry
        // task that `dispatch_inline_once` schedules (1s backoff at
        // fail_streak=0), and on a successful retry `last_fire_ms` moves.
        //
        // Why this matters: `last_fire_ms` is the cursor used by scheduled
        // triggers to compute the next fire. Advancing it on a failed
        // audit write would lose one firing from the TriggerFiring log.
        let sm = make_schema_manager().await;
        register_view(
            &sm,
            "V1",
            "S1",
            vec![Trigger::OnWrite {
                schemas: vec!["S1".into()],
            }],
        );
        let clock = Arc::new(MockClock::new(1_000));
        let fire = RecordingFireHandler::all_success();
        let writer = CountingFiringWriter::new();
        // One write failure on the first attempt; the spawned retry
        // (after exp_backoff_ms(0) = 1000ms) will see the writer healthy.
        writer.set_fail_count(1);

        let runner = make_runner(
            Arc::clone(&sm),
            Arc::clone(&clock),
            Arc::clone(&fire) as Arc<dyn FireHandler>,
            Arc::clone(&writer) as Arc<dyn FiringWriter>,
        );

        runner.on_mutation_notified("S1").await.unwrap();

        // Sync attempt: fire handler is called once, writer is called
        // once and fails. `dispatch_inline_once` then spawns a retry task
        // that parks on clock.sleep(1000).
        let saw_sync_attempt = wait_for(
            || {
                writer.call_count.load(Ordering::SeqCst) >= 1
                    && fire.call_count.load(Ordering::SeqCst) >= 1
            },
            200,
        )
        .await;
        assert!(
            saw_sync_attempt,
            "sync fire + write attempt should complete"
        );

        // Park until the retry task has registered its sleeper (~1000ms).
        let saw_parked = wait_for(|| clock.pending_sleeps() >= 1, 200).await;
        assert!(saw_parked, "retry task should park on clock.sleep(1000)");

        // The sync-attempt write failed → last_fire_ms must still be 0.
        assert_eq!(
            runner.test_last_fire_ms("V1").await,
            0,
            "last_fire_ms must NOT advance on audit-write failure"
        );
        // Write failure is NOT a fire failure: fail_streak stays at 0, so
        // quarantine is not triggered by audit-write errors.
        assert_eq!(runner.test_fail_streak("V1").await, 0);
        assert!(!runner.test_is_quarantined("V1").await);
        assert!(
            writer.rows.lock().await.is_empty(),
            "no audit rows should have landed yet"
        );

        // Advance the mock clock past the backoff so the retry task runs.
        clock.advance(1_000);

        // Retry: fire succeeds, writer now succeeds, last_fire_ms advances.
        let saw_retry = wait_for(|| writer.call_count.load(Ordering::SeqCst) >= 2, 500).await;
        assert!(saw_retry, "retry attempt should run after 1s backoff");

        // last_fire_ms is updated under the view's persisted Mutex — wait
        // until we observe the advance. Uses real-time sleep in wait_for.
        let mut advanced = false;
        for _ in 0..200 {
            if runner.test_last_fire_ms("V1").await > 0 {
                advanced = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        }
        assert!(advanced, "last_fire_ms must advance after successful retry");

        assert_eq!(
            writer.call_count.load(Ordering::SeqCst),
            2,
            "expected 1 failed + 1 successful write attempt"
        );
        let rows = writer.rows.lock().await.clone();
        assert_eq!(rows.len(), 1, "exactly one audit row should land");
        assert_eq!(rows[0].view_name, "V1");
        assert_eq!(rows[0].status, FiringStatus::Success);
        assert!(!runner.test_is_quarantined("V1").await);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn at_least_once_multi_write_failure_does_not_quarantine() {
        // EXPECTED BEHAVIOR (and documents the quarantine boundary):
        //
        // TriggerFiring audit-write failures are SIDE EFFECTS of
        // successful fires — they must NOT count toward the 3-strikes
        // quarantine budget. Only `FireHandler` failures (the fire
        // itself going wrong) increment `fail_streak`. Otherwise a
        // transient schema_service outage could quarantine every view
        // in the registry, which would be a disaster.
        //
        // Runner code path that proves this (trigger_runner.rs:582-604,
        // 656-683): `fail_streak` is incremented only inside the
        // `!outcome.success` branch. The write_result is handled *after*
        // that decision and only gates `last_fire_ms` / the warn-log.
        //
        // This test exercises that contract under 4 consecutive write
        // failures (note 3 is the quarantine threshold for FIRE failures
        // per QUARANTINE_FAIL_STREAK): fire succeeds every time, writes
        // fail 4 times, recover on the 5th attempt. Neither quarantine
        // nor fail_streak advancement should occur.
        //
        // Driving the retries: each external mutation drives a
        // sync-attempt + 1 spawned retry (the inline dispatch path
        // doesn't loop on write failure — only on fire failure). So to
        // get 5 total attempts with writes 1..=4 failing we issue 3
        // mutations and advance the clock past each 1s backoff. The
        // 3rd mutation's spawned retry sees a healthy writer.
        let sm = make_schema_manager().await;
        register_view(
            &sm,
            "V1",
            "S1",
            vec![Trigger::OnWrite {
                schemas: vec!["S1".into()],
            }],
        );
        let clock = Arc::new(MockClock::new(1_000));
        let fire = RecordingFireHandler::all_success();
        let writer = CountingFiringWriter::new();
        writer.set_fail_count(4);

        let runner = make_runner(
            Arc::clone(&sm),
            Arc::clone(&clock),
            Arc::clone(&fire) as Arc<dyn FireHandler>,
            Arc::clone(&writer) as Arc<dyn FiringWriter>,
        );

        // Mutation 1: attempts 1 (sync, fail) + 2 (spawned, fail).
        runner.on_mutation_notified("S1").await.unwrap();
        // Wait for sync attempt.
        assert!(
            wait_for(|| writer.call_count.load(Ordering::SeqCst) >= 1, 200).await,
            "mutation 1 sync attempt should run"
        );
        // Wait for spawned retry to park.
        assert!(
            wait_for(|| clock.pending_sleeps() >= 1, 200).await,
            "mutation 1 retry task should park on backoff"
        );
        clock.advance(1_000);
        assert!(
            wait_for(|| writer.call_count.load(Ordering::SeqCst) >= 2, 500).await,
            "mutation 1 spawned retry should run"
        );
        // Both attempts failed writes; no quarantine, no advancement.
        assert_eq!(runner.test_last_fire_ms("V1").await, 0);
        assert_eq!(
            runner.test_fail_streak("V1").await,
            0,
            "write failures must not increment fail_streak"
        );
        assert!(!runner.test_is_quarantined("V1").await);

        // Wait for the spawned retry task to release dispatch_in_flight.
        // (The release happens at the end of the tokio::spawn closure,
        // after run_fire_with_refire_loop returns.) Short real-time sleep
        // is needed because the release runs on a different worker.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // Mutation 2: attempts 3 (sync, fail) + 4 (spawned, fail).
        runner.on_mutation_notified("S1").await.unwrap();
        assert!(
            wait_for(|| writer.call_count.load(Ordering::SeqCst) >= 3, 500).await,
            "mutation 2 sync attempt should run"
        );
        assert!(
            wait_for(|| clock.pending_sleeps() >= 1, 200).await,
            "mutation 2 retry task should park on backoff"
        );
        clock.advance(1_000);
        assert!(
            wait_for(|| writer.call_count.load(Ordering::SeqCst) >= 4, 500).await,
            "mutation 2 spawned retry should run"
        );
        // 4 total write failures now — past QUARANTINE_FAIL_STREAK (3).
        // The view MUST still not be quarantined.
        assert_eq!(runner.test_last_fire_ms("V1").await, 0);
        assert_eq!(runner.test_fail_streak("V1").await, 0);
        assert!(
            !runner.test_is_quarantined("V1").await,
            "4 consecutive audit-write failures must not quarantine — \
             quarantine budget is for FIRE failures only"
        );

        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        // Mutation 3: attempt 5 (sync). fail_count is now 0 → write
        // succeeds on the first (sync) try, last_fire_ms advances, no
        // spawned retry is queued.
        runner.on_mutation_notified("S1").await.unwrap();
        assert!(
            wait_for(
                || {
                    writer.call_count.load(Ordering::SeqCst) >= 5
                        && !writer.rows.try_lock().map(|r| r.is_empty()).unwrap_or(true)
                },
                500
            )
            .await,
            "mutation 3 sync write should succeed"
        );

        assert_eq!(
            writer.call_count.load(Ordering::SeqCst),
            5,
            "expected 4 failed + 1 successful write attempts"
        );
        let rows = writer.rows.lock().await.clone();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, FiringStatus::Success);
        assert!(runner.test_last_fire_ms("V1").await > 0);
        assert_eq!(runner.test_fail_streak("V1").await, 0);
        assert!(!runner.test_is_quarantined("V1").await);
    }
}
