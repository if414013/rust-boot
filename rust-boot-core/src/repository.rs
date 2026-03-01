//! Repository layer traits for database operations with transaction support.
//!
//! This module defines the core repository abstractions for data persistence,
//! following the Repository pattern to decouple business logic from data access.

use async_trait::async_trait;

use crate::error::Result;
use crate::service::Filter;

/// Represents a database transaction.
///
/// Implementations must ensure ACID properties for operations performed
/// within the transaction scope.
#[async_trait]
pub trait Transaction: Send + Sync {
    /// Commits all changes made within this transaction.
    async fn commit(self: Box<Self>) -> Result<()>;

    /// Rolls back all changes made within this transaction.
    async fn rollback(self: Box<Self>) -> Result<()>;
}

/// Represents a database connection that can execute queries and manage transactions.
#[async_trait]
pub trait DatabaseConnection: Send + Sync {
    /// Begins a new transaction on this connection.
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>>;

    /// Executes a raw query and returns the number of affected rows.
    async fn execute(&self, query: &str) -> Result<u64>;

    /// Returns `true` if the connection is currently active.
    fn is_connected(&self) -> bool;
}

/// Generic CRUD repository trait for entity persistence.
///
/// This trait provides a standard interface for Create, Read, Update, and Delete
/// operations on entities. Implementations handle the actual database interactions.
#[async_trait]
pub trait CrudRepository: Send + Sync {
    /// The entity type managed by this repository.
    type Entity: Send + Sync;
    /// The type used for entity identifiers.
    type Id: Send + Sync;
    /// The database connection type used by this repository.
    type Connection: DatabaseConnection;

    /// Returns a reference to the underlying database connection.
    fn connection(&self) -> &Self::Connection;

    /// Inserts a new entity and returns the created entity with its assigned ID.
    async fn insert(&self, entity: &Self::Entity) -> Result<Self::Entity>;

    /// Finds an entity by its ID, returning `None` if not found.
    async fn find_by_id(&self, id: Self::Id) -> Result<Option<Self::Entity>>;

    /// Retrieves a paginated list of all entities.
    async fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Self::Entity>>;

