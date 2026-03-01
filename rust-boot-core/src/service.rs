//! Service layer traits for CRUD operations with pagination and soft delete.

use std::fmt::Debug;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Parameters for paginated queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-indexed).
    pub page: u64,
    /// Number of items per page.
    pub per_page: u64,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 20,
        }
    }
}

impl PaginationParams {
    /// Creates new pagination parameters.
    pub fn new(page: u64, per_page: u64) -> Self {
        Self { page, per_page }
    }

    /// Calculates the offset for database queries.
    pub fn offset(&self) -> u64 {
        (self.page.saturating_sub(1)) * self.per_page
    }

    /// Returns the limit (items per page).
    pub fn limit(&self) -> u64 {
        self.per_page
    }
}

/// Result of a paginated query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    /// The items on this page.
    pub items: Vec<T>,
    /// Total number of items across all pages.
    pub total: u64,
    /// Current page number.
    pub page: u64,
    /// Items per page.
    pub per_page: u64,
    /// Total number of pages.
    pub total_pages: u64,
}

impl<T> PaginatedResult<T> {
    /// Creates a new paginated result.
    pub fn new(items: Vec<T>, total: u64, params: PaginationParams) -> Self {
        let total_pages = if params.per_page == 0 {
            0
        } else {
            (total + params.per_page - 1) / params.per_page
        };

        Self {
            items,
            total,
            page: params.page,
            per_page: params.per_page,
            total_pages,
        }
    }

    /// Returns true if there is a next page.
    pub fn has_next_page(&self) -> bool {
        self.page < self.total_pages
    }

    /// Returns true if there is a previous page.
    pub fn has_prev_page(&self) -> bool {
        self.page > 1
    }

    /// Returns true if the result contains no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the number of items on this page.
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

/// Sort direction for queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Asc
    }
}

/// Sort parameters for queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortParams {
    /// Field name to sort by.
    pub field: String,
    /// Sort direction.
    pub direction: SortDirection,
}

impl SortParams {
    /// Creates new sort parameters.
    pub fn new(field: impl Into<String>, direction: SortDirection) -> Self {
        Self {
            field: field.into(),
            direction,
        }
    }

    /// Creates ascending sort parameters.
    pub fn asc(field: impl Into<String>) -> Self {
        Self::new(field, SortDirection::Asc)
    }

    /// Creates descending sort parameters.
    pub fn desc(field: impl Into<String>) -> Self {
        Self::new(field, SortDirection::Desc)
    }
}

/// Filter operations for dynamic queries.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterOp {
    /// Equals comparison.
    Eq(String),
    /// Not equals comparison.
    Ne(String),
    /// Greater than comparison.
    Gt(String),
    /// Less than comparison.
    Lt(String),
    /// Greater than or equal comparison.
    Gte(String),
    /// Less than or equal comparison.
    Lte(String),
    /// Pattern matching (SQL LIKE).
    Like(String),
    /// Value in set.
    In(Vec<String>),
    /// Is null check.
    IsNull,
    /// Is not null check.
    IsNotNull,
}

/// Trait for implementing dynamic filters.
pub trait Filter: Send + Sync {
    /// Applies filter to a field, returning the operation if applicable.
    fn apply(&self, field: &str) -> Option<FilterOp>;
    /// Returns list of fields this filter applies to.
    fn fields(&self) -> Vec<String>;
}

/// Empty filter that matches everything.
#[derive(Debug, Clone, Default)]
pub struct NoFilter;

impl Filter for NoFilter {
    fn apply(&self, _field: &str) -> Option<FilterOp> {
        None
    }

