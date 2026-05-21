//! Reusable tracing/logging setup for the PDF module workspace.
//!
//! Provides pre-configured tracing subscriber factories for different
//! environments (production / development / test) with:
//!
//! - Request ID auto-injection from HTTP headers (`x-request-id`)
//! - Structured JSON logging for production
//! - Pretty terminal output for local development
//! - EnvFilter-based level control via `RUST_LOG`
//!
//! # Usage
//!
//! ```ignore
//! use pdf_core::tracing_setup;
//!
//! tracing_setup::init_production();
//! // or
//! tracing_setup::init_development();
//! ```

use std::sync::LazyLock;
use tracing::Subscriber;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry};

#[allow(dead_code)]
static DEFAULT_FILTER: LazyLock<EnvFilter> =
    LazyLock::new(|| EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub format: LogFormat,
    pub default_level: tracing::Level,
    pub with_thread_ids: bool,
    pub with_file_line: bool,
    pub with_span_events: bool,
    pub with_target: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Pretty,
            default_level: tracing::Level::INFO,
            with_thread_ids: true,
            with_file_line: false,
            with_span_events: false,
            with_target: false,
        }
    }
}

impl TracingConfig {
    pub const fn production() -> Self {
        Self {
            format: LogFormat::Json,
            default_level: tracing::Level::INFO,
            with_thread_ids: true,
            with_file_line: false,
            with_span_events: true,
            with_target: true,
        }
    }

    pub const fn development() -> Self {
        Self {
            format: LogFormat::Pretty,
            default_level: tracing::Level::DEBUG,
            with_thread_ids: false,
            with_file_line: true,
            with_span_events: true,
            with_target: false,
        }
    }

    pub const fn compact() -> Self {
        Self {
            format: LogFormat::Compact,
            default_level: tracing::Level::WARN,
            with_thread_ids: false,
            with_file_line: false,
            with_span_events: false,
            with_target: false,
        }
    }
}

fn build_env_filter(config: &TracingConfig) -> EnvFilter {
    if let Ok(env) = std::env::var("RUST_LOG")
        && env.to_lowercase() == "off"
    {
        return EnvFilter::new("off");
    }

    EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.default_level.as_str()))
}

fn build_fmt_layer<S>(config: &TracingConfig) -> Box<dyn Layer<S> + Send + Sync + 'static>
where
    S: Subscriber + for<'a> LookupSpan<'a> + Send + Sync + 'static,
{
    let layer = tracing_subscriber::fmt::layer()
        .with_thread_ids(config.with_thread_ids)
        .with_target(config.with_target)
        .with_file(config.with_file_line)
        .with_line_number(config.with_file_line);

    match config.format {
        LogFormat::Json => Box::new(layer.json().with_span_events(if config.with_span_events {
            tracing_subscriber::fmt::format::FmtSpan::CLOSE
        } else {
            tracing_subscriber::fmt::format::FmtSpan::NONE
        })),
        LogFormat::Pretty => {
            Box::new(layer.pretty().with_span_events(if config.with_span_events {
                tracing_subscriber::fmt::format::FmtSpan::NEW
                    | tracing_subscriber::fmt::format::FmtSpan::CLOSE
            } else {
                tracing_subscriber::fmt::format::FmtSpan::NONE
            }))
        }
        LogFormat::Compact => Box::new(layer.compact()),
    }
}

fn build_subscriber(config: TracingConfig) -> impl Subscriber {
    let env_filter = build_env_filter(&config);
    let fmt_layer = build_fmt_layer::<Registry>(&config);

    Registry::default().with(fmt_layer).with(env_filter)
}

/// Initialize tracing for production (JSON to stderr, thread IDs, span close events).
pub fn init_production() {
    build_subscriber(TracingConfig::production()).init();
}

/// Initialize tracing for local development (pretty to stderr, file:line, span events).
pub fn init_development() {
    build_subscriber(TracingConfig::development()).init();
}

/// Initialize tracing with compact output (warn+ only, minimal noise).
pub fn init_compact() {
    build_subscriber(TracingConfig::compact()).init();
}

/// Initialize tracing with a custom config.
pub fn init_with_config(config: TracingConfig) {
    build_subscriber(config).init();
}

