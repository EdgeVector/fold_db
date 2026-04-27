//! FMT layer — JSON formatter with format-time PII redaction.
//!
//! Defense-in-depth alongside the [`crate::redact!`] / [`crate::redact_id!`]
//! macros: even if a call site forgets to wrap a sensitive value, this
//! formatter still scrubs the value at write time when the field name
//! matches the deny-list.
//!
//! Output shape follows the OpenTelemetry log data model — one JSON object
//! per line with `time_unix_nano`, `severity_text`, `severity_number`,
//! `body`, `target`, optional `span`, and an `attributes` object holding
//! the event's structured fields.
//!
//! ```ignore
//! use observability::layers::fmt::{build_fmt_layer, FmtTarget};
//! use tracing_subscriber::layer::SubscriberExt;
//! use tracing_subscriber::Registry;
//!
//! let (layer, _guard) =
//!     build_fmt_layer::<Registry>(FmtTarget::Stdout).expect("init fmt layer");
//! let subscriber = Registry::default().with(layer);
//! tracing::subscriber::set_global_default(subscriber).unwrap();
//! ```

use std::collections::HashSet;
use std::fmt;
use std::fs::OpenOptions;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

use crate::ObsError;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Where the FMT layer writes formatted JSON events.
#[derive(Debug, Clone)]
pub enum FmtTarget {
    /// Append-write to a regular file. Created if absent.
    File(PathBuf),
    /// Process stdout. Suitable for Lambda / docker-style log capture.
    Stdout,
    /// Process stderr. Suitable for CLIs that reserve stdout for output.
    Stderr,
}

/// Holds the [`tracing_appender`] worker thread alive so its background
/// flush keeps draining the queue. **Must be retained for the lifetime of
/// the binary** — dropping the guard stops the worker mid-flush and any
/// log lines still in the channel are lost.
#[must_use = "FmtGuard must be held for the lifetime of the binary or log lines may be dropped"]
pub struct FmtGuard {
    _worker: WorkerGuard,
}

/// Build a JSON FMT [`Layer`] writing to `target`.
///
/// Returns the layer plus a [`FmtGuard`] which must be held alive for the
/// lifetime of the process. The layer applies the format-time redaction
/// deny-list (static names + `OBS_REDACT_EXTRA` env var) regardless of
/// what the call site passed for those fields.
///
/// This convenience constructor fixes the layer's `Subscriber` type
/// parameter to `S`, which limits how it can compose with other layers.
/// For multi-layer registries (FMT + RELOAD + RING), prefer
/// [`build_fmt_writer`] and inline the [`tracing_subscriber::fmt::layer`]
/// call so the compiler can infer the right `S` at the composition site.
pub fn build_fmt_layer<S>(target: FmtTarget) -> Result<(impl Layer<S>, FmtGuard), ObsError>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let (writer, guard) = build_fmt_writer(target)?;
    let layer = tracing_subscriber::fmt::layer()
        .event_format(RedactingFormat::from_env())
        .with_writer(writer);
    Ok((layer, guard))
}

/// Open the target sink and wrap it in [`tracing_appender::non_blocking`].
///
/// Used by [`build_fmt_layer`] above and by the multi-layer init helpers in
/// [`crate::init`], which build the fmt layer inline so the layer's
/// `Subscriber` type parameter is inferred at the composition site.
pub(crate) fn build_fmt_writer(target: FmtTarget) -> Result<(NonBlocking, FmtGuard), ObsError> {
    let writer: Box<dyn io::Write + Send + 'static> = match target {
        FmtTarget::File(path) => {
            let file = OpenOptions::new().create(true).append(true).open(&path)?;
            Box::new(file)
        }
        FmtTarget::Stdout => Box::new(io::stdout()),
        FmtTarget::Stderr => Box::new(io::stderr()),
    };
    let (non_blocking, worker) = tracing_appender::non_blocking(writer);
    Ok((non_blocking, FmtGuard { _worker: worker }))
}

// ---------------------------------------------------------------------------
// Deny-list — field names whose values are scrubbed at format time.
// ---------------------------------------------------------------------------

