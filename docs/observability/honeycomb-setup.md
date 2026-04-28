# Honeycomb dev setup

How to point a fold_db node at a Honeycomb environment for trace ingest, and
how to pick a sampling rate that fits the environment's budget.

> **Status.** Phase 4 / T5. The `OBS_SAMPLER` env var is plumbed through
> `init_node` today (controls head-sampling decisions on the local
> `TracerProvider`). The OTLP traces *exporter* (`OBS_OTLP_ENDPOINT`,
> `OBS_OTLP_HEADERS`) is wired by Phase 4 / T7 — until that lands, this
> doc describes the shape of the deploy-time setup so the env vars and
> sampling rates can be staged in advance.

## 1. Create a Honeycomb environment

1. Sign in to <https://ui.honeycomb.io>. fold_db's account lives under the
   `edgevector` team; ask in #observability for an invite if you don't see
   it.
2. **Environments → New Environment.** Name it after the deploy stage:
   - `fold-db-dev-<your-handle>` for personal dev work
   - `fold-db-staging` for the shared staging account
   - `fold-db-prod` for production
3. **Environment Settings → API Keys → Create API Key.** Scope:
   - `Send Events` ✅
   - everything else ❌
4. Copy the key — it starts with `hcaik_…` for ingest keys. Store it the
   same way you store any other credential (1Password, deploy-tool secret
   manager, etc.). Do not commit it.

## 2. Point a local node at Honeycomb

Honeycomb speaks OTLP/HTTP at `https://api.honeycomb.io/v1/traces` (US) or
`https://api.eu1.honeycomb.io/v1/traces` (EU). The ingest key goes in the
`x-honeycomb-team` header.

Add to your local `.envrc` (or shell rc) — keep this file out of git:

```sh
export OBS_OTLP_ENDPOINT="https://api.honeycomb.io/v1/traces"
export OBS_OTLP_HEADERS="x-honeycomb-team=hcaik_replace_with_real_key"
export OBS_SAMPLER="parentbased_traceidratio:1.0"   # 100% in dev
```

Run a node:

```sh
cargo run -p fold_db_node
```

Open Honeycomb → your env → **Query** and filter on
`service.name = fold_db_node`. The first trace usually shows up within
2-5 seconds of the first request handled.

## 3. Sampling configuration: `OBS_SAMPLER`

The env var mirrors OTel's [`OTEL_TRACES_SAMPLER`][otel-sampler-spec]
syntax so tuning advice copies cleanly across stacks. Recognized values:

| Spec string                        | Meaning                                                     |
|------------------------------------|-------------------------------------------------------------|
| `always_on`                        | Keep every span. Good for one-off load tests.               |
| `always_off`                       | Drop every span. Useful when isolating non-trace overhead.  |
| `traceidratio:<f>`                 | Keep `<f>` of traces. Ignores upstream parent decision.     |
| `parentbased_traceidratio:<f>`     | Honour parent's decision; for root spans, use ratio `<f>`.  |

`<f>` is a float in `[0.0, 1.0]`. Out-of-range, NaN, or non-numeric
values cause the node to refuse to boot — silent clamping is forbidden
because it masks operator typos.

When `OBS_SAMPLER` is unset, the default is `parentbased_traceidratio:1.0`
(100%, parent-honouring). Safe in dev; **must be tuned down before any
prod deploy** to avoid blowing the Honeycomb monthly event budget.

### Recommended per-env settings

| Env       | `OBS_SAMPLER` value                  | Why                                                                                           |
|-----------|--------------------------------------|-----------------------------------------------------------------------------------------------|
| dev       | `parentbased_traceidratio:1.0`       | Operator is the only consumer; lose nothing.                                                  |
| staging   | `parentbased_traceidratio:0.5`       | Half-rate keeps representative coverage at half the event cost.                               |
| prod      | `parentbased_traceidratio:0.1`       | 10% head sampling. Errors still flow via the Sentry layer; head sampling does not gate them.  |

`parentbased_` is what we want everywhere: when an upstream service
decides to keep a trace, we keep our part too — the alternative is
half-broken trees in the Honeycomb waterfall view.

### Cost projection

Honeycomb's free tier is 20M events/month; Pro is metered above that
([calculator][honeycomb-pricing]).  Using a rough back-of-envelope of
**~50 spans per fold_db request** (handler + storage + propagation
hops):

| Sampling | Requests/day for 20M/month free tier |
|----------|--------------------------------------|
| 100%     | ~13K req/day                          |
| 50%      | ~26K req/day                          |
| 10%      | ~133K req/day                         |
| 1%       | ~1.3M req/day                         |

Numbers are rough — the real per-request span count moves around as we
add instrumentation (Phase 4 / T6 LLM tracing will roughly double it).
Re-run the math when production traffic hits ~1K req/day so the next
budget cycle is informed.

## 4. Errors are independent of head sampling

Head sampling decides whether spans are *recorded*. The Sentry / error
routing layer reads `tracing::Event` levels regardless of the sampler
decision, so:

- A dropped trace **still emits its error events** to Sentry.
- A `tracing::error!` inside an unsampled span **still pages on-call**.

Don't lower the sampler in an attempt to silence noisy errors — fix the
errors upstream. Lowering the sampler only blinds you to the trace
context that explains *why* an error happened.

## 5. Tail sampling (deferred)

The right answer for "keep all error traces, drop most successful ones"
is a tail sampler running in an OTel Collector between fold_db and
Honeycomb. We're deferring that until prod throughput justifies the
operational cost of running the Collector. Track this in the deferred
follow-up list at the bottom of `docs/observability/sampling-config-notes.md`.

[otel-sampler-spec]: https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/#general-sdk-configuration
[honeycomb-pricing]: https://www.honeycomb.io/pricing
