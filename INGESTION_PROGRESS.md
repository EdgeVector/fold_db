# Ingestion Progress

## What it does

`src/ingestion/progress.rs` is the **progress-tracking adapter for ingestion operations**. It wraps the generic `JobTracker` (which stores any job type) and presents an ingestion-specific view — mapping between the generic `Job`/`JobStatus` vocabulary and the domain-specific `IngestionStep`/`IngestionProgress` vocabulary.

## Components

| Type | Role |
|---|---|
| `IngestionStep` | 8-variant enum naming the stages of an ingestion pipeline |
| `IngestionProgress` | Flattened read-model: `current_step`, `is_complete`, `is_failed`, `results`, timestamps |
| `IngestionResults` | Written to the job on completion: schema name, counts, all keys that were stored |
| `ProgressService` | Wraps `ProgressTracker` with methods for each lifecycle transition |
| `From<Job> for IngestionProgress` | Converts a generic `Job` into the ingestion read-model |

## ProgressService operations

| Method | What it does |
|---|---|
| `start_progress(id, user_id)` | Creates a new Ingestion job at 5% / `ValidatingConfig` |
| `update_progress(id, step, msg)` | Advances to a step; percentage is computed from the step |
| `update_progress_with_percentage(id, step, msg, pct)` | Same but with an explicit percentage override |
| `complete_progress(id, results)` | Marks job Completed and stores `IngestionResults` |
| `fail_progress(id, error)` | Marks job Failed with an error string |
| `get_progress(id)` | Returns the current state of a single job |
| `get_all_progress()` | Returns all Ingestion / Indexing / database_reset jobs for the current user |

## Step → percentage mapping

```
ValidatingConfig(5%) → FlatteningData(25%) → GettingAIRecommendation(40%)
  → SettingUpSchema(55%) → GeneratingMutations(75%) → ExecutingMutations(90%)
  → Completed(100%) / Failed(100%)
```

## What the original had that the rewrite removes

| Original pattern | Count | Why removed |
|---|---|---|
| 7-line "update metadata with step" block | ×4 | Extracted to `set_job_step(job, step)` helper |
| 3-line "save and warn on error" block | ×5 | Extracted to `async fn save_job(&self, job)` |
| `update_progress` duplicated the body of `update_progress_with_percentage` | ×1 | Now delegates with `step_to_percentage` |
| `job = job.with_user(user_id)` immediate reassignment in `start_progress` | ×1 | Chained in constructor call |
| `if let Ok(Some(mut job)) = ... { ... } else { None }` | ×4 | Replaced with `let...else { return None }` |
