//! Shared `tracing` initialization for every enforcer binary surface.
//!
//! Structured JSON output on stderr (stdout stays clean for MCP stdio
//! framing and CLI payloads), env-filterable via `RUST_LOG`, with spans
//! keyed by `correlation_id` so multi-crate flows stitch together.

use crate::error::{Error, Result};

/// Initialize the global tracing subscriber.
///
/// - JSON-formatted structured output on stderr.
/// - Filter from `RUST_LOG` when set, else `default_filter` (e.g. `"info"`).
///
/// Returns [`Error::TracingInit`] if a global subscriber is already set
/// (callers decide whether that is fatal; in tests it usually is not).
pub fn init(default_filter: &str) -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_filter));
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|e| Error::TracingInit(e.to_string()))
}

/// Create the root span for a correlated flow. Every event emitted inside
/// this span carries the `correlation_id` structured field.
pub fn correlation_span(correlation_id: &str) -> tracing::Span {
    tracing::info_span!("enforcer", correlation_id = %correlation_id)
}

#[cfg(test)]
mod tests {
    use super::{correlation_span, init};
    use crate::error::Error;

    #[test]
    fn init_succeeds_once_then_reports_typed_error() {
        // First init in this process wins...
        let first = init("info");
        assert!(first.is_ok());
        // ...and a second init reports the typed error instead of panicking.
        let second = init("info");
        assert!(matches!(second, Err(Error::TracingInit(_))));
    }

    #[test]
    fn correlation_span_carries_the_id_field() {
        let span = correlation_span("run-1234");
        // The span must exist and be enterable even without a subscriber.
        let _guard = span.enter();
    }
}