/// Compile-time deny-list. The dotted variants (`auth.token`, `api.key`)
/// match the canonical attribute style from [`crate::attrs`]; the
/// underscore variants match common ad-hoc field names.
const STATIC_DENY_LIST: &[&str] = &[
    "auth_token",
    "auth.token",
    "password",
    "api_key",
    "api.key",
    "secret",
    "email",
    "phone",
    "ssn",
];

const REDACTED_PLACEHOLDER: &str = "<redacted>";

const OBS_REDACT_EXTRA_ENV: &str = "OBS_REDACT_EXTRA";

#[derive(Clone, Debug)]
pub(crate) struct DenyList {
    set: HashSet<String>,
}

impl DenyList {
    /// Static list plus comma-separated names from the `OBS_REDACT_EXTRA`
    /// env var. Reads the env var fresh on every call — the layer
    /// constructor calls this once at startup, so the snapshot is taken
    /// when the binary boots.
    pub(crate) fn from_env() -> Self {
        let raw = std::env::var(OBS_REDACT_EXTRA_ENV).unwrap_or_default();
        let extras: Vec<&str> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        Self::with_extras(&extras)
    }

    pub(crate) fn with_extras(extras: &[&str]) -> Self {
        let mut set: HashSet<String> = STATIC_DENY_LIST.iter().map(|s| (*s).to_string()).collect();
        for extra in extras {
            set.insert((*extra).to_string());
        }
        Self { set }
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.set.contains(name)
    }
}

// ---------------------------------------------------------------------------
// RedactingFormat — custom FormatEvent impl
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) struct RedactingFormat {
    deny_list: DenyList,
    service_name: Option<String>,
}

impl RedactingFormat {
    pub(crate) fn from_env() -> Self {
        Self {
            deny_list: DenyList::from_env(),
            service_name: None,
        }
    }

    /// Like [`Self::from_env`] but stamps every formatted line with the OTel
    /// resource attribute `service.name = <name>`. Used by [`crate::init_node`]
    /// so a binary's file output is self-identifying.
    pub(crate) fn from_env_with_service(service_name: &str) -> Self {
        Self {
            deny_list: DenyList::from_env(),
            service_name: Some(service_name.to_string()),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_extras(extras: &[&str]) -> Self {
        Self {
            deny_list: DenyList::with_extras(extras),
            service_name: None,
        }
    }
}

impl<S, N> FormatEvent<S, N> for RedactingFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        let level = *metadata.level();

        let mut visitor = JsonFieldVisitor::new(&self.deny_list);
        event.record(&mut visitor);
        let JsonFieldVisitor {
            body, attributes, ..
        } = visitor;

        let mut obj = Map::new();
        obj.insert(
            "time_unix_nano".into(),
            Value::String(now_unix_nanos().to_string()),
        );
        obj.insert("severity_text".into(), Value::String(level.to_string()));
        obj.insert(
            "severity_number".into(),
            Value::from(severity_number(level)),
        );
        obj.insert("body".into(), Value::String(body.unwrap_or_default()));
        obj.insert(
            "target".into(),
            Value::String(metadata.target().to_string()),
        );
        if let Some(name) = self.service_name.as_deref() {
            obj.insert("service.name".into(), Value::String(name.to_string()));
        }
        if let Some(span) = ctx.lookup_current() {
            obj.insert("span".into(), Value::String(span.name().to_string()));
        }
        if !attributes.is_empty() {
            obj.insert("attributes".into(), Value::Object(attributes));
        }

        let line = serde_json::to_string(&Value::Object(obj)).map_err(|_| fmt::Error)?;
        writeln!(writer, "{line}")
    }
}

fn severity_number(level: Level) -> u32 {
    // Map tracing levels to the OTel SeverityNumber enum.
    match level {
        Level::TRACE => 1,
        Level::DEBUG => 5,
        Level::INFO => 9,
        Level::WARN => 13,
        Level::ERROR => 17,
    }
}

fn now_unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Visitor — walks event fields, applying the deny-list per field name.
// ---------------------------------------------------------------------------

struct JsonFieldVisitor<'a> {
    deny_list: &'a DenyList,
    body: Option<String>,
    attributes: Map<String, Value>,
}

impl<'a> JsonFieldVisitor<'a> {
    fn new(deny_list: &'a DenyList) -> Self {
        Self {
            deny_list,
            body: None,
            attributes: Map::new(),
        }
    }

