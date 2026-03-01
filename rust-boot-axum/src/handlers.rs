//! CRUD handler implementations for Axum endpoints.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;

/// Query parameters for paginated list endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationQuery {
    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: u64,
    /// Number of items per page.
    #[serde(default = "default_per_page")]
    pub per_page: u64,
    /// Whether to include soft-deleted items.
    #[serde(default)]
    pub include_deleted: bool,
}

const fn default_page() -> u64 {
    1
}

const fn default_per_page() -> u64 {
    20
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            page: default_page(),
            per_page: default_per_page(),
            include_deleted: false,
        }
    }
}

/// Response wrapper for paginated data.
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    /// The page data.
    pub data: Vec<T>,
    /// Current page number.
    pub page: u64,
    /// Items per page.
    pub per_page: u64,
    /// Total number of items.
    pub total: u64,
    /// Total number of pages.
    pub total_pages: u64,
}

impl<T> PaginatedResponse<T> {
    /// Creates a new paginated response.
    pub const fn new(data: Vec<T>, page: u64, per_page: u64, total: u64) -> Self {
        let total_pages = if per_page > 0 {
            total.div_ceil(per_page)
        } else {
            0
        };
        Self {
            data,
            page,
            per_page,
            total,
            total_pages,
        }
    }
}

/// Response wrapper for single-item API responses.
#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    /// The response data.
    pub data: T,
}

impl<T> ApiResponse<T> {
    /// Creates a new API response.
    pub const fn new(data: T) -> Self {
        Self { data }
    }
}

/// Structured error response for API endpoints.
#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    /// Error code (e.g., "`not_found`", "`bad_request`").
    pub error: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional additional error details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    /// Creates a new API error with code and message.
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Adds additional details to the error.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Creates a not found error for the given resource.
    pub fn not_found(resource: &str) -> Self {
        Self::new("not_found", format!("{resource} not found"))
    }

    /// Creates a bad request error.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("bad_request", message)
    }

    /// Creates an internal server error.
    pub fn internal_error() -> Self {
        Self::new("internal_error", "An internal error occurred")
    }

    /// Creates a validation error.
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::new("validation_error", message)
    }

    /// Creates a conflict error.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new("conflict", message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.error.as_str() {
            "not_found" => StatusCode::NOT_FOUND,
            "bad_request" => StatusCode::BAD_REQUEST,
            "validation_error" => StatusCode::UNPROCESSABLE_ENTITY,
            "conflict" => StatusCode::CONFLICT,
            "unauthorized" => StatusCode::UNAUTHORIZED,
            "forbidden" => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

/// Result type for single-item API responses.
pub type ApiResult<T> = Result<Json<ApiResponse<T>>, ApiError>;
/// Result type for paginated API responses.
pub type PaginatedResult<T> = Result<Json<PaginatedResponse<T>>, ApiError>;

/// Returns a successful API response with data.
pub const fn ok<T: Serialize>(data: T) -> ApiResult<T> {
    Ok(Json(ApiResponse::new(data)))
}

/// Returns a 201 Created response with data.
pub const fn created<T: Serialize>(data: T) -> (StatusCode, Json<ApiResponse<T>>) {
    (StatusCode::CREATED, Json(ApiResponse::new(data)))
}

/// Returns a 204 No Content response.
pub const fn no_content() -> StatusCode {
    StatusCode::NO_CONTENT
}

/// Returns a successful paginated response.
pub const fn paginated<T: Serialize>(
    data: Vec<T>,
    page: u64,
    per_page: u64,
    total: u64,
) -> PaginatedResult<T> {
    Ok(Json(PaginatedResponse::new(data, page, per_page, total)))
}

