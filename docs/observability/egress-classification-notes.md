# Egress classification notes

Cross-Phase 2 notes on `// trace-egress: <class>` classifier comments and
`observability::propagation::inject_w3c` wrapping. One file per repo where
ambiguous or awkward sites land. Future Phase 2 sweeps in sibling repos
should append.

Classes:

- `propagate` — call goes to one of our own services. Wrap eventual
  `.send()` callers with `observability::propagation::inject_w3c(builder)`.
- `loopback` — same as propagate but for internal localhost loopback or
  test scaffolds. Wrap if the call actually emits.
- `skip-s3` — presigned-URL S3 calls. DO NOT wrap; injecting headers
  breaks the SigV4 signature.
- `skip-3p` — third-party (Stripe, OpenRouter, etc.) that doesn't honour
  traceparent. DO NOT wrap.

## fold_db (this repo) — Phase 2 / T4 sweep

### Shared `Arc<reqwest::Client>` between `propagate` (auth Lambda) and `skip-s3` (presigned URLs)

Two production sites construct a single `Arc<reqwest::Client>` that flows
into BOTH consumer types:

- `crates/core/src/fold_db_core/factory.rs:157` (`build_database`)
- `crates/core/src/fold_db_core/fold_db.rs:243` (`enable_sync_with_setup`)

Each construction is `let http = Arc::new(reqwest::Client::new())` and is
immediately handed to:

1. `crate::sync::auth::AuthClient::new(http.clone(), ...)` — talks to our
   auth Lambda via `POST {base_url}{path}`. Classified `propagate`.
2. `crate::sync::s3::S3Client::new(http.clone())` — talks to S3 via
   presigned URLs (PUT/GET/DELETE on `presigned.url`). Classified `skip-s3`.

We classify the **construction** as `propagate` (the active class — it is
the one with required wrapping). Per-call wrapping happens in
`AuthClient::post` (`crates/core/src/sync/auth.rs`) via `inject_w3c`.
`S3Client::{upload,download,delete}` deliberately do NOT call `inject_w3c` —
attaching a `traceparent` header would change the canonical request and
invalidate the presigned URL signature.

If this gets confusing in future, the cleanup is to split into two distinct
`Arc<reqwest::Client>` — one classified `propagate` for `AuthClient`, one
classified `skip-s3` for `S3Client`. Today they share a single connection
pool, which is desirable; the structural split would only be cosmetic.

### Test scaffolding inside `src/`

Three `#[cfg(test)] mod tests` blocks contain `reqwest::Client::new()` calls
inside `src/` files (so the lint script catches them):

- `crates/core/src/storage/syncing_store.rs:122`
- `crates/core/src/storage/syncing_namespaced_store.rs:86`
- `crates/core/src/sync/engine.rs:2664`

These build `AuthClient`/`S3Client` for tests pointed at unreachable
localhost addresses (`http://localhost:0`, `http://127.0.0.1:1`) and never
emit real egress at runtime. They carry `// trace-egress: loopback`
classifiers — a documentation-only tag — to keep the lint passable without
teaching it to grep-skip `#[cfg(test)]` blocks.

The plan's note (`projects/observability-phase-2-propagation`) says egress
classification "matters at runtime, not in test scaffolding"; the lint
chooses simplicity over cleverness here.

## Cross-repo consistency

When Phase 2 sweeps run in `fold_db_node`, `schema_service`, and
`exemem-infra`, follow the same conventions:

- One classifier comment per construction, on one of the 3 lines
  immediately preceding `reqwest::(Client|ClientBuilder)::(new|default|builder)()`.
- For `propagate` / `loopback` clients with multiple call sites, prefer
  wrapping inside the lowest-level helper that builds the request (e.g.
  the `post()`/`get()` helper on a typed client struct), so adding a new
  endpoint method doesn't require remembering to wrap.
- For shared-client awkwardness (like the `propagate` + `skip-s3` case
  here), document it in this file under a per-repo heading rather than
  splitting clients prematurely.
