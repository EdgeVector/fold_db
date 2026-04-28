//! W3C trace context propagation across HTTP boundaries.
//!
//! Two helpers cover the boundaries we control:
//!
//! - [`inject_w3c`] — wraps a `reqwest::RequestBuilder` and adds the
//!   `traceparent` (and any tracestate) headers derived from the *current*
//!   tracing span, so downstream services can stitch into the same trace.
//! - [`extract_parent_context`] — reads `traceparent` (and friends) from an
//!   `http::HeaderMap` on ingress and returns an `opentelemetry::Context`
//!   that callers attach to the server-side span via
//!   `tracing_opentelemetry::OpenTelemetrySpanExt::set_parent`.
//!
//! Both helpers depend on a global text-map propagator being installed.
//! The standard installation (done in the `init_*` helpers in T6) is
//! `opentelemetry_sdk::propagation::TraceContextPropagator`. Tests in this
//! module install one ad-hoc.
//!
//! AWS SDK egress is **not** covered — Lambdas talk to AWS services, which
//! use a different propagation format. This is intentionally out of scope:
//! AWS SDK calls are auth/billing/sync metadata, not on the user-facing
//! critical path. A lightweight `#[tracing::instrument]` on the Lambda
//! wrapper functions is the fallback if span coverage is needed.

use opentelemetry::global;
#[cfg(test)]
use opentelemetry::propagation::Extractor;
use opentelemetry::propagation::Injector;
use opentelemetry::Context;
use opentelemetry_http::HeaderExtractor;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Inject the current span's W3C trace context into a `reqwest` request.
///
/// Use at every outgoing HTTP call site that you want stitched into the
/// caller's trace. Per the plan's classification:
///
/// - `propagate` — wrap with `inject_w3c(builder)`.
/// - `loopback` — wrap (it's still our own service downstream).
/// - `skip-s3`, `skip-3p` — do **not** wrap; third-party services would
///   reject or ignore the headers.
pub fn inject_w3c(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let cx = Span::current().context();

    let mut injector = StringHeaderInjector::default();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&cx, &mut injector);
    });

    let mut builder = builder;
    for (key, value) in injector.headers {
        builder = builder.header(key, value);
    }
    builder
}

/// Extract a parent `Context` from incoming HTTP headers.
///
/// Server-side handlers call this on the request's `HeaderMap`, then attach
/// the result to their root span:
///
/// ```ignore
/// let parent = observability::propagation::extract_parent_context(req.headers());
/// let span = tracing::info_span!("http.request");
/// span.set_parent(parent);
/// ```
pub fn extract_parent_context(headers: &http::HeaderMap) -> Context {
    let extractor = HeaderExtractor(headers);
    global::get_text_map_propagator(|propagator| propagator.extract(&extractor))
}

/// `Injector` impl that buffers `(name, value)` pairs in a `Vec` so we can
/// apply them to a `reqwest::RequestBuilder` afterwards. This avoids the
/// `http` crate version coupling between `reqwest` (0.11 → http 0.2) and
/// `opentelemetry-http` (→ http 1.x).
#[derive(Default)]
struct StringHeaderInjector {
    headers: Vec<(String, String)>,
}

impl Injector for StringHeaderInjector {
    fn set(&mut self, key: &str, value: String) {
        self.headers.push((key.to_string(), value));
    }
}

/// Test-only `Extractor` over `Vec<(String, String)>`, used to round-trip
/// through a propagator without depending on a specific HTTP framework.
#[cfg(test)]
#[derive(Default)]
struct StringHeaderExtractor {
    headers: Vec<(String, String)>,
}

#[cfg(test)]
impl Extractor for StringHeaderExtractor {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.headers.iter().map(|(k, _)| k.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::trace::{TraceContextExt, TracerProvider};
    use opentelemetry_sdk::propagation::TraceContextPropagator;
    use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn install_propagator() -> opentelemetry_sdk::trace::Tracer {
        INIT.call_once(|| {
            global::set_text_map_propagator(TraceContextPropagator::new());
        });
        // Each test gets its own tracer so spans are real and have IDs.
        let provider = SdkTracerProvider::builder().build();
        provider.tracer("observability-test")
    }

    #[test]
    fn round_trips_traceparent_through_inject_and_extract() {
        use opentelemetry::trace::Tracer;

        let tracer = install_propagator();

        // Make a real span with a real SpanContext.
        let span = tracer.start("client.request");
        let cx = Context::current_with_span(span);
        let original_trace_id = cx.span().span_context().trace_id();
        let original_span_id = cx.span().span_context().span_id();

        // Inject from the context into a string-pair injector (mimics inject_w3c
        // without needing an actual reqwest client + a current tracing span).
        let mut injector = StringHeaderInjector::default();
        global::get_text_map_propagator(|p| p.inject_context(&cx, &mut injector));

        // We expect at least a `traceparent` header.
        let traceparent = injector
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("traceparent"))
            .map(|(_, v)| v.clone())
            .expect("traceparent header should be injected");
        assert!(
            traceparent.starts_with("00-"),
            "expected W3C v00 traceparent, got {traceparent}"
        );

        // Extract on the other end and verify the trace/span IDs survive.
        let extractor = StringHeaderExtractor {
            headers: injector.headers,
        };
        let extracted = global::get_text_map_propagator(|p| p.extract(&extractor));
        let extracted_ctx = extracted.span().span_context().clone();
        assert!(
            extracted_ctx.is_valid(),
            "extracted span context must be valid"
        );
        assert_eq!(extracted_ctx.trace_id(), original_trace_id);
        assert_eq!(extracted_ctx.span_id(), original_span_id);
    }

    #[test]
    fn extract_parent_context_reads_http_header_map() {
        let _tracer = install_propagator();

        // A hand-built valid W3C traceparent.
        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let mut headers = http::HeaderMap::new();
        headers.insert("traceparent", traceparent.parse().unwrap());

        let cx = extract_parent_context(&headers);
        let sc = cx.span().span_context().clone();
        assert!(sc.is_valid());
        assert_eq!(
            format!("{:032x}", sc.trace_id()),
            "0af7651916cd43dd8448eb211c80319c"
        );
        assert_eq!(format!("{:016x}", sc.span_id()), "b7ad6b7169203331");
    }
}
