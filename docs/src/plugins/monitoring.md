# Monitoring Plugin

The `MonitoringPlugin` provides application observability through Prometheus-compatible metrics and a health check system with support for both liveness and readiness probes. It's designed for production deployments, particularly Kubernetes environments where health endpoints are essential.

The plugin is built on two subsystems: a metrics recorder that collects counters, gauges, and histograms in Prometheus text format, and a health check framework that aggregates results from multiple checks into a single status.

## Quick Start

```rust
use rust_boot::prelude::*;

// Configure metrics with a prefix and default labels
let config = MetricsConfig::new()
    .with_prefix("myapp")
    .with_label("env", "production");

// Register with the plugin system
let mut registry = PluginRegistry::new();
registry.register(MonitoringPlugin::new(config))?;
registry.init_all().await?;
registry.ready_all().await?;
```

## MetricsConfig

`MetricsConfig` controls how metrics are named and labeled. All metric names are automatically prefixed to avoid collisions with other systems.

```rust
let config = MetricsConfig::new();
```

| Field | Default | Description |
|---|---|---|
| `prefix` | `Some("rust_boot")` | Prefix prepended to all metric names (e.g., `rust_boot_http_requests_total`) |
| `default_labels` | `[]` | Labels applied to every metric automatically |
| `enable_process_metrics` | `true` | Whether to collect process-level metrics (CPU, memory) |

### Builder Methods

| Method | Description |
|---|---|
| `new()` | Creates a config with defaults |
| `with_prefix(impl Into<String>)` | Sets the metric name prefix |
| `without_prefix()` | Disables the prefix (metrics use bare names) |
| `with_label(key, value)` | Adds a default label to all metrics |
| `with_process_metrics(bool)` | Enables or disables process metrics |

```rust
let config = MetricsConfig::new()
    .with_prefix("api_gateway")
    .with_label("service", "user-service")
    .with_label("region", "us-east-1")
    .with_process_metrics(true);
```

## MetricsRecorder

`MetricsRecorder` is the core metrics engine. It wraps a Prometheus exporter and provides methods for recording counters, gauges, histograms, and HTTP request metrics.

The recorder is created during the plugin's `build()` phase and installed as the global metrics recorder during `ready()`.

### Recording Metrics

```rust
// Get the recorder from the plugin
let recorder = monitoring_plugin.recorder()
    .expect("recorder should be initialized after build()");

// Increment a counter
recorder.increment_counter("login_attempts");
// => myapp_login_attempts (counter)

// Set a gauge value
recorder.set_gauge("active_connections", 42.0);
// => myapp_active_connections (gauge)

// Record a histogram value
recorder.record_histogram("query_duration_ms", 12.5);
// => myapp_query_duration_ms (histogram)
```

### HTTP Request Metrics

The `record_request` method records both a counter and a histogram for HTTP requests, labeled by method, path, and status code:

```rust
recorder.record_request("GET", "/api/users", 200, 15.3);
// Increments: myapp_http_requests_total{method="GET", path="/api/users", status="200"}
// Records:    myapp_http_request_duration_ms{method="GET", path="/api/users", status="200"} = 15.3
```

### Timing Functions

The `time` method measures how long a closure takes and records the duration as a histogram:

```rust
let result = recorder.time("db_query_ms", || {
    // ... perform database query ...
    expensive_operation()
});
// Automatically records the elapsed time in milliseconds
```

### RequestTimer

For more control over HTTP request timing, use `RequestTimer`:

```rust
let timer = RequestTimer::new("GET", "/api/users");

// ... handle the request ...

// Finish timing and record metrics
timer.finish(&recorder, 200);

// Or check elapsed time without finishing
let elapsed = timer.elapsed_ms();
```

### Rendering Metrics

The `render()` method outputs all collected metrics in Prometheus text exposition format, ready to be served on a `/metrics` endpoint:

```rust
let output = recorder.render();
// Returns something like:
// # TYPE myapp_http_requests_total counter
// myapp_http_requests_total{method="GET",path="/api/users",status="200"} 42
// # TYPE myapp_http_request_duration_ms histogram
// ...
```

### Full MetricsRecorder API

| Method | Returns | Description |
|---|---|---|
| `new(config)` | `MetricsRecorder` | Creates a new recorder from config |
| `install()` | `Result<()>` | Installs as the global metrics recorder |
| `render()` | `String` | Renders all metrics in Prometheus text format |
| `record_request(method, path, status, duration_ms)` | `()` | Records HTTP request counter + histogram |
| `increment_counter(name)` | `()` | Increments a named counter by 1 |
| `set_gauge(name, value)` | `()` | Sets a gauge to the given value |
| `record_histogram(name, value)` | `()` | Records a value in a named histogram |
| `time(name, closure)` | `R` | Times a closure and records duration |

## Health Checks

The health check system provides a structured way to monitor the health of your application's dependencies (databases, caches, external services). It supports three health states and aggregates results from multiple checks into a single status.

### HealthState

```rust
pub enum HealthState {
    Healthy,    // Fully operational
    Degraded,   // Operational but with reduced capacity
    Unhealthy,  // Not operational
}
```

The aggregation logic is pessimistic: if any check is `Unhealthy`, the overall status is `Unhealthy`. If any check is `Degraded` (and none are `Unhealthy`), the overall status is `Degraded`. Only when all checks are `Healthy` is the overall status `Healthy`.

### HealthStatus

`HealthStatus` holds the aggregated state plus individual check results:

