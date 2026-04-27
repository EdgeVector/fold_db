//! RELOAD layer — runtime `EnvFilter` updates via a [`ReloadHandle`].
//!
//! Wraps [`tracing_subscriber::reload`] so callers can swap filter directives
//! at runtime without re-installing the subscriber. This subsumes the legacy
//! `LoggingSystem::update_feature_level` capability and generalizes it to the
//! full `EnvFilter` directive syntax (e.g. `"my_crate::module=debug,info"`),
//! enabling per-target filtering rather than just a flat per-feature level.

use tracing::Subscriber;
use tracing_subscriber::reload;
use tracing_subscriber::EnvFilter;

/// Errors raised by [`ReloadHandle::update`].
#[derive(Debug, thiserror::Error)]
pub enum ReloadError {
    /// The supplied directive could not be parsed as an [`EnvFilter`].
    #[error("invalid filter directive: {0}")]
    Parse(String),
    /// The reload handle could not be applied (e.g. the subscriber was
    /// dropped or the inner lock is poisoned).
    #[error("failed to apply filter: {0}")]
    Apply(String),
}

/// Type-erased closure that parses a directive and reloads the wrapped layer.
type ApplyFn = dyn Fn(&str) -> Result<(), ReloadError> + Send + Sync;

/// Type-erased handle to swap the active [`EnvFilter`] at runtime.
///
/// Cloning the underlying handle is cheap (it stores an `Arc` internally), but
/// since this struct erases the subscriber type parameter we wrap it in a
/// boxed closure. Wrap in [`std::sync::Arc`] if multiple owners need it.
pub struct ReloadHandle {
    apply: Box<ApplyFn>,
}

impl ReloadHandle {
    /// Parse `directive` as an [`EnvFilter`] and install it as the active
    /// filter. Subsequent log events are filtered by the new directive.
    pub fn update(&self, directive: &str) -> Result<(), ReloadError> {
        (self.apply)(directive)
    }
}

impl std::fmt::Debug for ReloadHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReloadHandle").finish_non_exhaustive()
    }
}

/// Build a reloadable [`EnvFilter`] layer.
///
/// Returns the layer (to be added to a `Registry`) and a [`ReloadHandle`]
/// that can be stored on the node / lambda / app context and exposed to
/// HTTP or IPC handlers for runtime filter updates.
pub fn build_reload_layer<S>(
    initial: EnvFilter,
) -> (reload::Layer<EnvFilter, S>, ReloadHandle)
where
    S: Subscriber,
{
    let (layer, handle) = reload::Layer::new(initial);
    let apply = Box::new(move |directive: &str| -> Result<(), ReloadError> {
        let filter =
            EnvFilter::try_new(directive).map_err(|e| ReloadError::Parse(e.to_string()))?;
        handle
            .reload(filter)
            .map_err(|e| ReloadError::Apply(e.to_string()))
    });
    (layer, ReloadHandle { apply })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tracing::{debug, info};
    use tracing_subscriber::fmt::MakeWriter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    /// MakeWriter that captures all output into a shared buffer for assertion.
    #[derive(Clone, Default)]
    struct CaptureWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl CaptureWriter {
        fn captured(&self) -> String {
            String::from_utf8(self.buf.lock().unwrap().clone()).unwrap()
        }
    }

    impl io::Write for CaptureWriter {
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            self.buf.lock().unwrap().extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for CaptureWriter {
        type Writer = CaptureWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn subscriber_with(
        initial: &str,
        writer: CaptureWriter,
    ) -> (impl Subscriber + Send + Sync, ReloadHandle) {
        let (reload_layer, handle) =
            build_reload_layer::<Registry>(EnvFilter::new(initial));
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_writer(writer)
            .without_time()
            .with_ansi(false);
        let subscriber = Registry::default().with(reload_layer).with(fmt_layer);
        (subscriber, handle)
    }

    #[test]
    fn initial_info_filter_drops_debug() {
        let writer = CaptureWriter::default();
        let (subscriber, _handle) = subscriber_with("info", writer.clone());

        tracing::subscriber::with_default(subscriber, || {
            info!("hello info");
            debug!("hello debug");
        });

        let out = writer.captured();
        assert!(out.contains("hello info"), "info should pass: {out}");
        assert!(!out.contains("hello debug"), "debug should drop: {out}");
    }

    #[test]
    fn update_enables_target_specific_debug() {
        let writer = CaptureWriter::default();
        let (subscriber, handle) = subscriber_with("info", writer.clone());

        tracing::subscriber::with_default(subscriber, || {
            // Pre-update: debug from any target is dropped.
            debug!(target: "my_crate", "pre-update-mycrate-debug");
            debug!(target: "other_crate", "pre-update-other-debug");

            handle.update("my_crate=debug,info").unwrap();

            // Post-update: my_crate=debug emits, other_crate=debug still drops,
            // info-level on any target still emits.
            debug!(target: "my_crate", "post-update-mycrate-debug");
            debug!(target: "other_crate", "post-update-other-debug");
            info!(target: "other_crate", "post-update-other-info");
        });

        let out = writer.captured();
        assert!(
            !out.contains("pre-update-mycrate-debug"),
            "pre-update debug should be dropped: {out}"
        );
        assert!(
            !out.contains("pre-update-other-debug"),
            "pre-update debug should be dropped: {out}"
        );
        assert!(
            out.contains("post-update-mycrate-debug"),
            "my_crate=debug should pass after update: {out}"
        );
        assert!(
            !out.contains("post-update-other-debug"),
            "other_crate=debug should still drop: {out}"
        );
        assert!(
            out.contains("post-update-other-info"),
            "info baseline should still pass: {out}"
        );
    }

    #[test]
    fn bad_directive_returns_err_without_panic() {
        let (_layer, handle) = build_reload_layer::<Registry>(EnvFilter::new("info"));
        let err = handle
            .update("=== not a valid directive ===")
            .expect_err("invalid directive must surface as Err");
        assert!(matches!(err, ReloadError::Parse(_)), "got: {err:?}");
    }
}
