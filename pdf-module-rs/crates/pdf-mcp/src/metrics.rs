//! Prometheus metrics middleware and endpoint for the MCP HTTP server.
//!
//! Provides:
//! - `GET /metrics` endpoint exposing OpenMetrics text
//! - Tower middleware that tracks HTTP request count, duration, and status codes
//! - Process metrics (CPU, memory) via prometheus process collector
//!
//! # Usage
//!
//! ```ignore
//! use crate::metrics::MetricsLayer;
//!
//! let app = Router::new()
//!     .route("/metrics", get(metrics_endpoint))
//!     .layer(MetricsLayer::new())
//!     .with_state(state);
//! ```

use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Router};
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder,
};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;
use tower::{Layer, Service};

static DEFAULT_BUCKETS: &[f64] = &[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];

pub struct HttpMetrics {
    pub registry: Registry,
    pub requests_total: IntCounterVec,
    pub request_duration: HistogramVec,
}

impl HttpMetrics {
    pub fn new() -> Self {
        let registry = Registry::new_custom(Some("rsut_pdf_mcp".into()), None)
            .expect("failed to create metrics registry");

        let requests_total = IntCounterVec::new(
            Opts::new("http_requests_total", "Total HTTP requests").namespace("rsut_pdf_mcp"),
            &["method", "path", "status"],
        )
        .expect("failed to create http_requests_total");

        let request_duration = HistogramVec::new(
            HistogramOpts::new("http_request_duration_seconds", "HTTP request duration")
                .namespace("rsut_pdf_mcp")
                .buckets(DEFAULT_BUCKETS.to_vec()),
            &["method", "path"],
        )
        .expect("failed to create http_request_duration_seconds");

        registry
            .register(Box::new(requests_total.clone()))
            .expect("failed to register http_requests_total");
        registry
            .register(Box::new(request_duration.clone()))
            .expect("failed to register http_request_duration_seconds");

        let process_collector = prometheus::process_collector::ProcessCollector::for_self();
        registry
            .register(Box::new(process_collector))
            .expect("failed to register process collector");

        Self { registry, requests_total, request_duration }
    }

    pub fn render(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).ok();
        String::from_utf8(buffer).unwrap_or_default()
    }
}

impl Default for HttpMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Handler for `GET /metrics`.
pub async fn metrics_endpoint(
    Extension(metrics): Extension<Arc<HttpMetrics>>,
) -> impl IntoResponse {
    let body = metrics.render();
    (StatusCode::OK, body)
}

/// Attach the `/metrics` route to an existing router.
pub fn add_metrics_route(router: Router, metrics: Arc<HttpMetrics>) -> Router {
    router.route("/metrics", get(metrics_endpoint)).layer(axum::extract::Extension(metrics))
}

/// Tower Layer that wraps every request with metrics tracking.
#[derive(Clone)]
pub struct MetricsLayer {
    metrics: Arc<HttpMetrics>,
}

impl MetricsLayer {
    pub fn new(metrics: Arc<HttpMetrics>) -> Self {
        Self { metrics }
    }
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsService { inner, metrics: Arc::clone(&self.metrics) }
    }
}

#[derive(Clone)]
pub struct MetricsService<S> {
    inner: S,
    metrics: Arc<HttpMetrics>,
}

impl<S, ReqBody> Service<Request<ReqBody>> for MetricsService<S>
where
    S: Service<Request<ReqBody>, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<axum::BoxError>,
    ReqBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let start = Instant::now();

        let metrics = Arc::clone(&self.metrics);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let response = inner.call(req).await?;
            let status = response.status().as_u16().to_string();
            let duration = start.elapsed().as_secs_f64();

            metrics.requests_total.with_label_values(&[&method, &path, &status]).inc();
            metrics.request_duration.with_label_values(&[&method, &path]).observe(duration);

            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_render_opens_with_help() {
        let m = HttpMetrics::new();
        m.requests_total.with_label_values(&["GET", "/health", "200"]).inc();
        let output = m.render();
        assert!(output.contains("http_requests_total"));
        assert!(output.contains("rsut_pdf_mcp"));
    }

    #[test]
    fn metrics_records_multiple_statuses() {
        let m = HttpMetrics::new();
        m.requests_total.with_label_values(&["GET", "/api/health", "200"]).inc();
        m.requests_total.with_label_values(&["POST", "/api/config", "500"]).inc();
        m.requests_total.with_label_values(&["GET", "/api/health", "200"]).inc();

        let output = m.render();
        assert!(output.contains("200"));
        assert!(output.contains("500"));
    }

    #[test]
    fn histogram_observes_durations() {
        let m = HttpMetrics::new();
        m.request_duration.with_label_values(&["GET", "/api/search"]).observe(0.15);
        m.request_duration.with_label_values(&["GET", "/api/search"]).observe(0.35);

        let output = m.render();
        assert!(output.contains("http_request_duration_seconds"));
    }
}