```rust
pub struct HealthStatus {
    pub status: HealthState,
    pub checks: HashMap<String, CheckResult>,
}
```

```rust
// Create programmatically
let status = HealthStatus::healthy()
    .with_check("database", CheckResult::healthy_with_latency(5))
    .with_check("cache", CheckResult::healthy())
    .with_check("redis", CheckResult::degraded("High latency"));

assert!(status.is_degraded()); // degraded because of redis

// Merge two statuses
let db_status = HealthStatus::healthy()
    .with_check("postgres", CheckResult::healthy());
let cache_status = HealthStatus::healthy()
    .with_check("moka", CheckResult::healthy());

let combined = db_status.merge(cache_status);
assert!(combined.is_healthy());
```

`HealthStatus` serializes to JSON for use in HTTP health endpoints:

```json
{
  "status": "degraded",
  "checks": {
    "database": { "status": "healthy", "latency_ms": 5 },
    "cache": { "status": "healthy" },
    "redis": { "status": "degraded", "message": "High latency" }
  }
}
```

### CheckResult

Individual check results carry a status, an optional message, and an optional latency measurement:

| Constructor | Description |
|---|---|
| `CheckResult::healthy()` | Healthy with no extra info |
| `CheckResult::healthy_with_latency(ms)` | Healthy with latency measurement |
| `CheckResult::degraded(message)` | Degraded with explanation |
| `CheckResult::unhealthy(message)` | Unhealthy with explanation |

### HealthCheck Trait

Implement `HealthCheck` to create custom health checks:

```rust
use async_trait::async_trait;

struct DatabaseHealthCheck {
    pool: Arc<DatabasePool>,
}

#[async_trait]
impl HealthCheck for DatabaseHealthCheck {
    fn name(&self) -> &str {
        "database"
    }

    async fn check(&self) -> HealthStatus {
        let start = std::time::Instant::now();
        match self.pool.ping().await {
            Ok(_) => {
                let latency = start.elapsed().as_millis() as u64;
                HealthStatus::healthy()
                    .with_check("database", CheckResult::healthy_with_latency(latency))
            }
            Err(e) => {
                HealthStatus::unhealthy(format!("Database ping failed: {e}"))
            }
        }
    }
}
```

### ReadinessCheck Trait

For Kubernetes readiness probes, implement `ReadinessCheck` in addition to `HealthCheck`. A readiness check determines whether the service should receive traffic — it might be healthy but not yet ready (e.g., still warming up caches).

```rust
#[async_trait]
impl HealthCheck for DatabaseHealthCheck {
    fn name(&self) -> &str { "database" }

    async fn check(&self) -> HealthStatus {
        // ... liveness check ...
        HealthStatus::healthy()
    }

    // Opt into readiness checking
    fn as_readiness(&self) -> Option<&dyn ReadinessCheck> {
        Some(self)
    }
}

#[async_trait]
impl ReadinessCheck for DatabaseHealthCheck {
    async fn check_ready(&self) -> HealthStatus {
        // Check if the connection pool has available connections
        if self.pool.available() > 0 {
            HealthStatus::healthy()
        } else {
            HealthStatus::unhealthy("No available database connections")
        }
    }
}
```

## MonitoringPlugin Lifecycle

The plugin registers with the name `"monitoring"` and version `"0.1.0"`. It has no dependencies on other plugins.

- **build()** — Creates a `MetricsRecorder` from the config and stores it as `Arc<MetricsRecorder>`.
- **ready()** — Installs the recorder as the global metrics recorder.
- **cleanup()** — Drops the recorder reference.

### Registering Health Checks

Add health checks to the plugin before registration using the builder pattern:

```rust
let plugin = MonitoringPlugin::new(MetricsConfig::new())
    .with_health_check(DatabaseHealthCheck::new(pool.clone()))
    .with_health_check(CacheHealthCheck::new(cache.clone()));

registry.register(plugin)?;
```

### Running Health Checks

After the plugin is initialized, run checks programmatically:

```rust
// Liveness: runs all registered health checks
let health = monitoring_plugin.check_health().await;
if health.is_healthy() {
    // All systems operational
}

// Readiness: only runs checks that implement ReadinessCheck
let readiness = monitoring_plugin.check_readiness().await;
if readiness.is_healthy() {
    // Ready to receive traffic
}
```

## Complete Example

```rust
use rust_boot::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Configure metrics
    let config = MetricsConfig::new()
        .with_prefix("myapp")
        .with_label("service", "api")
        .with_label("version", "1.0.0");

    // 2. Create plugin with health checks
    let plugin = MonitoringPlugin::new(config);
    // .with_health_check(your_health_check)

    // 3. Register and initialize
    let mut registry = PluginRegistry::new();
    registry.register(plugin)?;
    registry.init_all().await?;
    registry.ready_all().await?;

    // 4. Record some metrics (in request handlers, middleware, etc.)
    // let recorder = monitoring_plugin.recorder().unwrap();
    // recorder.record_request("GET", "/api/users", 200, 12.5);
    // recorder.increment_counter("cache_hits");
    // recorder.set_gauge("queue_depth", 7.0);

    // 5. Expose metrics endpoint
    // let metrics_output = recorder.render();
    // Serve metrics_output on GET /metrics

    // 6. Health endpoints
    // let health = monitoring_plugin.check_health().await;
    // Serve health as JSON on GET /health

    // 7. Shutdown
    registry.finish_all().await?;
    registry.cleanup_all().await?;

    Ok(())
}
```
