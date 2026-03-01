//! Monitoring plugin with Prometheus metrics and health checks.

mod health;
mod metrics;

pub use health::{HealthCheck, HealthStatus, ReadinessCheck};
pub use metrics::{MetricsConfig, MetricsRecorder};

use async_trait::async_trait;
use rust_boot_core::{
    error::Result,
    plugin::{CrudPlugin, PluginContext, PluginMeta},
};
use std::sync::Arc;

/// Plugin for application monitoring with Prometheus metrics and health checks.
///
/// Provides observability features including metrics recording, health checks,
/// and readiness probes for Kubernetes-style deployments.
pub struct MonitoringPlugin {
    /// Metrics configuration.
    config: MetricsConfig,
    /// Optional metrics recorder instance.
    recorder: Option<Arc<MetricsRecorder>>,
    /// Registered health checks.
    health_checks: Vec<Box<dyn HealthCheck>>,
}

impl MonitoringPlugin {
    /// Creates a new monitoring plugin with the given configuration.
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            config,
            recorder: None,
            health_checks: Vec::new(),
        }
    }

    /// Adds a health check to this plugin.
    pub fn with_health_check<H: HealthCheck + 'static>(mut self, check: H) -> Self {
        self.health_checks.push(Box::new(check));
        self
    }

    /// Returns the metrics recorder if initialized.
    pub fn recorder(&self) -> Option<Arc<MetricsRecorder>> {
        self.recorder.clone()
    }

    /// Runs all registered health checks and returns aggregated status.
    pub async fn check_health(&self) -> HealthStatus {
        let mut status = HealthStatus::healthy();

        for check in &self.health_checks {
            let result = check.check().await;
            status = status.merge(result);
        }

        status
    }

    /// Runs readiness checks for Kubernetes readiness probes.
    pub async fn check_readiness(&self) -> HealthStatus {
        let mut status = HealthStatus::healthy();

        for check in &self.health_checks {
            if let Some(readiness) = check.as_readiness() {
                let result = readiness.check_ready().await;
                status = status.merge(result);
            }
        }

        status
    }
}

impl Default for MonitoringPlugin {
    fn default() -> Self {
        Self::new(MetricsConfig::default())
    }
}

#[async_trait]
impl CrudPlugin for MonitoringPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("monitoring", "0.1.0")
    }

    async fn build(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        let recorder = MetricsRecorder::new(&self.config);
        self.recorder = Some(Arc::new(recorder));
        Ok(())
    }

    async fn ready(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        if let Some(recorder) = &self.recorder {
            recorder.install()?;
        }
        Ok(())
    }

    async fn finish(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        self.recorder = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_plugin_creation() {
        let plugin = MonitoringPlugin::default();
        assert_eq!(plugin.meta().name, "monitoring");
    }

    #[tokio::test]
    async fn test_monitoring_plugin_recorder_none_initially() {
        let plugin = MonitoringPlugin::default();
        assert!(plugin.recorder().is_none());
    }

    #[tokio::test]
    async fn test_health_check_empty() {
        let plugin = MonitoringPlugin::default();
        let status = plugin.check_health().await;
        assert!(status.is_healthy());
    }
}