/// Trait defining standard CRUD handler signatures.
#[async_trait::async_trait]
pub trait CrudHandlers<S, Id, Entity, CreateDto, UpdateDto>
where
    S: Clone + Send + Sync + 'static,
    Id: Send + Sync + 'static,
    Entity: Serialize + Send + Sync + 'static,
    CreateDto: DeserializeOwned + Send + Sync + 'static,
    UpdateDto: DeserializeOwned + Send + Sync + 'static,
{
    /// Lists entities with pagination.
    async fn list(state: State<S>, query: Query<PaginationQuery>) -> PaginatedResult<Entity>;

    /// Gets a single entity by ID.
    async fn get(state: State<S>, id: Path<Id>) -> ApiResult<Entity>;

    /// Creates a new entity.
    async fn create(
        state: State<S>,
        payload: Json<CreateDto>,
    ) -> (StatusCode, Json<ApiResponse<Entity>>);

    /// Updates an existing entity.
    async fn update(state: State<S>, id: Path<Id>, payload: Json<UpdateDto>) -> ApiResult<Entity>;

    /// Deletes an entity.
    async fn delete(state: State<S>, id: Path<Id>) -> StatusCode;

    /// Restores a soft-deleted entity.
    async fn restore(state: State<S>, id: Path<Id>) -> ApiResult<Entity>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_query_default() {
        let query = PaginationQuery::default();
        assert_eq!(query.page, 1);
        assert_eq!(query.per_page, 20);
        assert!(!query.include_deleted);
    }

    #[test]
    fn test_paginated_response_new() {
        let data = vec![1, 2, 3];
        let response = PaginatedResponse::new(data, 1, 10, 25);

        assert_eq!(response.page, 1);
        assert_eq!(response.per_page, 10);
        assert_eq!(response.total, 25);
        assert_eq!(response.total_pages, 3);
    }

    #[test]
    fn test_paginated_response_total_pages_calculation() {
        let response: PaginatedResponse<i32> = PaginatedResponse::new(vec![], 1, 10, 100);
        assert_eq!(response.total_pages, 10);

        let response: PaginatedResponse<i32> = PaginatedResponse::new(vec![], 1, 10, 101);
        assert_eq!(response.total_pages, 11);

        let response: PaginatedResponse<i32> = PaginatedResponse::new(vec![], 1, 10, 0);
        assert_eq!(response.total_pages, 0);
    }

    #[test]
    fn test_paginated_response_zero_per_page() {
        let response: PaginatedResponse<i32> = PaginatedResponse::new(vec![], 1, 0, 100);
        assert_eq!(response.total_pages, 0);
    }

    #[test]
    fn test_api_response_new() {
        let response = ApiResponse::new("test");
        assert_eq!(response.data, "test");
    }

    #[test]
    fn test_api_error_not_found() {
        let error = ApiError::not_found("User");
        assert_eq!(error.error, "not_found");
        assert_eq!(error.message, "User not found");
    }

    #[test]
    fn test_api_error_bad_request() {
        let error = ApiError::bad_request("Invalid input");
        assert_eq!(error.error, "bad_request");
        assert_eq!(error.message, "Invalid input");
    }

    #[test]
    fn test_api_error_with_details() {
        let error = ApiError::validation_error("Field validation failed")
            .with_details(serde_json::json!({"field": "email", "reason": "invalid format"}));

        assert!(error.details.is_some());
    }

    #[test]
    fn test_api_error_internal() {
        let error = ApiError::internal_error();
        assert_eq!(error.error, "internal_error");
    }

    #[test]
    fn test_api_error_conflict() {
        let error = ApiError::conflict("Resource already exists");
        assert_eq!(error.error, "conflict");
    }

    #[derive(Serialize)]
    struct TestEntity {
        id: i64,
        name: String,
    }

    #[test]
    fn test_ok_response() {
        let entity = TestEntity {
            id: 1,
            name: "test".to_string(),
        };
        let result = ok(entity);
        assert!(result.is_ok());
    }

    #[test]
    fn test_created_response() {
        let entity = TestEntity {
            id: 1,
            name: "test".to_string(),
        };
        let (status, _json) = created(entity);
        assert_eq!(status, StatusCode::CREATED);
    }

    #[test]
    fn test_no_content_response() {
        let status = no_content();
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    #[test]
    fn test_paginated_helper() {
        let data = vec![
            TestEntity {
                id: 1,
                name: "a".to_string(),
            },
            TestEntity {
                id: 2,
                name: "b".to_string(),
            },
        ];
        let result = paginated(data, 1, 10, 50);
        assert!(result.is_ok());
    }
}
