# Tracing egress classifier lint (Phase 2 / T4 + Phase 5 / T3)

Static guard that fails CI if a `reqwest::Client` / `reqwest::ClientBuilder`
construction in `crates/*/src/` is not annotated with a `// trace-egress: <class>`
classifier comment in the three lines immediately preceding it.

This pairs with the runtime W3C-traceparent injection wired through
`observability::propagation::inject_w3c`. The classifier comment forces a
deliberate decision at every construction site: does this client need to
propagate `traceparent`, or is propagation deliberately skipped (presigned
S3, third-party API)? The lint enforces that the decision is recorded.

For the per-class semantics (when to wrap, when to skip) and the running
list of in-tree sites, see
[`egress-classification-notes.md`](./egress-classification-notes.md).

## Why it matters

`reqwest::Client::new()` constructs a fresh HTTP client whose requests
carry no trace context unless the caller explicitly attaches one. Without
the classifier we have to re-derive intent every time a reviewer reads
the call site:

- Is this a call to one of our own services, where downstream handlers
  expect `traceparent` so cross-service spans stitch back into a single
  trace? (`propagate` / `loopback`)
- Is this a presigned-URL S3 upload where attaching headers would break
  the SigV4 canonical request and invalidate the signature? (`skip-s3`)
- Is this a third-party that doesn't honour `traceparent` and would just
  log the unknown header? (`skip-3p`)

The classifier comment encodes the answer once at construction. Reviewers
reading a diff can verify wrapping vs. non-wrapping without re-reasoning
about the destination. Phase 2 / T4 (PR #636) seeded the lint and
classified the in-tree sites; Phase 5 / T3 turns it into a hard CI gate.

## The pattern the lint enforces

The lint walks every `reqwest::(Client|ClientBuilder)::(new|default|builder)()`
match in `crates/*/src/`. For each one, it scans the three lines
immediately preceding the call for a `// trace-egress:` comment. If
none is present, the site is unclassified and (in `--strict` mode)
fails the build.

### Canonical good shapes

Inline classifier on the line before:

```rust
// trace-egress: propagate — auth Lambda, .post() wraps with inject_w3c.
let http = Arc::new(reqwest::Client::new());
```

Two-line block when the rationale is non-obvious (e.g. shared client):

```rust
// trace-egress: propagate — also handed to S3Client (skip-s3); active
// class is propagate because AuthClient::post wraps with inject_w3c.
let http = Arc::new(reqwest::Client::new());
```

The classifier may sit anywhere in the three preceding lines. `rustfmt`
does not touch comments, so the relationship is stable across reformat.

## Classes

| Class       | When to use                                                              | Wrap with `inject_w3c`? |
|-------------|--------------------------------------------------------------------------|-------------------------|
| `propagate` | Call goes to one of our own services (auth Lambda, fold_db_node, etc.).  | Yes                     |
| `loopback`  | Same as `propagate` but for localhost loopback / `#[cfg(test)]` fakes.   | Yes (when it emits)     |
| `skip-s3`   | Presigned-URL S3 calls. Adding headers breaks the SigV4 signature.       | No                      |
| `skip-3p`   | Third-party APIs (Stripe, OpenRouter, etc.) that ignore `traceparent`.   | No                      |

The classifier names a contract; it does not (today) drive any code
generation. `inject_w3c` wrapping happens by hand in the helper that
issues the request — typically a `post()` / `get()` method on a typed
client struct, so adding a new endpoint method doesn't require
remembering to wrap.

## Running locally

```sh
# Warn-only (default): print unclassified sites, exit 0.
bash scripts/lint-tracing-egress.sh

# Strict: same scan, exit 1 on any unclassified site.
bash scripts/lint-tracing-egress.sh --strict
```

The default mode is warn-only so a half-finished local edit doesn't
block iteration. CI runs with `--strict` so unclassified constructions
cannot land on `main`.

The CI step `Lint tracing egress classifiers` inside the `Rust Tests`
job (`.github/workflows/ci-tests.yml`) invokes `--strict` on every PR
and `push` to `main`.

## Scope and limits

- `crates/*/src/` only. Top-level integration tests under
  `crates/*/tests/` are out of scope, mirroring `lint-spawn-instrument.sh`
  and `lint-redaction.sh`. Classification matters at runtime, not in
  scaffolding outside `src/`.
- The lint matches on a three-line preceding window. If `rustfmt` ever
  splits a `let http = ...;` across more than three lines (it does not
  today), revisit the window.
- A site that constructs a single `Arc<reqwest::Client>` shared between
  two consumer types (e.g. `AuthClient` *and* `S3Client`) is classified
  by its *active* class (the one with required wrapping). See
  `egress-classification-notes.md` for the in-tree case.
- Sibling repos (`fold_db_node`, `schema_service`, `exemem-infra`) each
  ship their own copy of the same lint as a Cohort C follow-up.
