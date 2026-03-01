//! Custom Plugin Example - Demonstrating How to Create rust-boot Plugins
//!
//! This example shows how to build custom plugins for the rust-boot framework.
//! It demonstrates:
//! - Implementing the `CrudPlugin` trait
//! - Plugin lifecycle hooks (build, ready, finish, cleanup)
//! - Plugin dependencies
//! - Using `PluginContext` to share state between plugins
//!
//! To run this example:
//! ```bash
//! cargo run --example custom_plugin
//! ```

#![allow(missing_docs)]

use async_trait::async_trait;
use rust_boot::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

fn main() {
    println!("=== Custom Plugin Example ===\n");

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if let Err(e) = run_example().await {
            eprintln!("Error: {e}");
        }
    });
}

async fn run_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. Creating custom plugins...\n");

    // Create our custom plugins
    let request_counter = RequestCounterPlugin::new();
    let audit_logger = AuditLoggerPlugin::new();
    // RateLimiter depends on RequestCounter, so it must be registered after
    let rate_limiter = RateLimiterPlugin::new(100);

    println!("2. Creating plugin registry and registering plugins...\n");

    let mut registry = PluginRegistry::new();

    // Register plugins in dependency order
    // RequestCounter has no dependencies - register first
    registry.register(request_counter)?;
    println!("   ✓ Registered RequestCounterPlugin");

    // AuditLogger has no dependencies
    registry.register(audit_logger)?;
    println!("   ✓ Registered AuditLoggerPlugin");

    // RateLimiter depends on RequestCounter - must be registered after
    registry.register(rate_limiter)?;
    println!("   ✓ Registered RateLimiterPlugin (depends on request-counter)");

    println!("\n3. Running plugin lifecycle...\n");

    // init_all() calls build() on all plugins in dependency order
    registry.init_all().await?;
    println!("   ✓ Init phase complete (build called on all plugins)");

    // ready_all() calls ready() on all plugins
    registry.ready_all().await?;
    println!("   ✓ Ready phase complete");

    println!("\n4. Accessing shared state from context...\n");

    // Plugins store shared state in the registry's context during build()
    let counter: Option<Arc<RequestCounter>> = registry.context().get("request_counter").await;
    let counter = counter.expect("Counter should be registered in context");

    // Simulate some requests
    for i in 1..=5 {
        counter.increment();
        println!("   Request #{}: Total count = {}", i, counter.count());
    }

    // Access the audit log
    let audit_log: Option<Arc<AuditLog>> = registry.context().get("audit_log").await;
    let audit_log = audit_log.expect("Audit log should be registered in context");

    println!("\n5. Logging audit events...\n");

    audit_log.log("user.login", "User alice logged in").await;
    audit_log
        .log("user.update", "User alice updated profile")
        .await;
    audit_log.log("user.logout", "User alice logged out").await;

    println!("   Logged 3 audit events");

    println!("\n6. Reviewing audit log...\n");
    let entries = audit_log.entries().await;
    for entry in entries {
        println!(
            "   [{}] {} - {}",
            entry.timestamp, entry.action, entry.details
        );
    }

    println!("\n7. Shutdown phases...\n");

    // finish_all() is called in reverse dependency order
    registry.finish_all().await?;
    println!("   ✓ Finish phase complete");

    // cleanup_all() is called in reverse dependency order
    registry.cleanup_all().await?;
    println!("   ✓ Cleanup phase complete");

    println!("\n=== Example Complete ===");

    Ok(())
}

// ============================================================================
// RequestCounter - A shared counter that tracks request count
// ============================================================================

/// Shared state for counting requests (stored in `PluginContext`)
pub struct RequestCounter {
    counter: AtomicU64,
    start_time: RwLock<Option<Instant>>,
}

impl RequestCounter {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
            start_time: RwLock::new(None),
        }
    }

    pub fn increment(&self) {
        self.counter.fetch_add(1, Ordering::SeqCst);
    }

    pub fn count(&self) -> u64 {
        self.counter.load(Ordering::SeqCst)
    }

    pub async fn start_timer(&self) {
        let mut start_time = self.start_time.write().await;
        *start_time = Some(Instant::now());
    }

    pub async fn uptime_secs(&self) -> Option<f64> {
        self.start_time
            .read()
            .await
            .map(|start| start.elapsed().as_secs_f64())
    }
}

impl Default for RequestCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin that provides request counting functionality
pub struct RequestCounterPlugin {
    /// Shared counter state that will be stored in `PluginContext`
    counter: Arc<RequestCounter>,
}

