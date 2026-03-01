//! Health check abstractions for liveness and readiness probes.

use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;

/// Aggregated health status from multiple health checks.
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    /// Overall health state.
    pub status: HealthState,
    /// Individual check results by name.
    pub checks: HashMap<String, CheckResult>,
}

/// Represents the health state of a component or service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    /// Component is fully operational.
    Healthy,
    /// Component is operational but with reduced capacity.
    Degraded,
    /// Component is not operational.
    Unhealthy,
}

/// Result of an individual health check.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    /// Health state of this check.
    pub status: HealthState,
    /// Optional message describing the check result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Optional latency measurement in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

impl HealthStatus {
    /// Creates a healthy status with no checks.
    pub fn healthy() -> Self {
        Self {
            status: HealthState::Healthy,
            checks: HashMap::new(),
        }
    }

    /// Creates an unhealthy status with the given reason.
    pub fn unhealthy(reason: impl Into<String>) -> Self {
        let mut checks = HashMap::new();
        checks.insert(
            "default".to_string(),
            CheckResult {
                status: HealthState::Unhealthy,
                message: Some(reason.into()),
                latency_ms: None,
            },
        );
        Self {
            status: HealthState::Unhealthy,
            checks,
        }
    }

    /// Returns true if the overall status is healthy.
    pub fn is_healthy(&self) -> bool {
        self.status == HealthState::Healthy
    }

    /// Returns true if the overall status is degraded.
    pub fn is_degraded(&self) -> bool {
        self.status == HealthState::Degraded
    }

    /// Returns true if the overall status is unhealthy.
    pub fn is_unhealthy(&self) -> bool {
        self.status == HealthState::Unhealthy
    }

    /// Adds a named check result and recalculates overall status.
    pub fn with_check(mut self, name: impl Into<String>, result: CheckResult) -> Self {
        let name = name.into();
        self.checks.insert(name, result);
        self.recalculate_status();
        self
    }

    /// Merges another health status into this one.
    pub fn merge(mut self, other: HealthStatus) -> Self {
        for (name, result) in other.checks {
            self.checks.insert(name, result);
        }
        self.recalculate_status();
        self
    }

    fn recalculate_status(&mut self) {
        let mut has_unhealthy = false;
        let mut has_degraded = false;

        for result in self.checks.values() {
            match result.status {
                HealthState::Unhealthy => has_unhealthy = true,
                HealthState::Degraded => has_degraded = true,
                HealthState::Healthy => {}
            }
        }

        self.status = if has_unhealthy {
            HealthState::Unhealthy
        } else if has_degraded {
            HealthState::Degraded
        } else {
            HealthState::Healthy
        };
    }
}

impl CheckResult {
    /// Creates a healthy check result.
    pub fn healthy() -> Self {
        Self {
            status: HealthState::Healthy,
            message: None,
            latency_ms: None,
        }
    }

    /// Creates a healthy check result with latency measurement.
    pub fn healthy_with_latency(latency_ms: u64) -> Self {
        Self {
            status: HealthState::Healthy,
            message: None,
            latency_ms: Some(latency_ms),
        }
    }

    /// Creates a degraded check result with message.
    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: HealthState::Degraded,
            message: Some(message.into()),
            latency_ms: None,
        }
    }

    /// Creates an unhealthy check result with message.
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: HealthState::Unhealthy,
            message: Some(message.into()),
            latency_ms: None,
        }
    }
}

/// Trait for implementing health checks.
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// Returns the name of this health check.
    fn name(&self) -> &str;
    /// Performs the health check and returns status.
    async fn check(&self) -> HealthStatus;
    /// Returns this check as a readiness check if applicable.
    fn as_readiness(&self) -> Option<&dyn ReadinessCheck> {
        None
    }
}

/// Trait for implementing Kubernetes readiness probes.
#[async_trait]
pub trait ReadinessCheck: Send + Sync {
    /// Checks if the service is ready to receive traffic.
    async fn check_ready(&self) -> HealthStatus;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_healthy() {
        let status = HealthStatus::healthy();
        assert!(status.is_healthy());
        assert!(!status.is_degraded());
        assert!(!status.is_unhealthy());
    }

    #[test]
    fn test_health_status_unhealthy() {
        let status = HealthStatus::unhealthy("Database connection failed");
        assert!(status.is_unhealthy());
        assert!(!status.is_healthy());
    }

    #[test]
    fn test_health_status_with_check() {
        let status = HealthStatus::healthy()
            .with_check("database", CheckResult::healthy())
            .with_check("cache", CheckResult::healthy());

        assert!(status.is_healthy());
        assert_eq!(status.checks.len(), 2);
    }

    #[test]
    fn test_health_status_degraded_on_degraded_check() {
        let status = HealthStatus::healthy()
            .with_check("database", CheckResult::healthy())
            .with_check("cache", CheckResult::degraded("High latency"));

        assert!(status.is_degraded());
    }

    #[test]
    fn test_health_status_unhealthy_on_unhealthy_check() {
        let status = HealthStatus::healthy()
            .with_check("database", CheckResult::unhealthy("Connection refused"))
            .with_check("cache", CheckResult::healthy());

        assert!(status.is_unhealthy());
    }

    #[test]
    fn test_health_status_merge() {
        let status1 = HealthStatus::healthy().with_check("db", CheckResult::healthy());
        let status2 = HealthStatus::healthy().with_check("cache", CheckResult::healthy());

        let merged = status1.merge(status2);
        assert!(merged.is_healthy());
        assert_eq!(merged.checks.len(), 2);
    }

    #[test]
    fn test_check_result_healthy_with_latency() {
        let result = CheckResult::healthy_with_latency(42);
        assert_eq!(result.status, HealthState::Healthy);
        assert_eq!(result.latency_ms, Some(42));
    }

    #[test]
    fn test_health_state_serialization() {
        let status = HealthStatus::healthy();
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
    }
}
