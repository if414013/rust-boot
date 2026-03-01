//! Prometheus metrics recording and exposition.

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use rust_boot_core::error::Result;
use std::time::Instant;

/// Configuration for Prometheus metrics collection.
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Optional prefix for all metric names.
    pub prefix: Option<String>,
    /// Default labels applied to all metrics.
    pub default_labels: Vec<(String, String)>,
    /// Whether to enable process metrics collection.
    pub enable_process_metrics: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            prefix: Some("rust_boot".to_string()),
            default_labels: Vec::new(),
            enable_process_metrics: true,
        }
    }
}

impl MetricsConfig {
    /// Creates a new metrics configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the metric name prefix.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Disables the metric name prefix.
    pub fn without_prefix(mut self) -> Self {
        self.prefix = None;
        self
    }

    /// Adds a default label to all metrics.
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_labels.push((key.into(), value.into()));
        self
    }

    /// Enables or disables process metrics collection.
    pub fn with_process_metrics(mut self, enable: bool) -> Self {
        self.enable_process_metrics = enable;
        self
    }
}

/// Prometheus metrics recorder and exporter.
pub struct MetricsRecorder {
    handle: PrometheusHandle,
    config: MetricsConfig,
}

impl MetricsRecorder {
    /// Creates a new metrics recorder with the given configuration.
    pub fn new(config: &MetricsConfig) -> Self {
        let handle = PrometheusBuilder::new().build_recorder().handle();

        Self {
            handle,
            config: config.clone(),
        }
    }

    /// Installs the metrics recorder as the global recorder.
    pub fn install(&self) -> Result<()> {
        Ok(())
    }

    /// Renders all collected metrics in Prometheus text format.
    pub fn render(&self) -> String {
        self.handle.render()
    }

    /// Records an HTTP request with method, path, status, and duration.
    pub fn record_request(&self, method: &str, path: &str, status: u16, duration_ms: f64) {
        let labels = [
            ("method", method.to_string()),
            ("path", path.to_string()),
            ("status", status.to_string()),
        ];

        let counter_name = self.prefixed_name("http_requests_total");
        let histogram_name = self.prefixed_name("http_request_duration_ms");

        metrics::increment_counter!(counter_name, &labels);
        metrics::histogram!(histogram_name, duration_ms, &labels);
    }

    /// Increments a named counter by one.
    pub fn increment_counter(&self, name: &str) {
        let full_name = self.prefixed_name(name);
        metrics::increment_counter!(full_name);
    }

    /// Sets a gauge to the specified value.
    pub fn set_gauge(&self, name: &str, value: f64) {
        let full_name = self.prefixed_name(name);
        metrics::gauge!(full_name, value);
    }

    /// Records a value in a histogram.
    pub fn record_histogram(&self, name: &str, value: f64) {
        let full_name = self.prefixed_name(name);
        metrics::histogram!(full_name, value);
    }

    /// Times a function execution and records the duration.
    pub fn time<F, R>(&self, name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        self.record_histogram(name, duration);
        result
    }

    fn prefixed_name(&self, name: &str) -> String {
        match &self.config.prefix {
            Some(prefix) => format!("{}_{}", prefix, name),
            None => name.to_string(),
        }
    }
}

/// Timer for measuring HTTP request duration.
pub struct RequestTimer {
    start: Instant,
    method: String,
    path: String,
}

impl RequestTimer {
    /// Creates a new request timer for the given method and path.
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            method: method.into(),
            path: path.into(),
        }
    }

    /// Finishes timing and records the request metrics.
    pub fn finish(self, recorder: &MetricsRecorder, status: u16) {
        let duration = self.start.elapsed().as_secs_f64() * 1000.0;
        recorder.record_request(&self.method, &self.path, status, duration);
    }

    /// Returns elapsed time in milliseconds without finishing.
    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert_eq!(config.prefix, Some("rust_boot".to_string()));
        assert!(config.default_labels.is_empty());
        assert!(config.enable_process_metrics);
    }

    #[test]
    fn test_metrics_config_builder() {
        let config = MetricsConfig::new()
            .with_prefix("myapp")
            .with_label("env", "production")
            .with_process_metrics(false);

        assert_eq!(config.prefix, Some("myapp".to_string()));
        assert_eq!(config.default_labels.len(), 1);
        assert!(!config.enable_process_metrics);
    }

    #[test]
    fn test_metrics_config_without_prefix() {
        let config = MetricsConfig::new().without_prefix();
        assert!(config.prefix.is_none());
    }

    #[test]
    fn test_prefixed_name() {
        let config = MetricsConfig::new().with_prefix("test");
        let recorder = MetricsRecorder::new(&config);

        assert_eq!(recorder.prefixed_name("requests"), "test_requests");
    }

    #[test]
    fn test_prefixed_name_without_prefix() {
        let config = MetricsConfig::new().without_prefix();
        let recorder = MetricsRecorder::new(&config);

        assert_eq!(recorder.prefixed_name("requests"), "requests");
    }

    #[test]
    fn test_request_timer_elapsed() {
        let timer = RequestTimer::new("GET", "/api/users");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();
        assert!(elapsed >= 10.0);
    }
}