impl RequestCounterPlugin {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(RequestCounter::new()),
        }
    }
}

impl Default for RequestCounterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CrudPlugin for RequestCounterPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("request-counter", "1.0.0")
    }

    async fn build(&mut self, ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        println!("   [RequestCounter] Building plugin...");

        // Store the shared counter in the context so other plugins can access it
        ctx.insert("request_counter", self.counter.clone()).await;

        Ok(())
    }

    async fn ready(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        println!("   [RequestCounter] Plugin ready, starting timer...");

        // Start the uptime timer when the plugin becomes ready
        self.counter.start_timer().await;

        Ok(())
    }

    async fn finish(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        let count = self.counter.count();
        let uptime = self.counter.uptime_secs().await.unwrap_or(0.0);
        println!("   [RequestCounter] Finishing... Total requests: {count}, Uptime: {uptime:.2}s");
        Ok(())
    }

    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        println!("   [RequestCounter] Cleanup complete");
        Ok(())
    }
}

// ============================================================================
// AuditLog - A simple in-memory audit logging system
// ============================================================================

/// An audit log entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: String,
    pub action: String,
    pub details: String,
}

/// Shared audit log storage (stored in `PluginContext`)
pub struct AuditLog {
    entries: RwLock<Vec<AuditEntry>>,
    entry_count: AtomicU64,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            entry_count: AtomicU64::new(0),
        }
    }

    pub async fn log(&self, action: &str, details: &str) {
        let count = self.entry_count.fetch_add(1, Ordering::SeqCst);
        let entry = AuditEntry {
            // Simple timestamp format without external dependencies
            timestamp: format!("entry-{:04}", count + 1),
            action: action.to_string(),
            details: details.to_string(),
        };
        self.entries.write().await.push(entry);
    }

    pub async fn entries(&self) -> Vec<AuditEntry> {
        self.entries.read().await.clone()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin that provides audit logging functionality
pub struct AuditLoggerPlugin {
    log: Arc<AuditLog>,
}

impl AuditLoggerPlugin {
    pub fn new() -> Self {
        Self {
            log: Arc::new(AuditLog::new()),
        }
    }
}

impl Default for AuditLoggerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CrudPlugin for AuditLoggerPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("audit-logger", "1.0.0")
    }

    async fn build(&mut self, ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        println!("   [AuditLogger] Building plugin...");

        // Store the shared audit log in the context
        ctx.insert("audit_log", self.log.clone()).await;

        Ok(())
    }

    async fn ready(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        println!("   [AuditLogger] Plugin ready");
        self.log.log("system", "Audit logger initialized").await;
        Ok(())
    }

    async fn finish(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        let count = self.log.entries().await.len();
        println!("   [AuditLogger] Finishing... Total entries: {count}");
        Ok(())
    }

    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        println!("   [AuditLogger] Cleanup complete");
        Ok(())
    }
}

// ============================================================================
// RateLimiter - Demonstrates plugin dependencies
// ============================================================================

/// Plugin that provides rate limiting (depends on request-counter)
pub struct RateLimiterPlugin {
    max_requests_per_minute: u64,
    enabled: bool,
}

impl RateLimiterPlugin {
    pub const fn new(max_requests_per_minute: u64) -> Self {
        Self {
            max_requests_per_minute,
            enabled: false,
        }
    }

    #[allow(dead_code)]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[allow(dead_code)]
    pub const fn max_requests(&self) -> u64 {
        self.max_requests_per_minute
    }
}

#[async_trait]
impl CrudPlugin for RateLimiterPlugin {
    fn meta(&self) -> PluginMeta {
        // This plugin depends on request-counter
        // It will fail to register if request-counter isn't registered first
        PluginMeta::new("rate-limiter", "1.0.0").with_dependency("request-counter")
    }

    async fn build(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        println!(
            "   [RateLimiter] Building plugin (max: {} req/min)...",
            self.max_requests_per_minute
        );
        Ok(())
    }

    async fn ready(&mut self, ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        // Check if the request counter is available (it should be, since we depend on it)
        let counter: Option<Arc<RequestCounter>> = ctx.get("request_counter").await;
        if counter.is_some() {
            println!("   [RateLimiter] Found request counter, enabling rate limiting");
            self.enabled = true;
        } else {
            println!("   [RateLimiter] No request counter found, rate limiting disabled");
        }
        Ok(())
    }

    async fn finish(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        println!("   [RateLimiter] Finishing... Enabled: {}", self.enabled);
        Ok(())
    }

    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        self.enabled = false;
        println!("   [RateLimiter] Cleanup complete");
        Ok(())
    }
}
