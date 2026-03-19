# View Chain Performance: Bounds, Locks, and Mitigations

## Problem Statement

Views can form arbitrarily deep chains (ViewC → ViewB → ViewA → Schema) and
fan-out graphs (ViewD depends on ViewA, ViewB, ViewC). Two operations walk
these graphs at runtime:

1. **Query resolution** — recursive descent to compute a view's output
2. **Cascade invalidation** — upward propagation when a source is mutated

Both have unbounded depth/width today. This doc analyzes the theoretical
bounds, practical risks, and proposed mitigations.

---

## Current Architecture

```
 QUERY PATH (top-down)                INVALIDATION PATH (bottom-up)

 Query("ViewC", ["f"])                Mutate("Schema", {"f": "new"})
       │                                     │
       ▼                                     ▼
 ┌─ QueryExecutor ──────────┐      ┌─ MutationManager ────────────┐
 │ schema lookup: miss      │      │ write to storage             │
 │ view lookup: ViewC       │      │                              │
 │ load cache: Empty        │      │ invalidate_dependent_caches  │
 │                          │      │   lock registry (short)      │
 │ ViewResolver::resolve()  │      │   collect dependents         │
 │   input_query → "ViewB"  │      │   unlock                    │
 │                          │      │                              │
 │   RecursiveSourceQuery   │      │   for each dependent:        │
 │     schema lookup: miss  │      │     set_cache(Empty) [async] │
 │     view lookup: ViewB   │      │     flush() [async]          │
 │     load cache: Empty    │      │                              │
 │                          │      │     cascade_invalidate()     │
 │     ViewResolver again   │      │       lock registry (short)  │
 │       input → "ViewA"    │      │       collect dependents     │
 │       ... recurse ...    │      │       unlock                 │
 │       input → "Schema"   │      │       recurse...             │
 │       ──▶ direct DB read │      │                              │
 └──────────────────────────┘      └──────────────────────────────┘
```

---

## Theoretical Bounds

### Query Resolution

| Metric | Bound | Notes |
|--------|-------|-------|
| Chain depth | O(V) where V = total views | No depth limit; cycles prevented at registration |
| Queries per resolution | O(V × Q) where Q = max input queries per view | Each view level executes its input queries |
| DB reads | O(V × Q × F) where F = fields per query | Leaf schemas hit storage |
| Lock acquisitions | O(V) | Registry mutex acquired once per view level |
| Lock duration | O(1) per acquisition | Copy view definition, release immediately |
| Cache writes | O(V) | One `set_view_cache_state` + flush per level on cache miss |

**Worst case:** A 100-view chain with no cached intermediates triggers 100
recursive resolutions, 100 registry lock acquisitions, 100 cache flushes,
and at least 100 DB reads. Each flush is a Sled fsync or DynamoDB PutItem.

### Cascade Invalidation

| Metric | Bound | Notes |
|--------|-------|-------|
| Views visited | O(V) | `visited` HashSet prevents re-processing |
| Lock acquisitions | O(V) | Registry mutex re-acquired at each cascade step |
| Cache writes | O(V) | One `set_view_cache_state(Empty)` + flush per view |
| Total flushes | O(V) | **No batching** — each invalidation flushes independently |

**Worst case:** A schema with 1000 transitive dependents triggers 1000
lock-acquire-release cycles, 1000 cache state writes, and 1000 flushes.

---

## Lock Analysis

### Will it deadlock?

**No.** The locking discipline is consistent:

1. Registry mutex is always acquired for short, bounded operations (copy data out)
2. Registry mutex is **never held across `.await` points**
3. No nested lock acquisition (registry lock is dropped before any async work)
4. `visited` HashSet in cascade prevents infinite loops

### Will it block other operations?

**Yes, briefly.** During cascade invalidation of N views:
- The registry mutex is acquired N times (once per cascade step)
- Each acquisition is O(1) (just reading the dependency tracker HashMap)
- Other queries/mutations will block momentarily while waiting for the mutex
- **Not a sustained lock** — it's N short acquisitions, not one long hold