    fn fields(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Service trait for CRUD operations.
#[async_trait]
pub trait CrudService: Send + Sync {
    /// Entity type managed by this service.
    type Entity: Send + Sync;
    /// Entity identifier type.
    type Id: Send + Sync;
    /// DTO for creating entities.
    type CreateDto: Send + Sync;
    /// DTO for updating entities.
    type UpdateDto: Send + Sync;

    /// Creates a new entity.
    async fn create(&self, dto: Self::CreateDto) -> Result<Self::Entity>;

    /// Finds an entity by ID.
    async fn find_by_id(&self, id: Self::Id) -> Result<Option<Self::Entity>>;

    /// Finds all entities with pagination.
    async fn find_all(&self, pagination: PaginationParams) -> Result<PaginatedResult<Self::Entity>>;

    /// Finds entities matching a filter with pagination.
    async fn find_with_filter(
        &self,
        filter: &dyn Filter,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<Self::Entity>>;

    /// Updates an entity.
    async fn update(&self, id: Self::Id, dto: Self::UpdateDto) -> Result<Self::Entity>;

    /// Soft deletes an entity.
    async fn delete(&self, id: Self::Id) -> Result<()>;

    /// Permanently deletes an entity.
    async fn hard_delete(&self, id: Self::Id) -> Result<()>;

    /// Restores a soft-deleted entity.
    async fn restore(&self, id: Self::Id) -> Result<Self::Entity>;

    /// Finds all entities including soft-deleted ones.
    async fn find_all_including_deleted(
        &self,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<Self::Entity>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[test]
    fn test_pagination_params_default() {
        let params = PaginationParams::default();
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 20);
    }

    #[test]
    fn test_pagination_params_offset() {
        let params = PaginationParams::new(1, 20);
        assert_eq!(params.offset(), 0);

        let params = PaginationParams::new(2, 20);
        assert_eq!(params.offset(), 20);

        let params = PaginationParams::new(3, 10);
        assert_eq!(params.offset(), 20);
    }

    #[test]
    fn test_pagination_params_offset_page_zero() {
        let params = PaginationParams::new(0, 20);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_paginated_result_new() {
        let items = vec![1, 2, 3];
        let result = PaginatedResult::new(items, 100, PaginationParams::new(1, 10));

        assert_eq!(result.items, vec![1, 2, 3]);
        assert_eq!(result.total, 100);
        assert_eq!(result.page, 1);
        assert_eq!(result.per_page, 10);
        assert_eq!(result.total_pages, 10);
    }

    #[test]
    fn test_paginated_result_total_pages_calculation() {
        let result: PaginatedResult<i32> =
            PaginatedResult::new(vec![], 95, PaginationParams::new(1, 10));
        assert_eq!(result.total_pages, 10);

        let result: PaginatedResult<i32> =
            PaginatedResult::new(vec![], 100, PaginationParams::new(1, 10));
        assert_eq!(result.total_pages, 10);

        let result: PaginatedResult<i32> =
            PaginatedResult::new(vec![], 101, PaginationParams::new(1, 10));
        assert_eq!(result.total_pages, 11);
    }

    #[test]
    fn test_paginated_result_has_next_page() {
        let result: PaginatedResult<i32> =
            PaginatedResult::new(vec![], 100, PaginationParams::new(1, 10));
        assert!(result.has_next_page());

        let result: PaginatedResult<i32> =
            PaginatedResult::new(vec![], 100, PaginationParams::new(10, 10));
        assert!(!result.has_next_page());
    }

    #[test]
    fn test_paginated_result_has_prev_page() {
        let result: PaginatedResult<i32> =
            PaginatedResult::new(vec![], 100, PaginationParams::new(1, 10));
        assert!(!result.has_prev_page());

        let result: PaginatedResult<i32> =
            PaginatedResult::new(vec![], 100, PaginationParams::new(2, 10));
        assert!(result.has_prev_page());
    }

    #[test]
    fn test_paginated_result_is_empty_and_len() {
        let result: PaginatedResult<i32> =
            PaginatedResult::new(vec![], 0, PaginationParams::default());
        assert!(result.is_empty());
        assert_eq!(result.len(), 0);

        let result = PaginatedResult::new(vec![1, 2, 3], 3, PaginationParams::default());
        assert!(!result.is_empty());
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_sort_direction_default() {
        assert_eq!(SortDirection::default(), SortDirection::Asc);
    }

    #[test]
    fn test_sort_params_constructors() {
        let sort = SortParams::asc("name");
        assert_eq!(sort.field, "name");
        assert_eq!(sort.direction, SortDirection::Asc);

        let sort = SortParams::desc("created_at");
        assert_eq!(sort.field, "created_at");
        assert_eq!(sort.direction, SortDirection::Desc);
    }

    #[test]
    fn test_filter_op_variants() {
        let _eq = FilterOp::Eq("value".to_string());
        let _ne = FilterOp::Ne("value".to_string());
        let _gt = FilterOp::Gt("10".to_string());
        let _lt = FilterOp::Lt("10".to_string());
        let _gte = FilterOp::Gte("10".to_string());
        let _lte = FilterOp::Lte("10".to_string());
        let _like = FilterOp::Like("%pattern%".to_string());
        let _in_op = FilterOp::In(vec!["a".to_string(), "b".to_string()]);
        let _is_null = FilterOp::IsNull;
        let _is_not_null = FilterOp::IsNotNull;
    }

    #[test]
    fn test_no_filter() {
        let filter = NoFilter;
        assert_eq!(filter.apply("any_field"), None);
        assert!(filter.fields().is_empty());
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

    #[test]
    fn test_custom_filter() {
        let mut conditions = HashMap::new();
        conditions.insert("status".to_string(), FilterOp::Eq("active".to_string()));
        conditions.insert("age".to_string(), FilterOp::Gte("18".to_string()));

        let filter = TestFilter { conditions };

        assert_eq!(
            filter.apply("status"),
            Some(FilterOp::Eq("active".to_string()))
        );
        assert_eq!(filter.apply("age"), Some(FilterOp::Gte("18".to_string())));
        assert_eq!(filter.apply("unknown"), None);
        assert_eq!(filter.fields().len(), 2);
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TestEntity {
        id: u64,
        name: String,
        deleted: bool,
    }

    #[derive(Debug, Clone)]
    struct CreateTestDto {
        name: String,
    }

    #[derive(Debug, Clone)]
    struct UpdateTestDto {
        name: Option<String>,
    }

    struct MockService {
        entities: Arc<RwLock<Vec<TestEntity>>>,
        next_id: Arc<RwLock<u64>>,
    }

    impl MockService {
        fn new() -> Self {
            Self {
                entities: Arc::new(RwLock::new(Vec::new())),
                next_id: Arc::new(RwLock::new(1)),
            }
        }
    }

    #[async_trait]
    impl CrudService for MockService {
        type Entity = TestEntity;
        type Id = u64;
        type CreateDto = CreateTestDto;
        type UpdateDto = UpdateTestDto;

        async fn create(&self, dto: Self::CreateDto) -> Result<Self::Entity> {
            let mut next_id = self.next_id.write().await;
            let entity = TestEntity {
                id: *next_id,
                name: dto.name,
                deleted: false,
            };
            *next_id += 1;

            let mut entities = self.entities.write().await;
            entities.push(entity.clone());
            Ok(entity)
        }

        async fn find_by_id(&self, id: Self::Id) -> Result<Option<Self::Entity>> {
            let entities = self.entities.read().await;
            Ok(entities.iter().find(|e| e.id == id && !e.deleted).cloned())
        }

        async fn find_all(
            &self,
            pagination: PaginationParams,
        ) -> Result<PaginatedResult<Self::Entity>> {
            let entities = self.entities.read().await;
            let active: Vec<_> = entities.iter().filter(|e| !e.deleted).cloned().collect();
            let total = active.len() as u64;
            let offset = pagination.offset() as usize;
            let limit = pagination.limit() as usize;
            let items: Vec<_> = active.into_iter().skip(offset).take(limit).collect();
            Ok(PaginatedResult::new(items, total, pagination))
        }

        async fn find_with_filter(
            &self,
            filter: &dyn Filter,
            pagination: PaginationParams,
        ) -> Result<PaginatedResult<Self::Entity>> {
            let entities = self.entities.read().await;
            let filtered: Vec<_> = entities
                .iter()
                .filter(|e| !e.deleted)
                .filter(|e| {
                    if let Some(FilterOp::Eq(val)) = filter.apply("name") {
                        e.name == val
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();
            let total = filtered.len() as u64;
            let offset = pagination.offset() as usize;
            let limit = pagination.limit() as usize;
            let items: Vec<_> = filtered.into_iter().skip(offset).take(limit).collect();
            Ok(PaginatedResult::new(items, total, pagination))
        }

        async fn update(&self, id: Self::Id, dto: Self::UpdateDto) -> Result<Self::Entity> {
            let mut entities = self.entities.write().await;
            let entity = entities
                .iter_mut()
                .find(|e| e.id == id && !e.deleted)
                .ok_or_else(|| crate::error::RustBootError::Database("Not found".to_string()))?;

            if let Some(name) = dto.name {
                entity.name = name;
            }
            Ok(entity.clone())
        }

        async fn delete(&self, id: Self::Id) -> Result<()> {
            let mut entities = self.entities.write().await;
            if let Some(entity) = entities.iter_mut().find(|e| e.id == id) {
                entity.deleted = true;
            }
            Ok(())
        }

        async fn hard_delete(&self, id: Self::Id) -> Result<()> {
            let mut entities = self.entities.write().await;
            entities.retain(|e| e.id != id);
            Ok(())
        }

        async fn restore(&self, id: Self::Id) -> Result<Self::Entity> {
            let mut entities = self.entities.write().await;
            let entity = entities
                .iter_mut()
                .find(|e| e.id == id && e.deleted)
                .ok_or_else(|| crate::error::RustBootError::Database("Not found".to_string()))?;
            entity.deleted = false;
            Ok(entity.clone())
        }

        async fn find_all_including_deleted(
            &self,
            pagination: PaginationParams,
        ) -> Result<PaginatedResult<Self::Entity>> {
            let entities = self.entities.read().await;
            let total = entities.len() as u64;
            let offset = pagination.offset() as usize;
            let limit = pagination.limit() as usize;
            let items: Vec<_> = entities
                .iter()
                .skip(offset)
                .take(limit)
                .cloned()
                .collect();
            Ok(PaginatedResult::new(items, total, pagination))
        }
    }

    #[tokio::test]
    async fn test_mock_service_create() {
        let service = MockService::new();
        let entity = service
            .create(CreateTestDto {
                name: "Test".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(entity.id, 1);
        assert_eq!(entity.name, "Test");
        assert!(!entity.deleted);
    }

    #[tokio::test]
    async fn test_mock_service_find_by_id() {
        let service = MockService::new();
        service
            .create(CreateTestDto {
                name: "Test".to_string(),
            })
            .await
            .unwrap();

        let found = service.find_by_id(1).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test");

        let not_found = service.find_by_id(999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_mock_service_find_all() {
        let service = MockService::new();
        for i in 0..25 {
            service
                .create(CreateTestDto {
                    name: format!("Entity {}", i),
                })
                .await
                .unwrap();
        }

        let result = service.find_all(PaginationParams::default()).await.unwrap();
        assert_eq!(result.total, 25);
        assert_eq!(result.items.len(), 20);
        assert!(result.has_next_page());
    }

    #[tokio::test]
    async fn test_mock_service_soft_delete() {
        let service = MockService::new();
        service
            .create(CreateTestDto {
                name: "Test".to_string(),
            })
            .await
            .unwrap();

        service.delete(1).await.unwrap();

        let found = service.find_by_id(1).await.unwrap();
        assert!(found.is_none());

        let all_including_deleted = service
            .find_all_including_deleted(PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(all_including_deleted.total, 1);
    }

    #[tokio::test]
    async fn test_mock_service_hard_delete() {
        let service = MockService::new();
        service
            .create(CreateTestDto {
                name: "Test".to_string(),
            })
            .await
            .unwrap();

        service.hard_delete(1).await.unwrap();

        let all_including_deleted = service
            .find_all_including_deleted(PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(all_including_deleted.total, 0);
    }

    #[tokio::test]
    async fn test_mock_service_restore() {
        let service = MockService::new();
        service
            .create(CreateTestDto {
                name: "Test".to_string(),
            })
            .await
            .unwrap();

        service.delete(1).await.unwrap();
        let restored = service.restore(1).await.unwrap();
        assert!(!restored.deleted);

        let found = service.find_by_id(1).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_mock_service_update() {
        let service = MockService::new();
        service
            .create(CreateTestDto {
                name: "Original".to_string(),
            })
            .await
            .unwrap();

        let updated = service
            .update(
                1,
                UpdateTestDto {
                    name: Some("Updated".to_string()),
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.name, "Updated");
    }

    #[tokio::test]
    async fn test_mock_service_find_with_filter() {
        let service = MockService::new();
        service
            .create(CreateTestDto {
                name: "Alice".to_string(),
            })
            .await
            .unwrap();
        service
            .create(CreateTestDto {
                name: "Bob".to_string(),
            })
            .await
            .unwrap();

        let mut conditions = HashMap::new();
        conditions.insert("name".to_string(), FilterOp::Eq("Alice".to_string()));
        let filter = TestFilter { conditions };

        let result = service
            .find_with_filter(&filter, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Alice");
    }
}
