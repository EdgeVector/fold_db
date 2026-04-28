# Sampling config notes

Phase 4 / T5 implementation notes for `OBS_SAMPLER` env-driven head
sampling and the Honeycomb setup doc. Captures decisions, what was
intentionally left out, and the follow-ups that fall out.

## What landed

- `crates/observability/src/sampling.rs`
  - `parse_sampler() -> Result<Sampler, SamplerParseError>` reads
    `OBS_SAMPLER` from the env.
  - `parse_sampler_spec(&str)` parses one spec string in OTel-spec
    syntax. Public so tests don't have to mutate the real environment.
  - Recognized variants: `always_on`, `always_off`,
    `traceidratio:<f>`, `parentbased_traceidratio:<f>`.
  - Default when env unset: `parentbased_traceidratio:1.0` (100%,
    parent-honouring — dev-safe, must be tuned down for prod).
  - Errors are structured: `InvalidRatio`, `MissingRatio`,
    `UnknownSampler`. The error string includes the bad value so the
    boot log is self-explanatory.
- `crates/observability/src/init.rs::init_node`
  - Sampler is constructed via `parse_sampler()` and applied with
    `SdkTracerProvider::builder().with_sampler(sampler)` *before* the
    OTLP exporter is wired (Phase 4 / T7). Today the exporter is still
    a no-op, but the sampler decision already gates `is_recording()`
    for downstream layers, so observable behaviour is consistent from
    the moment the exporter lands.
- `docs/observability/honeycomb-setup.md`
  - How to create a Honeycomb env + ingest key.
  - Where to set `OBS_OTLP_ENDPOINT` / `OBS_OTLP_HEADERS` (those env
    vars are not yet read by code — that's Phase 4 / T7 — but the doc
    fixes the names so we don't bikeshed them later).
  - Recommended `OBS_SAMPLER` per env: dev=100%, staging=50%, prod=10%.
  - Cost projection table tied to a back-of-envelope ~50 spans per
    fold_db request.
- `docs/observability/README.md` — index across the existing
  observability notes; was missing.

## Decisions worth flagging

1. **Reject bad ratios at boot rather than clamping.** The OTel SDK
   silently clamps `traceidratio:1.5` to `1.0`. We don't — the node
   refuses to start. Reason: silent clamping would mask operator typos
   that move sampling rates by 10x. Better to fail loudly.
2. **Default is `parentbased_traceidratio:1.0`, not `always_on`.** Both
   keep 100% of traces in dev. The parent-based form additionally
   honours an upstream "drop" decision, which matters the moment a
   distributed trace from a sampling-aware caller hits a fold_db node.
   `always_on` would override that and produce half-broken trees.
3. **Used `Builder::with_sampler(...)` directly.** The task brief
   suggested `Config::default().with_sampler(...)`, but `Config` is
   `#[deprecated(since = "0.27.1")]` in `opentelemetry_sdk` ≥ 0.27.
   The two paths are wire-compatible; the non-deprecated one keeps
   future SDK upgrades quieter.
4. **Sampler errors surface as `ObsError::SubscriberInstall`.** No new
   error variant — a malformed `OBS_SAMPLER` is still an init-time
   failure from the caller's perspective, and the existing variant
   already serializes a string. Worth revisiting if a downstream caller
   needs to programmatically distinguish "sampler malformed" from
   "subscriber install failed".
5. **Behavioural test probes, not pattern matches.** `Sampler` is a
   non-`PartialEq` enum that owns a `Box<dyn ShouldSample>` for the
   `ParentBased` variant. Test assertions probe the sampler's
   `should_sample` output for known trace ids instead of matching on
   variants — that pins the *behaviour* we care about and survives
   SDK version bumps.

## Out of scope (intentional)

- **Provisioning the Honeycomb account** — manual ops step described in
  the setup doc.
- **OTLP exporter wiring** — Phase 4 / T7. Until then the sampler runs
  on a no-op `TracerProvider`; spans are decided-on but not exported.
- **Tail sampling Collector deployment** — deferred until prod throughput
  justifies the Collector's operational cost.
- **`OTEL_TRACES_SAMPLER` / `OTEL_TRACES_SAMPLER_ARG` compat** — we read
  our own `OBS_SAMPLER` only. The OTel-standard env vars are an obvious
  add when there's demand from a sibling crate that already reads them.
- **Moving `honeycomb-setup.md` to `exemem-workspace/docs/observability/`**
  — task brief explicitly defers this. The doc was created in
  `fold_db/docs/observability/` for now; a follow-up task moves it.

## Follow-ups to file

- Move `docs/observability/honeycomb-setup.md` into
  `exemem-workspace/docs/observability/` once that repo has a docs tree
  for it. Leave a redirect stub here.
- Wire `OBS_OTLP_ENDPOINT` + `OBS_OTLP_HEADERS` to a real OTLP/HTTP
  exporter on the `TracerProvider` (Phase 4 / T7). The setup doc
  already names these vars.
- Phase 4 / T6 (LLM tracing) is expected to roughly double per-request
  span count. When that lands, re-derive the cost-projection table in
  `honeycomb-setup.md`.
- Add a workspace-level boot test that flips `OBS_SAMPLER=garbage` and
  asserts the node refuses to start. The unit tests cover the parser
  in isolation; an integration test would catch a regression in the
  init-time wiring.
- Decide whether to also accept the upstream
  `OTEL_TRACES_SAMPLER` / `OTEL_TRACES_SAMPLER_ARG` env vars so OTel
  Collector sidecars can configure us with their existing playbooks.
