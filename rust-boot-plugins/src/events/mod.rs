//! Event sourcing plugin with CQRS support.

mod event;
mod store;

pub use event::{DomainEvent, EventEnvelope, EventMetadata};
pub use store::{EventStore, InMemoryEventStore};

use async_trait::async_trait;
use rust_boot_core::{
    error::Result,
    plugin::{CrudPlugin, PluginContext, PluginMeta},
};
use std::sync::Arc;

/// Plugin for event sourcing with CQRS pattern support.
pub struct EventSourcingPlugin {
    store: Option<Arc<dyn EventStore>>,
}

impl EventSourcingPlugin {
    /// Creates a new event sourcing plugin without a store.
    pub fn new() -> Self {
        Self { store: None }
    }

    /// Configures the plugin with a custom event store.
    pub fn with_store<S: EventStore + 'static>(mut self, store: S) -> Self {
        self.store = Some(Arc::new(store));
        self
    }

    /// Returns the configured event store if available.
    pub fn store(&self) -> Option<Arc<dyn EventStore>> {
        self.store.clone()
    }
}

impl Default for EventSourcingPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CrudPlugin for EventSourcingPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("event-sourcing", "0.1.0")
    }

    async fn build(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        if self.store.is_none() {
            self.store = Some(Arc::new(InMemoryEventStore::new()));
        }
        Ok(())
    }

    async fn ready(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    async fn finish(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        self.store = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_sourcing_plugin_creation() {
        let plugin = EventSourcingPlugin::default();
        assert_eq!(plugin.meta().name, "event-sourcing");
    }

    #[tokio::test]
    async fn test_event_sourcing_plugin_store_none_initially() {
        let plugin = EventSourcingPlugin::new();
        assert!(plugin.store().is_none());
    }

    #[tokio::test]
    async fn test_event_sourcing_plugin_build_creates_default_store() {
        let mut plugin = EventSourcingPlugin::new();
        let mut ctx = PluginContext::new();

        plugin.build(&mut ctx).await.unwrap();
        assert!(plugin.store().is_some());
    }

    #[tokio::test]
    async fn test_event_sourcing_plugin_with_custom_store() {
        let store = InMemoryEventStore::new();
        let plugin = EventSourcingPlugin::new().with_store(store);
        assert!(plugin.store().is_some());
    }
}