    fn record_field(&mut self, name: &str, value: Value) {
        // tracing routes the macro's bare-string message to the special
        // field named `message`; promote it to the OTel `body` slot.
        if name == "message" {
            self.body = Some(match value {
                Value::String(s) => s,
                other => other.to_string(),
            });
            return;
        }
        let final_value = if self.deny_list.contains(name) {
            Value::String(REDACTED_PLACEHOLDER.into())
        } else {
            value
        };
        self.attributes.insert(name.to_string(), final_value);
    }
}

impl<'a> Visit for JsonFieldVisitor<'a> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_field(field.name(), Value::String(value.into()));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_field(field.name(), Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record_field(field.name(), Value::Number(value.into()));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        let num = serde_json::Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or(Value::Null);
        self.record_field(field.name(), num);
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record_field(field.name(), Value::Bool(value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record_field(field.name(), Value::String(format!("{value:?}")));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex, OnceLock};
    use tracing::subscriber::with_default;
    use tracing_subscriber::fmt::MakeWriter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    /// Serializes env-var manipulation so parallel tests don't see each
    /// other's `OBS_REDACT_EXTRA` mutations.
    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[derive(Clone)]
    struct VecMakeWriter(Arc<Mutex<Vec<u8>>>);

    impl<'a> MakeWriter<'a> for VecMakeWriter {
        type Writer = VecWriter;
        fn make_writer(&'a self) -> Self::Writer {
            VecWriter(self.0.clone())
        }
    }

    struct VecWriter(Arc<Mutex<Vec<u8>>>);

    impl io::Write for VecWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn capture<F: FnOnce()>(format: RedactingFormat, emit: F) -> Vec<Value> {
        let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let layer = tracing_subscriber::fmt::layer()
            .event_format(format)
            .with_writer(VecMakeWriter(buf.clone()));
        let subscriber = Registry::default().with(layer);
        with_default(subscriber, emit);
        let bytes = buf.lock().unwrap().clone();
        let text = String::from_utf8(bytes).expect("utf-8 output");
        text.lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str::<Value>(l).expect("valid json line"))
            .collect()
    }

    #[test]
    fn no_deny_list_fields_render_verbatim() {
        let lines = capture(RedactingFormat::with_extras(&[]), || {
            tracing::info!(user_id = "alice", count = 42, "request received");
        });
        assert_eq!(lines.len(), 1);
        let line = &lines[0];
        assert_eq!(line["body"], "request received");
        assert_eq!(line["severity_text"], "INFO");
        assert_eq!(line["severity_number"], 9);
        let attrs = line["attributes"].as_object().expect("attributes object");
        assert_eq!(attrs["user_id"], Value::String("alice".into()));
        assert_eq!(attrs["count"], Value::Number(42i64.into()));
    }

    #[test]
    fn password_field_is_redacted() {
        let lines = capture(RedactingFormat::with_extras(&[]), || {
            tracing::info!(user_id = "alice", password = "hunter2", "login attempt");
        });
        assert_eq!(lines.len(), 1);
        let attrs = lines[0]["attributes"]
            .as_object()
            .expect("attributes object");
        assert_eq!(
            attrs["password"],
            Value::String(REDACTED_PLACEHOLDER.into()),
            "password value must never reach the formatted output",
        );
        // Non-deny-list fields still appear verbatim.
        assert_eq!(attrs["user_id"], Value::String("alice".into()));
        // And the raw secret literal is nowhere on the line.
        let raw = serde_json::to_string(&lines[0]).unwrap();
        assert!(!raw.contains("hunter2"), "redacted value leaked: {raw}");
    }

    #[test]
    fn each_static_deny_list_field_is_redacted() {
        let lines = capture(RedactingFormat::with_extras(&[]), || {
            tracing::info!(
                auth_token = "t",
                password = "p",
                api_key = "k",
                secret = "s",
                email = "e@x",
                phone = "555",
                ssn = "111",
                "all sensitive",
            );
        });
        let attrs = lines[0]["attributes"]
            .as_object()
            .expect("attributes object");
        for k in [
            "auth_token",
            "password",
            "api_key",
            "secret",
            "email",
            "phone",
            "ssn",
        ] {
            assert_eq!(
                attrs[k],
                Value::String(REDACTED_PLACEHOLDER.into()),
                "field {k} should be redacted",
            );
        }
    }

    #[test]
    fn obs_redact_extra_extends_deny_list_at_runtime() {
        let _guard = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var(OBS_REDACT_EXTRA_ENV).ok();
        // SAFETY: env mutation is serialized via env_lock above.
        std::env::set_var(OBS_REDACT_EXTRA_ENV, "shoe_size, favorite_color");
        let format = RedactingFormat::from_env();
        // Restore env immediately — `format` already snapshotted the deny-list.
        match prev {
            Some(v) => std::env::set_var(OBS_REDACT_EXTRA_ENV, v),
            None => std::env::remove_var(OBS_REDACT_EXTRA_ENV),
        }

        let lines = capture(format, || {
            tracing::info!(
                shoe_size = 10,
                favorite_color = "blue",
                note = "ok",
                "extras",
            );
        });
        let attrs = lines[0]["attributes"]
            .as_object()
            .expect("attributes object");
        assert_eq!(
            attrs["shoe_size"],
            Value::String(REDACTED_PLACEHOLDER.into()),
        );
        assert_eq!(
            attrs["favorite_color"],
            Value::String(REDACTED_PLACEHOLDER.into()),
        );
        assert_eq!(attrs["note"], Value::String("ok".into()));
    }

    #[test]
    fn obs_redact_extra_email_added_at_runtime() {
        // Mirrors the spec example verbatim: `OBS_REDACT_EXTRA=email`.
        // Even though `email` is already in the static list, this verifies
        // the env-var path is wired up.
        let _guard = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var(OBS_REDACT_EXTRA_ENV).ok();
        std::env::set_var(OBS_REDACT_EXTRA_ENV, "email");
        let format = RedactingFormat::from_env();
        match prev {
            Some(v) => std::env::set_var(OBS_REDACT_EXTRA_ENV, v),
            None => std::env::remove_var(OBS_REDACT_EXTRA_ENV),
        }

        let lines = capture(format, || {
            tracing::info!(email = "user@example.com", "signup");
        });
        let attrs = lines[0]["attributes"]
            .as_object()
            .expect("attributes object");
        assert_eq!(attrs["email"], Value::String(REDACTED_PLACEHOLDER.into()));
    }

    #[test]
    fn output_is_json_lines_with_otel_shape() {
        let lines = capture(RedactingFormat::with_extras(&[]), || {
            tracing::warn!(http_method = "GET", "edge case");
        });
        let line = &lines[0];
        assert!(line["time_unix_nano"].is_string(), "time_unix_nano present");
        assert_eq!(line["severity_text"], "WARN");
        assert_eq!(line["severity_number"], 13);
        assert_eq!(line["body"], "edge case");
        assert!(line["target"].is_string(), "target present");
        let attrs = line["attributes"].as_object().expect("attributes object");
        assert_eq!(attrs["http_method"], Value::String("GET".into()));
    }

    #[test]
    fn fmt_layer_composes_into_registry_without_panic() {
        // Drives the `Layer<S>` impl bound for the `Registry` Subscriber —
        // exercising what the init helpers will do at startup.
        let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let layer = tracing_subscriber::fmt::layer()
            .event_format(RedactingFormat::with_extras(&[]))
            .with_writer(VecMakeWriter(buf));
        let subscriber = Registry::default().with(layer);
        with_default(subscriber, || {
            tracing::info!("smoke");
        });
    }

    #[test]
    fn build_fmt_layer_writes_to_file_target() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fmt.log");

        let (layer, guard) =
            build_fmt_layer::<Registry>(FmtTarget::File(path.clone())).expect("build layer");
        let subscriber = Registry::default().with(layer);
        with_default(subscriber, || {
            tracing::info!(password = "hunter2", user = "alice", "login");
        });
        // Drop the guard so the worker thread drains its queue to the file.
        drop(guard);

        let contents = std::fs::read_to_string(&path).expect("read log");
        let line = contents
            .lines()
            .find(|l| !l.is_empty())
            .expect("at least one line");
        let parsed: Value = serde_json::from_str(line).expect("valid json");
        let attrs = parsed["attributes"].as_object().expect("attributes object");
        assert_eq!(
            attrs["password"],
            Value::String(REDACTED_PLACEHOLDER.into()),
        );
        assert!(!contents.contains("hunter2"), "secret leaked into file");
    }
}
