# Event Sourcing Plugin

The `EventSourcingPlugin` brings event sourcing and CQRS (Command Query Responsibility Segregation) patterns to rust-boot. Instead of storing only the current state of your entities, event sourcing captures every change as an immutable domain event. The full history of an aggregate can be reconstructed by replaying its events.

The plugin provides three layers: a `DomainEvent` trait for defining your events, an `EventEnvelope` that pairs events with rich metadata, and an `EventStore` trait with an in-memory implementation for development and testing.

## Quick Start

```rust
use rust_boot::prelude::*;

// Create the plugin (defaults to InMemoryEventStore)
let mut registry = PluginRegistry::new();
registry.register(EventSourcingPlugin::new())?;
registry.init_all().await?;

// Access the event store
let store = registry
    .context()
    .get::<Arc<dyn EventStore>>("event_store")
    .await
    .unwrap_or_else(|| {
        // Or get it directly from the plugin
        Arc::new(InMemoryEventStore::new())
    });
```

## Core Concepts

### Domain Events

A domain event represents something that happened in your system. Events are immutable facts — once recorded, they never change. Every event knows its type name and the aggregate it belongs to.

Implement the `DomainEvent` trait for your event types:

```rust
use rust_boot::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum UserEvent {
    Created { name: String, email: String },
    EmailChanged { old_email: String, new_email: String },
    Deactivated { reason: String },
}

impl DomainEvent for UserEvent {
    fn event_type(&self) -> &'static str {
        match self {
            Self::Created { .. } => "UserCreated",
            Self::EmailChanged { .. } => "UserEmailChanged",
            Self::Deactivated { .. } => "UserDeactivated",
        }
    }

    fn aggregate_type(&self) -> &'static str {
        "User"
    }
}
```

The `DomainEvent` trait requires `Serialize + DeserializeOwned + Send + Sync + Clone`, so your events must be fully serializable.

### Built-in CrudEvent

For common CRUD operations, rust-boot provides a generic `CrudEvent<T>` enum that covers the typical entity lifecycle:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrudEvent<T: Clone + Serialize> {
    Created { entity: T },
    Updated { entity: T, changes: Vec<String> },
    Deleted { id: String },
    Restored { entity: T },
}
```

`CrudEvent<T>` automatically implements `DomainEvent` with event types `"Created"`, `"Updated"`, `"Deleted"`, and `"Restored"`, and an aggregate type of `"Entity"`.

```rust
#[derive(Clone, Serialize, Deserialize)]
struct Product { id: String, name: String, price: f64 }

let event: CrudEvent<Product> = CrudEvent::Created {
    entity: Product {
        id: "prod-1".to_string(),
        name: "Widget".to_string(),
        price: 9.99,
    },
};

assert_eq!(event.event_type(), "Created");
assert_eq!(event.aggregate_type(), "Entity");
```

## EventMetadata

Every event is wrapped with metadata that provides context for auditing, debugging, and distributed tracing:

```rust
pub struct EventMetadata {
    pub event_id: String,        // Auto-generated UUID
    pub aggregate_id: String,    // ID of the aggregate this event belongs to
    pub aggregate_type: String,  // Type name of the aggregate
    pub event_type: String,      // Type name of the event
    pub version: u64,            // Sequential version in the aggregate stream
    pub timestamp: u64,          // Unix timestamp in milliseconds
    pub correlation_id: Option<String>,  // For distributed tracing
    pub causation_id: Option<String>,    // Links to the parent event
    pub user_id: Option<String>,         // Who triggered this event
}
```

### Creating Metadata

```rust
let metadata = EventMetadata::new(
    "user-123",       // aggregate_id
    "User",           // aggregate_type
    "UserCreated",    // event_type
    1,                // version
);
// event_id and timestamp are auto-generated

// Add tracing context
let metadata = EventMetadata::new("user-123", "User", "UserCreated", 1)
    .with_correlation_id("req-abc-456")
    .with_causation_id("cmd-create-user")
    .with_user_id("admin-001");
```

| Method | Description |
|---|---|
| `new(aggregate_id, aggregate_type, event_type, version)` | Creates metadata with auto-generated UUID and timestamp |
| `with_correlation_id(id)` | Sets the correlation ID for distributed tracing |
| `with_causation_id(id)` | Sets the causation ID linking to a parent event |
| `with_user_id(id)` | Sets the ID of the user who triggered the event |

## EventEnvelope

`EventEnvelope<E>` pairs an event payload with its metadata. This is the unit that gets stored and retrieved from the event store.

```rust
pub struct EventEnvelope<E> {
    pub metadata: EventMetadata,
    pub payload: E,
}
```

### Creating Envelopes

```rust
// Auto-generate metadata from the event
let event = UserEvent::Created {
    name: "Alice".to_string(),
    email: "alice@example.com".to_string(),
};
let envelope = EventEnvelope::new("user-123", 1, event);
// metadata.aggregate_type and event_type are derived from the DomainEvent trait

// Or provide pre-built metadata
let metadata = EventMetadata::new("user-123", "User", "UserCreated", 1)
    .with_user_id("admin");
