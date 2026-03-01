//! Event store abstractions and implementations.

use async_trait::async_trait;
use rust_boot_core::error::{Result, RustBootError};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

/// Represents a persisted event in the event store.
#[derive(Debug, Clone)]
pub struct StoredEvent {
    /// Unique identifier for this event.
    pub event_id: String,
    /// Identifier of the aggregate this event belongs to.
    pub aggregate_id: String,
    /// Type of aggregate this event belongs to.
    pub aggregate_type: String,
    /// Type name of this event.
    pub event_type: String,
    /// Version number in the aggregate stream.
    pub version: u64,
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
    /// Serialized event payload.
    pub payload: Value,
    /// Serialized event metadata.
    pub metadata: Value,
}

/// Trait for event store implementations.
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Appends events to an aggregate's event stream.
    async fn append(&self, aggregate_id: &str, events: Vec<StoredEvent>) -> Result<()>;

    /// Loads all events for an aggregate.
    async fn load(&self, aggregate_id: &str) -> Result<Vec<StoredEvent>>;

    /// Loads events starting from a specific version.
    async fn load_from_version(
        &self,
        aggregate_id: &str,
        from_version: u64,
    ) -> Result<Vec<StoredEvent>>;

    /// Returns the latest version number for an aggregate.
    async fn get_latest_version(&self, aggregate_id: &str) -> Result<Option<u64>>;

    /// Loads all events for a given aggregate type.
    async fn load_all_by_type(&self, aggregate_type: &str) -> Result<Vec<StoredEvent>>;
}

/// In-memory event store for development and testing.
pub struct InMemoryEventStore {
    events: RwLock<HashMap<String, Vec<StoredEvent>>>,
}

impl InMemoryEventStore {
    /// Creates a new empty in-memory event store.
    pub fn new() -> Self {
        Self {
            events: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn append(&self, aggregate_id: &str, events: Vec<StoredEvent>) -> Result<()> {
        let mut store = self.events.write().map_err(|e| {
            RustBootError::Internal(format!("Failed to acquire write lock: {}", e))
        })?;

        let aggregate_events = store.entry(aggregate_id.to_string()).or_default();

        let current_version = aggregate_events.last().map(|e| e.version).unwrap_or(0);

        for event in events {
            if event.version != current_version + 1 {
                return Err(RustBootError::Internal(format!(
                    "Version conflict: expected {}, got {}",
                    current_version + 1,
                    event.version
                )));
            }
            aggregate_events.push(event);
        }

        Ok(())
    }

    async fn load(&self, aggregate_id: &str) -> Result<Vec<StoredEvent>> {
        let store = self.events.read().map_err(|e| {
            RustBootError::Internal(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(store.get(aggregate_id).cloned().unwrap_or_default())
    }

    async fn load_from_version(
        &self,
        aggregate_id: &str,
        from_version: u64,
    ) -> Result<Vec<StoredEvent>> {
        let store = self.events.read().map_err(|e| {
            RustBootError::Internal(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(store
            .get(aggregate_id)
            .map(|events| {
                events
                    .iter()
                    .filter(|e| e.version >= from_version)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn get_latest_version(&self, aggregate_id: &str) -> Result<Option<u64>> {
        let store = self.events.read().map_err(|e| {
            RustBootError::Internal(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(store
            .get(aggregate_id)
            .and_then(|events| events.last().map(|e| e.version)))
    }

    async fn load_all_by_type(&self, aggregate_type: &str) -> Result<Vec<StoredEvent>> {
        let store = self.events.read().map_err(|e| {
            RustBootError::Internal(format!("Failed to acquire read lock: {}", e))
        })?;

        let mut result = Vec::new();
        for events in store.values() {
            for event in events {
                if event.aggregate_type == aggregate_type {
                    result.push(event.clone());
                }
            }
        }

        result.sort_by_key(|e| e.timestamp);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_event(aggregate_id: &str, version: u64) -> StoredEvent {
        StoredEvent {
            event_id: format!("evt-{}", version),
            aggregate_id: aggregate_id.to_string(),
            aggregate_type: "TestAggregate".to_string(),
            event_type: "TestEvent".to_string(),
            version,
            timestamp: 1000 + version,
            payload: json!({"data": "test"}),
            metadata: json!({}),
        }
    }

    #[tokio::test]
    async fn test_in_memory_store_append_and_load() {
        let store = InMemoryEventStore::new();

        let event = create_test_event("agg-1", 1);
        store.append("agg-1", vec![event]).await.unwrap();

        let events = store.load("agg-1").await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].version, 1);
    }

    #[tokio::test]
    async fn test_in_memory_store_append_multiple() {
        let store = InMemoryEventStore::new();

        let event1 = create_test_event("agg-1", 1);
        store.append("agg-1", vec![event1]).await.unwrap();

        let event2 = create_test_event("agg-1", 2);
        store.append("agg-1", vec![event2]).await.unwrap();

        let events = store.load("agg-1").await.unwrap();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_in_memory_store_version_conflict() {
        let store = InMemoryEventStore::new();

        let event1 = create_test_event("agg-1", 1);
        store.append("agg-1", vec![event1]).await.unwrap();

        let event_wrong_version = create_test_event("agg-1", 5);
        let result = store.append("agg-1", vec![event_wrong_version]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_in_memory_store_load_from_version() {
        let store = InMemoryEventStore::new();

        for v in 1..=5 {
            let event = create_test_event("agg-1", v);
            store.append("agg-1", vec![event]).await.unwrap();
        }

        let events = store.load_from_version("agg-1", 3).await.unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].version, 3);
    }

    #[tokio::test]
    async fn test_in_memory_store_get_latest_version() {
        let store = InMemoryEventStore::new();

        let version = store.get_latest_version("agg-1").await.unwrap();
        assert!(version.is_none());

        for v in 1..=3 {
            let event = create_test_event("agg-1", v);
            store.append("agg-1", vec![event]).await.unwrap();
        }

        let version = store.get_latest_version("agg-1").await.unwrap();
        assert_eq!(version, Some(3));
    }

    #[tokio::test]
    async fn test_in_memory_store_load_nonexistent() {
        let store = InMemoryEventStore::new();
        let events = store.load("nonexistent").await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_store_load_all_by_type() {
        let store = InMemoryEventStore::new();

        let event1 = create_test_event("agg-1", 1);
        store.append("agg-1", vec![event1]).await.unwrap();

        let event2 = create_test_event("agg-2", 1);
        store.append("agg-2", vec![event2]).await.unwrap();

        let events = store.load_all_by_type("TestAggregate").await.unwrap();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_in_memory_store_isolation() {
        let store = InMemoryEventStore::new();

        let event1 = create_test_event("agg-1", 1);
        store.append("agg-1", vec![event1]).await.unwrap();

        let event2 = create_test_event("agg-2", 1);
        store.append("agg-2", vec![event2]).await.unwrap();

        let events1 = store.load("agg-1").await.unwrap();
        let events2 = store.load("agg-2").await.unwrap();

        assert_eq!(events1.len(), 1);
        assert_eq!(events2.len(), 1);
        assert_eq!(events1[0].aggregate_id, "agg-1");
        assert_eq!(events2[0].aggregate_id, "agg-2");
    }
}
