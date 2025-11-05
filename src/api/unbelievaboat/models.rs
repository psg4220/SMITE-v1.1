use serde::{Deserialize, Serialize};

/// Response from GET user balance endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBalance {
    pub cash: i64,
    pub bank: i64,
}

/// Full balance response from the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub user_id: String,
    pub cash: i64,
    pub bank: i64,
}

/// Request body for PUT user balance (set balance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceUpdateRequest {
    pub cash: Option<i64>,
    pub bank: Option<i64>,
}

/// Request body for PATCH user balance (modify balance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceModifyRequest {
    pub cash: Option<i64>,
    pub bank: Option<i64>,
}

/// Error response from the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: Option<String>,
    pub message: Option<String>,
    pub status: Option<i32>,
}

/// Rate limit information from API response headers
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: Option<i32>,
    pub remaining: Option<i32>,
    pub reset: Option<i64>,
}

/// 429 Rate limit response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResponse {
    pub message: String,
    pub retry_after: Option<i64>,
    pub global: Option<bool>,
}

/// Comprehensive error type for API operations
#[derive(Debug, Clone)]
pub enum ApiError {
    /// 400 Bad Request
    BadRequest(String),
    /// 401 Unauthorized
    Unauthorized(String),
    /// 403 Forbidden
    Forbidden(String),
    /// 404 Not Found
    NotFound(String),
    /// 429 Too Many Requests (rate limited)
    RateLimited {
        retry_after: i64,
        is_global: bool,
    },
    /// 5xx Server Error
    ServerError(i32, String),
    /// Other HTTP errors
    HttpError(i32, String),
    /// Network/request error
    RequestError(String),
    /// Deserialization error
    DeserializationError(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            ApiError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            ApiError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            ApiError::RateLimited {
                retry_after,
                is_global,
            } => write!(
                f,
                "Rate Limited ({}). Retry after {} ms",
                if *is_global { "global" } else { "per-endpoint" },
                retry_after
            ),
            ApiError::ServerError(code, msg) => write!(f, "Server Error ({}): {}", code, msg),
            ApiError::HttpError(code, msg) => write!(f, "HTTP Error ({}): {}", code, msg),
            ApiError::RequestError(msg) => write!(f, "Request Error: {}", msg),
            ApiError::DeserializationError(msg) => write!(f, "Deserialization Error: {}", msg),
        }
    }
}