    /// Retrieves a filtered and paginated list of entities.
    async fn find_with_filter(
        &self,
        filter: &dyn Filter,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Self::Entity>>;

    /// Updates an existing entity and returns the updated version.
    async fn update(&self, entity: &Self::Entity) -> Result<Self::Entity>;

    /// Deletes an entity by its ID.
    async fn delete(&self, id: Self::Id) -> Result<()>;

    /// Returns the total count of entities.
    async fn count(&self) -> Result<u64>;

    /// Returns the count of entities matching the filter.
    async fn count_with_filter(&self, filter: &dyn Filter) -> Result<u64>;

    /// Returns `true` if an entity with the given ID exists.
    async fn exists(&self, id: Self::Id) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::RustBootError;
    use crate::service::{FilterOp, NoFilter};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    struct MockTransaction {
        committed: Arc<AtomicBool>,
        rolled_back: Arc<AtomicBool>,
    }

    #[async_trait]
    impl Transaction for MockTransaction {
        async fn commit(self: Box<Self>) -> Result<()> {
            self.committed.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn rollback(self: Box<Self>) -> Result<()> {
            self.rolled_back.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    struct MockConnection {
        connected: AtomicBool,
        last_committed: Arc<AtomicBool>,
        last_rolled_back: Arc<AtomicBool>,
    }

    impl MockConnection {
        fn new() -> Self {
            Self {
                connected: AtomicBool::new(true),
                last_committed: Arc::new(AtomicBool::new(false)),
                last_rolled_back: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    #[async_trait]
    impl DatabaseConnection for MockConnection {
        async fn begin_transaction(&self) -> Result<Box<dyn Transaction>> {
            self.last_committed.store(false, Ordering::SeqCst);
            self.last_rolled_back.store(false, Ordering::SeqCst);
            Ok(Box::new(MockTransaction {
                committed: self.last_committed.clone(),
                rolled_back: self.last_rolled_back.clone(),
            }))
        }

        async fn execute(&self, _query: &str) -> Result<u64> {
            Ok(1)
        }

        fn is_connected(&self) -> bool {
            self.connected.load(Ordering::SeqCst)
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TestEntity {
        id: u64,
        name: String,
    }

    struct TestFilter {
        conditions: HashMap<String, FilterOp>,
    }

    impl Filter for TestFilter {
        fn apply(&self, field: &str) -> Option<FilterOp> {
            self.conditions.get(field).cloned()
        }

        fn fields(&self) -> Vec<String> {
            self.conditions.keys().cloned().collect()
        }
    }

    struct MockRepository {
        connection: MockConnection,
        entities: Arc<RwLock<Vec<TestEntity>>>,
        next_id: AtomicU64,
    }

    impl MockRepository {
        fn new() -> Self {
            Self {
                connection: MockConnection::new(),
                entities: Arc::new(RwLock::new(Vec::new())),
                next_id: AtomicU64::new(1),
            }
        }
    }

    #[async_trait]
    impl CrudRepository for MockRepository {
        type Entity = TestEntity;
        type Id = u64;
        type Connection = MockConnection;

        fn connection(&self) -> &Self::Connection {
            &self.connection
        }

        async fn insert(&self, entity: &Self::Entity) -> Result<Self::Entity> {
            let id = self.next_id.fetch_add(1, Ordering::SeqCst);
            let new_entity = TestEntity {
                id,
                name: entity.name.clone(),
            };
            let mut entities = self.entities.write().await;
            entities.push(new_entity.clone());
            Ok(new_entity)
        }

        async fn find_by_id(&self, id: Self::Id) -> Result<Option<Self::Entity>> {
            let entities = self.entities.read().await;
            Ok(entities.iter().find(|e| e.id == id).cloned())
        }

        async fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Self::Entity>> {
            let entities = self.entities.read().await;
            Ok(entities.iter().skip(offset).take(limit).cloned().collect())
        }

        async fn find_with_filter(
            &self,
            filter: &dyn Filter,
            limit: usize,
            offset: usize,
        ) -> Result<Vec<Self::Entity>> {
            let entities = self.entities.read().await;
            let filtered: Vec<_> = entities
                .iter()
                .filter(|e| {
                    if let Some(FilterOp::Eq(val)) = filter.apply("name") {
                        e.name == val
                    } else {
                        true
                    }
                })
                .skip(offset)
                .take(limit)
                .cloned()
                .collect();
            Ok(filtered)
        }

        async fn update(&self, entity: &Self::Entity) -> Result<Self::Entity> {
            let mut entities = self.entities.write().await;
            if let Some(e) = entities.iter_mut().find(|e| e.id == entity.id) {
                e.name = entity.name.clone();
                Ok(e.clone())
            } else {
                Err(RustBootError::Database("Entity not found".to_string()))
            }
        }

        async fn delete(&self, id: Self::Id) -> Result<()> {
            let mut entities = self.entities.write().await;
            entities.retain(|e| e.id != id);
            Ok(())
        }

        async fn count(&self) -> Result<u64> {
            let entities = self.entities.read().await;
            Ok(entities.len() as u64)
        }

        async fn count_with_filter(&self, filter: &dyn Filter) -> Result<u64> {
            let entities = self.entities.read().await;
            let count = entities
                .iter()
                .filter(|e| {
                    if let Some(FilterOp::Eq(val)) = filter.apply("name") {
                        e.name == val
                    } else {
                        true
                    }
                })
                .count();
            Ok(count as u64)
        }

        async fn exists(&self, id: Self::Id) -> Result<bool> {
            let entities = self.entities.read().await;
            Ok(entities.iter().any(|e| e.id == id))
        }
    }

    #[tokio::test]
    async fn test_mock_connection_is_connected() {
        let conn = MockConnection::new();
        assert!(conn.is_connected());
    }

    #[tokio::test]
    async fn test_mock_connection_execute() {
        let conn = MockConnection::new();
        let result = conn.execute("SELECT 1").await.unwrap();
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        let conn = MockConnection::new();
        let tx = conn.begin_transaction().await.unwrap();
        tx.commit().await.unwrap();
        assert!(conn.last_committed.load(Ordering::SeqCst));
        assert!(!conn.last_rolled_back.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let conn = MockConnection::new();
        let tx = conn.begin_transaction().await.unwrap();
        tx.rollback().await.unwrap();
        assert!(!conn.last_committed.load(Ordering::SeqCst));
        assert!(conn.last_rolled_back.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_repository_insert() {
        let repo = MockRepository::new();
        let entity = TestEntity {
            id: 0,
            name: "Test".to_string(),
        };

        let inserted = repo.insert(&entity).await.unwrap();
        assert_eq!(inserted.id, 1);
        assert_eq!(inserted.name, "Test");
    }

    #[tokio::test]
    async fn test_repository_find_by_id() {
        let repo = MockRepository::new();
        let entity = TestEntity {
            id: 0,
            name: "Test".to_string(),
        };
        repo.insert(&entity).await.unwrap();

        let found = repo.find_by_id(1).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test");

        let not_found = repo.find_by_id(999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_repository_find_all() {
        let repo = MockRepository::new();
        for i in 0..5 {
            repo.insert(&TestEntity {
                id: 0,
                name: format!("Entity {i}"),
            })
            .await
            .unwrap();
        }

        let all = repo.find_all(10, 0).await.unwrap();
        assert_eq!(all.len(), 5);

        let paginated = repo.find_all(2, 2).await.unwrap();
        assert_eq!(paginated.len(), 2);
        assert_eq!(paginated[0].name, "Entity 2");
    }

    #[tokio::test]
    async fn test_repository_find_with_filter() {
        let repo = MockRepository::new();
        repo.insert(&TestEntity {
            id: 0,
            name: "Alice".to_string(),
        })
        .await
        .unwrap();
        repo.insert(&TestEntity {
            id: 0,
            name: "Bob".to_string(),
        })
        .await
        .unwrap();

        let mut conditions = HashMap::new();
        conditions.insert("name".to_string(), FilterOp::Eq("Alice".to_string()));
        let filter = TestFilter { conditions };

        let filtered = repo.find_with_filter(&filter, 10, 0).await.unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Alice");
    }

    #[tokio::test]
    async fn test_repository_update() {
        let repo = MockRepository::new();
        repo.insert(&TestEntity {
            id: 0,
            name: "Original".to_string(),
        })
        .await
        .unwrap();

        let updated = repo
            .update(&TestEntity {
                id: 1,
                name: "Updated".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(updated.name, "Updated");
    }

    #[tokio::test]
    async fn test_repository_delete() {
        let repo = MockRepository::new();
        repo.insert(&TestEntity {
            id: 0,
            name: "Test".to_string(),
        })
        .await
        .unwrap();

        repo.delete(1).await.unwrap();

        let found = repo.find_by_id(1).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_repository_count() {
        let repo = MockRepository::new();
        assert_eq!(repo.count().await.unwrap(), 0);

        repo.insert(&TestEntity {
            id: 0,
            name: "Test".to_string(),
        })
        .await
        .unwrap();
        assert_eq!(repo.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_repository_count_with_filter() {
        let repo = MockRepository::new();
        repo.insert(&TestEntity {
            id: 0,
            name: "Alice".to_string(),
        })
        .await
        .unwrap();
        repo.insert(&TestEntity {
            id: 0,
            name: "Bob".to_string(),
        })
        .await
        .unwrap();

        let mut conditions = HashMap::new();
        conditions.insert("name".to_string(), FilterOp::Eq("Alice".to_string()));
        let filter = TestFilter { conditions };

        assert_eq!(repo.count_with_filter(&filter).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_repository_exists() {
        let repo = MockRepository::new();
        repo.insert(&TestEntity {
            id: 0,
            name: "Test".to_string(),
        })
        .await
        .unwrap();

        assert!(repo.exists(1).await.unwrap());
        assert!(!repo.exists(999).await.unwrap());
    }

    #[tokio::test]
    async fn test_repository_connection_access() {
        let repo = MockRepository::new();
        assert!(repo.connection().is_connected());
    }

    #[test]
    fn test_no_filter_with_repository() {
        let filter = NoFilter;
        assert!(filter.apply("any").is_none());
    }
}