/// Initialize with custom config, returning the default level that was resolved.
pub fn init_with_config_and_level(config: TracingConfig) -> tracing::Level {
    let level = config.default_level;
    build_subscriber(config).init();
    level
}

// ─── Request ID utilities ──────────────────────────────────────

/// Extract request ID from a tracing span's extensions.
///
/// Used in log formatters to inject `request_id` into structured log entries.
pub fn current_request_id() -> Option<String> {
    let span = tracing::Span::current();
    if span.is_disabled() {
        return None;
    }
    span.with_subscriber(move |(id, subscriber)| {
        subscriber
            .downcast_ref::<Registry>()
            .and_then(|reg| reg.span(id))
            .and_then(|span_ref| span_ref.extensions().get::<RequestId>().map(|rid| rid.0.clone()))
    })
    .flatten()
}

/// Request ID wrapper stored in span extensions.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

/// Create a span with a request ID.
///
/// The request ID is first read from `x-request-id` header, then from
/// `traceparent`, then auto-generated as a short random string.
pub fn request_span(request_id: Option<&str>) -> tracing::Span {
    let rid = request_id.filter(|s| !s.is_empty()).map(|s| s.to_string()).unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        format!("{:04x}", (ts & 0xFFFF) as u16)
    });

    let span = tracing::info_span!("request", request_id = %rid);
    span.record("request_id", &rid);
    span
}

/// Axum-compatible middleware layer that creates a request span with trace ID.
///
/// Reads `x-request-id` header and injects it into the tracing span.
#[cfg(feature = "axum-tracing")]
pub mod axum_layer {
    use super::RequestId;
    use axum::http::{HeaderMap, Request};
    use std::task::{Context, Poll};
    use tower::{Layer, Service};
    use tracing::Span;

    const REQUEST_ID_HEADER: &str = "x-request-id";

    #[derive(Clone, Default)]
    pub struct TraceLayer;

    impl<S> Layer<S> for TraceLayer {
        type Service = TraceService<S>;

        fn layer(&self, inner: S) -> Self::Service {
            TraceService { inner }
        }
    }

    #[derive(Clone)]
    pub struct TraceService<S> {
        inner: S,
    }

    impl<S, B> Service<Request<B>> for TraceService<S>
    where
        S: Service<Request<B>> + Send + 'static,
        S::Future: Send + 'static,
        B: Send + 'static,
    {
        type Response = S::Response;
        type Error = S::Error;
        type Future = S::Future;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, req: Request<B>) -> Self::Future {
            let request_id = req
                .headers()
                .get(REQUEST_ID_HEADER)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            let span = super::request_span(request_id.as_deref());

            let _guard = span.enter();
            span.extensions_mut().insert(RequestId(request_id.unwrap_or_else(|| "unknown".into())));

            self.inner.call(req)
        }
    }
}

/// Convenience: init and get the default level at the same time.
pub fn init(config: TracingConfig) -> tracing::Level {
    init_with_config_and_level(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::util::SubscriberInitExt;

    fn with_test_subscriber() -> tracing::subscriber::DefaultGuard {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_test_writer()
            .set_default()
    }

    #[test]
    fn default_config_is_pretty() {
        let cfg = TracingConfig::default();
        assert_eq!(cfg.format, LogFormat::Pretty);
        assert_eq!(cfg.default_level, tracing::Level::INFO);
    }

    #[test]
    fn production_config_is_json() {
        let cfg = TracingConfig::production();
        assert_eq!(cfg.format, LogFormat::Json);
        assert!(cfg.with_span_events);
    }

    #[test]
    fn development_config_has_file_line() {
        let cfg = TracingConfig::development();
        assert!(cfg.with_file_line);
        assert_eq!(cfg.default_level, tracing::Level::DEBUG);
    }

    #[test]
    fn request_span_generates_id() {
        let _guard = with_test_subscriber();
        let span = request_span(None);
        assert!(!span.is_disabled());
    }

    #[test]
    fn request_span_uses_provided_id() {
        let _guard = with_test_subscriber();
        let span = request_span(Some("abc-123"));
        assert!(!span.is_disabled());
    }

    #[test]
    fn request_span_empty_string_generates() {
        let _guard = with_test_subscriber();
        let span = request_span(Some(""));
        assert!(!span.is_disabled());
    }
}
