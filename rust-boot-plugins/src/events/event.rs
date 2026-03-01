//! Domain event types and traits.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Trait for domain events in event sourcing.
pub trait DomainEvent: Serialize + DeserializeOwned + Send + Sync + Clone {
    /// Returns the type name of this event.
    fn event_type(&self) -> &'static str;
    /// Returns the aggregate type this event belongs to.
    fn aggregate_type(&self) -> &'static str;
}

/// Metadata associated with a domain event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Unique identifier for this event.
    pub event_id: String,
    /// Identifier of the aggregate this event belongs to.
    pub aggregate_id: String,
    /// Type of aggregate this event belongs to.
    pub aggregate_type: String,
    /// Type name of this event.
    pub event_type: String,
    /// Version number of the event in the aggregate stream.
    pub version: u64,
    /// Unix timestamp in milliseconds when the event occurred.
    pub timestamp: u64,
    /// Optional correlation ID for distributed tracing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Optional causation ID linking to parent event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<String>,
    /// Optional ID of the user who triggered this event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl EventMetadata {
    /// Creates new event metadata with required fields.
    pub fn new(
        aggregate_id: impl Into<String>,
        aggregate_type: impl Into<String>,
        event_type: impl Into<String>,
        version: u64,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            event_id: Uuid::new_v4().to_string(),
            aggregate_id: aggregate_id.into(),
            aggregate_type: aggregate_type.into(),
            event_type: event_type.into(),
            version,
            timestamp,
            correlation_id: None,
            causation_id: None,
            user_id: None,
        }
    }

    /// Sets the correlation ID for distributed tracing.
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Sets the causation ID linking to a parent event.
    pub fn with_causation_id(mut self, id: impl Into<String>) -> Self {
        self.causation_id = Some(id.into());
        self
    }

    /// Sets the user ID who triggered this event.
    pub fn with_user_id(mut self, id: impl Into<String>) -> Self {
        self.user_id = Some(id.into());
        self
    }
}

/// Wrapper combining event metadata with event payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope<E> {
    /// Event metadata.
    pub metadata: EventMetadata,
    /// Event payload.
    pub payload: E,
}

impl<E: DomainEvent> EventEnvelope<E> {
    /// Creates a new event envelope with auto-generated metadata.
    pub fn new(aggregate_id: impl Into<String>, version: u64, event: E) -> Self {
        let metadata = EventMetadata::new(
            aggregate_id,
            event.aggregate_type(),
            event.event_type(),
            version,
        );

        Self {
            metadata,
            payload: event,
        }
    }

    /// Creates an event envelope with pre-built metadata.
    pub const fn with_metadata(metadata: EventMetadata, event: E) -> Self {
        Self {
            metadata,
            payload: event,
        }
    }
}

/// Generic CRUD events for entity lifecycle tracking.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrudEvent<T: Clone + Serialize> {
    /// Entity was created.
    Created {
        /// The created entity.
        entity: T,
    },
    /// Entity was updated.
    Updated {
        /// The updated entity.
        entity: T,
        /// List of changed field names.
        changes: Vec<String>,
    },
    /// Entity was deleted.
    Deleted {
        /// ID of the deleted entity.
        id: String,
    },
    /// Entity was restored from deletion.
    Restored {
        /// The restored entity.
        entity: T,
    },
}

impl<T: Clone + Serialize + DeserializeOwned + Send + Sync> DomainEvent for CrudEvent<T> {
    fn event_type(&self) -> &'static str {
        match self {
            Self::Created { .. } => "Created",
            Self::Updated { .. } => "Updated",
            Self::Deleted { .. } => "Deleted",
            Self::Restored { .. } => "Restored",
        }
    }

    fn aggregate_type(&self) -> &'static str {
        "Entity"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestEntity {
        id: String,
        name: String,
    }

    #[test]
    fn test_event_metadata_creation() {
        let metadata = EventMetadata::new("agg-123", "User", "UserCreated", 1);

        assert_eq!(metadata.aggregate_id, "agg-123");
        assert_eq!(metadata.aggregate_type, "User");
        assert_eq!(metadata.event_type, "UserCreated");
        assert_eq!(metadata.version, 1);
        assert!(!metadata.event_id.is_empty());
        assert!(metadata.timestamp > 0);
    }

    #[test]
    fn test_event_metadata_with_context() {
        let metadata = EventMetadata::new("agg-123", "User", "UserCreated", 1)
            .with_correlation_id("corr-456")
            .with_causation_id("cause-789")
            .with_user_id("user-001");

        assert_eq!(metadata.correlation_id, Some("corr-456".to_string()));
        assert_eq!(metadata.causation_id, Some("cause-789".to_string()));
        assert_eq!(metadata.user_id, Some("user-001".to_string()));
    }

    #[test]
    fn test_event_envelope_creation() {
        let entity = TestEntity {
            id: "1".to_string(),
            name: "Test".to_string(),
        };
        let event: CrudEvent<TestEntity> = CrudEvent::Created { entity };
        let envelope = EventEnvelope::new("agg-123", 1, event);

        assert_eq!(envelope.metadata.aggregate_id, "agg-123");
        assert_eq!(envelope.metadata.version, 1);
        assert_eq!(envelope.metadata.event_type, "Created");
    }

    #[test]
    fn test_crud_event_types() {
        let entity = TestEntity {
            id: "1".to_string(),
            name: "Test".to_string(),
        };

        let created: CrudEvent<TestEntity> = CrudEvent::Created {
            entity: entity.clone(),
        };
        assert_eq!(created.event_type(), "Created");

        let updated: CrudEvent<TestEntity> = CrudEvent::Updated {
            entity: entity.clone(),
            changes: vec!["name".to_string()],
        };
        assert_eq!(updated.event_type(), "Updated");

        let deleted: CrudEvent<TestEntity> = CrudEvent::Deleted {
            id: "1".to_string(),
        };
        assert_eq!(deleted.event_type(), "Deleted");

        let restored: CrudEvent<TestEntity> = CrudEvent::Restored { entity };
        assert_eq!(restored.event_type(), "Restored");
    }

    #[test]
    fn test_event_serialization() {
        let entity = TestEntity {
            id: "1".to_string(),
            name: "Test".to_string(),
        };
        let event: CrudEvent<TestEntity> = CrudEvent::Created { entity };
        let envelope = EventEnvelope::new("agg-123", 1, event);

        let json = serde_json::to_string(&envelope).unwrap();
        assert!(json.contains("Created"));
        assert!(json.contains("agg-123"));

        let deserialized: EventEnvelope<CrudEvent<TestEntity>> =
            serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.metadata.aggregate_id, "agg-123");
    }
}
