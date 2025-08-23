# AI Orchestrated Query Loop (AOQL)

Instead of pre-authoring a composable plan, the AI iteratively issues the next best query until a goal condition is met. This approach reduces cognitive load for users while maintaining strict guardrails.

## Architecture

```
Goal (required) ──► Loop Controller (AI) ──► Next Query Proposal
                           ▲                         │
                           │                         ▼
               QueryResultStore ◄── Executor ◄── Validation/Policy
```

- **Goal**: Declarative condition to satisfy (e.g., "return user prefs and scores for all Eng users; join on user_id")
- **Loop Controller (AI)**: Proposes the next QueryStep based on current results + goal state
- **Validator/Executor**: Reuses existing Query type, permission checks, and limits
- **QueryResultStore**: Holds step outputs (already implemented)
- **Goal Evaluator**: Deterministic function that says "done / not done / impossible"

## New Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopGoal {
    pub description: String,           // human-readable goal
    pub success_predicate: GoalSpec,   // deterministic spec
    pub max_steps: u32,                // hard cap (e.g., 10)
    pub max_cost_units: u64,           // e.g., row*field units or $ budget
    pub deadline_ms: u64,              // time cap for the loop
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalSpec {
    // Example: requires fields across schemas for a set of user_ids
    HaveRows {
        schema_name: String,
        required_fields: Vec<String>,
        min_rows: usize,
        join_key: Option<String>,      // if success depends on merging with another result
    },
    // Add others as needed (e.g., NonEmpty, CountAtLeast, FieldCoverage, PredicateExpr)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopState {
    pub step: u32,
    pub results: QueryResultStore,
    pub spent_cost_units: u64,
    pub trace: Vec<LoopTraceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopTraceItem {
    pub id: String,
    pub query: Query,                  // redacted where necessary
    pub rows: usize,
    pub ms: u64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopOutcome {
    Success { final_result: QueryResult, state: LoopState },
    Exhausted { reason: String, state: LoopState },      // max_steps, max_cost, deadline
    Blocked { error: String, state: LoopState },         // permission, validation
}
```

## Loop Algorithm

```rust
// Deterministic wrapper around the AI
init LoopState(step=0, spent=0)
validate permissions for all schemas the AI is allowed to touch (whitelist)

while step < max_steps and spent < max_cost and now < deadline:
  if goal_success(LoopState, GoalSpec): break (Success)
  proposal = AI.next_step(Goal, LoopState)  // proposes a Query
  validate(proposal):                       // schema exists, fields exist, no forward refs, trust_distance ok
    if fail => Blocked
  estimate_cost(proposal) and check against remaining budget
  result = execute(proposal)                // existing executor
  record(result) into LoopState.results
  update spent_cost_units with measured rows, bytes, time
  step++

if goal_success: Outcome=Success(final_result=materialize_from LoopState)
else Outcome=Exhausted(reason)
```

## Guardrails (Non-Negotiable)

- **Whitelisted schemas only** for AOQL
- **Hard budgets**: `max_steps`, `max_cost_units`, `deadline_ms`, fanout caps
- **Reference rules**: Phase-1 only allows referencing top-level fields in previous step objects
- **No privilege amplification**: intersect permissions; `trust_distance = max`
- **Deterministic success predicate**: never let the AI declare success; the engine does

## API Surface

```rust
// POST /loop/execute
{
  "goal": {
    "description": "prefs+scores for all Eng users",
    "success_predicate": {
      "HaveRows": {
        "schema_name": "UserPreferences",
        "required_fields": ["user_id","theme","language","total_score","rank"],
        "min_rows": 1,
        "join_key": "user_id"
      }
    },
    "max_steps": 6,
    "max_cost_units": 5_000_000,
    "deadline_ms": 10000
  },
  "options": { "debug": true }
}
```

Response: `LoopOutcome` with `execution_trace`.

## AI Policy (Simple Strategy)

1. **If target rows need keys**: Issue an index/source query to get those keys (e.g., Department → user_ids)
2. **If target fields missing**: Issue a fanout query using obtained keys (e.g., prefs for user_ids)
3. **If join fields missing**: Issue complementary query (e.g., scores), then ask engine to merge
4. **If fanout size > threshold**: Switch to `fanout.mode="In"` or batch with estimated coverage
5. **If validation fails**: Choose an alternate schema/field or ask for human confirmation

## Example Run: Eng → Prefs → Scores → Merge

**Goal**: `HaveRows` on "UserPreferences" with fields `["user_id","theme","language","total_score","rank"]`, `join_key="user_id"`.

**Step 1** (AI proposes):
```json
{"schema_name": "Department", "fields": ["user_ids"], "filter": {"department_name":"Engineering"}}
```
Result: `{"user_ids":["alice","bob","charlie"]}`

**Step 2**:
```json
{"schema_name": "UserPreferences", "fields": ["theme","language"], "filter": {"user_id": ["alice","bob","charlie"]}}
```
Result: 3 rows with theme/language (+ user_id)

**Step 3**:
```json
{"schema_name": "UserScores", "fields": ["total_score","rank"], "filter": {"user_id": ["alice","bob","charlie"]}}
```
Result: 3 rows with total_score/rank (+ user_id)

**Step 4** (engine, not AI):
`ParallelAggregation::Merge { join_key:"user_id", prefer:"Left" }` or simple deterministic merge function.
Goal satisfied → Success, return merged rows + execution_trace.

## Pros vs. Preplanned Composable

**Pros**:
- Lower cognitive load for users; ask in plain English
- Adapts to partial/unknown schema coverage
- Can react to data (e.g., small fanout → Each, huge fanout → In)

**Cons**:
- More steps = higher cost/latency
- Harder to reproduce if AI varies too much (use deterministic prompting, temperature=0)
- Needs strong budgets and schema whitelists

## Implementation Order (1 Sprint)

1. Add `LoopGoal`, `LoopState`, `LoopOutcome`, `GoalSpec::HaveRows`
2. Implement `goal_success()` and budgets
3. Build `/loop/execute` that repeatedly calls AI for `next_step`, then existing validator/executor
4. Whitelist schemas, enforce trust-distance, add rate limiting by loop weight (steps + fanout)
5. Log execution_trace and redacted filter hashes

**Note**: AOQL reuses existing `Query`, `QueryResultStore`, validation, permissions, and fanout rules.

## Conclusion

The AI Orchestrated Query Loop provides an alternative approach for users who prefer declarative goals over explicit query composition. It reduces cognitive load while maintaining strict security guardrails and reusing the existing query infrastructure.