use prometheus::{Encoder, Histogram, HistogramOpts, IntCounterVec, Opts, Registry, TextEncoder};
use std::sync::Arc;
use std::time::Instant;

/// Prometheus metrics collector for VLM gateway.
///
/// Exposes three metric families following OpenMetrics conventions:
/// - `vlm_requests_total{provider, status}` — request counter with labels
/// - `vlm_request_duration_seconds{provider}` — latency histogram with provider label
/// - `vlm_degradations_total{reason}` — degradation counter with reason label
pub struct MetricsCollector {
    registry: Registry,
    requests_total: IntCounterVec,
    request_duration: Histogram,
    degradations_total: IntCounterVec,
}

impl MetricsCollector {
    pub fn new(registry: Registry) -> Self {
        let requests_total = IntCounterVec::new(
            Opts::new("vlm_requests_total", "Total number of VLM requests"),
            &["provider", "status"],
        )
        .expect("failed to create vlm_requests_total");

        let request_duration = Histogram::with_opts(
            HistogramOpts::new(
                "vlm_request_duration_seconds",
                "VLM request duration in seconds",
            )
            .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 30.0]),
        )
        .expect("failed to create vlm_request_duration_seconds");

        let degradations_total = IntCounterVec::new(
            Opts::new("vlm_degradations_total", "Total number of VLM degradations"),
            &["reason"],
        )
        .expect("failed to create vlm_degradations_total");

        registry.register(Box::new(requests_total.clone())).ok();
        registry.register(Box::new(request_duration.clone())).ok();
        registry.register(Box::new(degradations_total.clone())).ok();

        Self {
            registry,
            requests_total,
            request_duration,
            degradations_total,
        }
    }

    /// Create with a shared default registry.
    pub fn with_default_registry() -> Self {
        Self::new(Registry::default())
    }

    /// Start a request timer.  Call `.observe_success(provider)` when done.
    pub fn start_request_timer(&self) -> RequestTimer<'_> {
        RequestTimer {
            start: Instant::now(),
            collector: self,
        }
    }

    fn record_success_inner(&self, elapsed: std::time::Duration, provider: &str) {
        self.requests_total
            .with_label_values(&[provider, "success"])
            .inc();
        self.request_duration.observe(elapsed.as_secs_f64());
    }

    fn record_timeout_inner(&self, provider: &str) {
        self.requests_total
            .with_label_values(&[provider, "timeout"])
            .inc();
    }

    fn record_error_inner(&self, provider: &str) {
        self.requests_total
            .with_label_values(&[provider, "error"])
            .inc();
    }

    /// Record a degradation event with reason label.
    pub fn record_degradation(&self, reason: &str) {
        self.degradations_total.with_label_values(&[reason]).inc();
    }

    /// Render all metrics as OpenMetrics text.
    pub fn render(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).ok();
        String::from_utf8(buffer).unwrap_or_default()
    }

    /// Wrap in Arc for sharing.
    pub fn into_shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}

/// Guard that records request duration.
pub struct RequestTimer<'a> {
    start: Instant,
    collector: &'a MetricsCollector,
}

impl<'a> RequestTimer<'a> {
    /// Mark the request as successful.
    pub fn observe_success(self, provider: &str) {
        let elapsed = self.start.elapsed();
        self.collector.record_success_inner(elapsed, provider);
    }

    /// Mark the request as timed out.
    pub fn observe_timeout(self, provider: &str) {
        self.collector.record_timeout_inner(provider);
    }

    /// Mark the request as errored.
    pub fn observe_error(self, provider: &str) {
        self.collector.record_error_inner(provider);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_metrics_with_labels() {
        let mc = MetricsCollector::with_default_registry();
        let timer = mc.start_request_timer();
        timer.observe_success("gpt-4o");

        let timer2 = mc.start_request_timer();
        timer2.observe_timeout("claude-3.5-sonnet");

        mc.record_degradation("timeout");
        mc.record_degradation("unavailable");
        mc.record_degradation("timeout");

        let output = mc.render();
        // Verify label values appear in output
        assert!(output.contains("vlm_requests_total"));
        assert!(output.contains("vlm_degradations_total"));
        assert!(output.contains("gpt-4o"));
        assert!(output.contains("claude-3.5-sonnet"));
    }
}