### The real bottleneck: flushes, not locks

Each cache invalidation calls `set_view_cache_state` which calls `flush()`.
For Sled, this is an fsync. For DynamoDB, it's a PutItem API call. With N
dependent views, that's N sequential flushes — potentially seconds of I/O.

---

## Practical Risk Assessment

| Chain depth | Query latency | Invalidation latency | Risk level |
|-------------|--------------|----------------------|------------|
| 1-5 views | < 50ms | < 100ms | Low |
| 5-20 views | 50-500ms | 100ms-2s | Medium |
| 20-100 views | 500ms-5s | 2-10s | High |
| 100+ views | > 5s | > 10s | Critical |

The system is fine for typical use (2-5 view chains). It becomes problematic
at scale without the mitigations below.

---

## Proposed Mitigations

### 1. Depth limit on view chains (low effort, high value)

Add a `MAX_VIEW_CHAIN_DEPTH` constant (e.g., 16) enforced at registration
time. The cycle detection DFS already walks the graph — extend it to check
depth.

```
register_view("ViewN")
  └─ would_create_cycle() already does DFS
  └─ extend: if DFS depth > MAX_VIEW_CHAIN_DEPTH → reject

Error: "View chain depth would exceed limit of 16
        (ViewN → ViewM → ... → ViewA → Schema)"
```

**Where:** `src/view/dependency_tracker.rs` in `would_create_cycle()`

### 2. Batch cache invalidation (medium effort, high value)

Collect all views to invalidate first, then batch-write all cache states
and flush once.

```
Current:  for each view { set_cache(Empty); flush(); }  // N flushes
Proposed: for each view { set_cache(Empty); }; flush_once();  // 1 flush
```

**Where:** `src/fold_db_core/mutation_manager.rs` in
`invalidate_dependent_view_caches()` and `invalidate_cascading_view_caches()`

### 3. Single-pass cascade collection (low effort, medium value)

Currently the registry mutex is re-acquired at each cascade step. Instead,
collect all transitive dependents in one graph walk while holding the lock
once.

```
Current:
  for each direct dependent:
    lock(); get_dependents(); unlock();  // N lock acquisitions
    recurse...

Proposed:
  lock()
  walk entire DAG, collect all transitive dependents into Vec
  unlock()
  batch invalidate all                                // 1 lock acquisition
```

**Where:** Add `get_all_transitive_dependents()` to `DependencyTracker`,
call once from `invalidate_dependent_view_caches()`

### 4. Lazy cascade invalidation (medium effort, medium value)

Instead of eagerly invalidating the entire cascade on mutation, only
invalidate direct dependents. Transitive dependents discover staleness
on query — their source view's cache is Empty, so they recompute.

```
Current:  Mutate Schema → invalidate ViewA, ViewB, ViewC (eager, 3 writes)
Proposed: Mutate Schema → invalidate ViewA only (1 write)
          Query ViewC → ViewB cache hit → ViewA cache miss → recompute
          ViewC's cache is stale but that's ok — it recomputes from ViewA
```

**Trade-off:** Reads become slightly slower (one extra cache miss check per
level) but writes become O(direct dependents) instead of O(all transitive
dependents). Good if fan-out is large but queries are infrequent.

**Caveat:** Requires checking source freshness on cache hit, not just
"is my cache populated?" This changes the cache model from "Empty/Cached"
to "Empty/Cached(version)" where version tracks source mutation count.

---

## Recommendation

**Do #1 and #3 now** (depth limit + single-pass collection). They're
small changes that prevent pathological cases without changing the
architecture.

**Defer #2 and #4.** Batch flushing is a nice optimization but not urgent
until someone actually has 20+ view chains. Lazy cascade is an
architectural change that should wait until there's a demonstrated need.
