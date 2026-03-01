//! Built-in plugins providing caching, monitoring, authentication, and event-sourcing capabilities.

pub mod auth;
pub mod cache;
pub mod events;
pub mod monitoring;

pub use auth::{AuthPlugin, Claims, JwtConfig, JwtManager, Role};
pub use cache::{CacheBackend, CacheConfig, CachingPlugin, MokaBackend, RedisBackend};
pub use events::{
    DomainEvent, EventEnvelope, EventMetadata, EventSourcingPlugin, EventStore, InMemoryEventStore,
};
pub use monitoring::{
    HealthCheck, HealthStatus, MetricsConfig, MetricsRecorder, MonitoringPlugin, ReadinessCheck,
};