let envelope = EventEnvelope::with_metadata(metadata, event);
```

Envelopes are fully serializable — they implement `Serialize` and `Deserialize`, so you can store them as JSON:

```rust
let json = serde_json::to_string(&envelope)?;
let deserialized: EventEnvelope<UserEvent> = serde_json::from_str(&json)?;
```

## EventStore Trait

The `EventStore` trait defines the interface for persisting and loading events:

```rust
#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, aggregate_id: &str, events: Vec<StoredEvent>) -> Result<()>;
    async fn load(&self, aggregate_id: &str) -> Result<Vec<StoredEvent>>;
    async fn load_from_version(&self, aggregate_id: &str, from_version: u64) -> Result<Vec<StoredEvent>>;
    async fn get_latest_version(&self, aggregate_id: &str) -> Result<Option<u64>>;
    async fn load_all_by_type(&self, aggregate_type: &str) -> Result<Vec<StoredEvent>>;
}
```

| Method | Description |
|---|---|
| `append(aggregate_id, events)` | Appends events to an aggregate's stream with version checking |
| `load(aggregate_id)` | Loads all events for an aggregate |
| `load_from_version(aggregate_id, version)` | Loads events starting from a specific version |
| `get_latest_version(aggregate_id)` | Returns the latest version number, or `None` if no events exist |
| `load_all_by_type(aggregate_type)` | Loads all events across aggregates of a given type, sorted by timestamp |

### StoredEvent

Events in the store are represented as `StoredEvent`, which uses `serde_json::Value` for the payload and metadata to allow storage without knowing the concrete event type:

```rust
pub struct StoredEvent {
    pub event_id: String,
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub event_type: String,
    pub version: u64,
    pub timestamp: u64,
    pub payload: serde_json::Value,
    pub metadata: serde_json::Value,
}
```

## InMemoryEventStore

The built-in `InMemoryEventStore` is a thread-safe, in-memory implementation suitable for development, testing, and prototyping. It stores events in a `RwLock<HashMap<String, Vec<StoredEvent>>>`, keyed by aggregate ID.

```rust
let store = InMemoryEventStore::new();
```

### Version Checking

The in-memory store enforces strict sequential versioning. When you append events, each event's version must be exactly `current_version + 1`. This prevents concurrent writes from corrupting the event stream:

```rust
let store = InMemoryEventStore::new();

// First event must be version 1
let event1 = StoredEvent { version: 1, /* ... */ };
store.append("order-1", vec![event1]).await?; // OK

// Next event must be version 2
let event2 = StoredEvent { version: 2, /* ... */ };
store.append("order-1", vec![event2]).await?; // OK

// Skipping versions fails
let event5 = StoredEvent { version: 5, /* ... */ };
store.append("order-1", vec![event5]).await; // Error: "Version conflict: expected 3, got 5"
```

### Querying Events

```rust
// Load all events for an aggregate
let events = store.load("order-1").await?;

// Load events from version 3 onwards (useful for snapshots)
let recent = store.load_from_version("order-1", 3).await?;

// Get the latest version number
let version = store.get_latest_version("order-1").await?;
// => Some(2)

// Load all events for a type across all aggregates
let all_order_events = store.load_all_by_type("Order").await?;
// Results are sorted by timestamp
```

## EventSourcingPlugin Lifecycle

The plugin registers with the name `"event-sourcing"` and version `"0.1.0"`. It has no dependencies.

- **build()** — If no store was provided via `with_store()`, creates an `InMemoryEventStore` as the default.
- **cleanup()** — Drops the store reference.

### Custom Event Store

Provide your own `EventStore` implementation for production use (e.g., backed by PostgreSQL, DynamoDB, or EventStoreDB):

```rust
let plugin = EventSourcingPlugin::new()
    .with_store(PostgresEventStore::new(pool));

// Or use the default in-memory store
let plugin = EventSourcingPlugin::new();
```

## Complete Example

```rust
use rust_boot::prelude::*;
use serde::{Serialize, Deserialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum OrderEvent {
    Placed { items: Vec<String>, total: f64 },
    Shipped { tracking_number: String },
    Delivered,
}

impl DomainEvent for OrderEvent {
    fn event_type(&self) -> &'static str {
        match self {
            Self::Placed { .. } => "OrderPlaced",
            Self::Shipped { .. } => "OrderShipped",
            Self::Delivered => "OrderDelivered",
        }
    }
    fn aggregate_type(&self) -> &'static str { "Order" }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Set up the plugin
    let mut registry = PluginRegistry::new();
    registry.register(EventSourcingPlugin::new())?;
    registry.init_all().await?;

    // 2. Create an event store
    let store = InMemoryEventStore::new();

    // 3. Record events for an order
    let placed = StoredEvent {
        event_id: "evt-1".to_string(),
        aggregate_id: "order-100".to_string(),
        aggregate_type: "Order".to_string(),
        event_type: "OrderPlaced".to_string(),
        version: 1,
        timestamp: 1700000000000,
        payload: json!({"items": ["Widget", "Gadget"], "total": 49.98}),
        metadata: json!({"user_id": "customer-42"}),
    };
    store.append("order-100", vec![placed]).await?;

    let shipped = StoredEvent {
        event_id: "evt-2".to_string(),
        aggregate_id: "order-100".to_string(),
        aggregate_type: "Order".to_string(),
        event_type: "OrderShipped".to_string(),
        version: 2,
        timestamp: 1700000060000,
        payload: json!({"tracking_number": "TRACK-123"}),
        metadata: json!({}),
    };
    store.append("order-100", vec![shipped]).await?;

    // 4. Replay the event stream
    let events = store.load("order-100").await?;
    for event in &events {
        println!("v{}: {} at {}", event.version, event.event_type, event.timestamp);
    }
    // v1: OrderPlaced at 1700000000000
    // v2: OrderShipped at 1700000060000

    // 5. Query by version
    let latest = store.get_latest_version("order-100").await?;
    assert_eq!(latest, Some(2));

    // 6. Cleanup
    registry.finish_all().await?;
    registry.cleanup_all().await?;

    Ok(())
}
```
