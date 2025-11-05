use reqwest::Client as HttpClient;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use super::models::{
    BalanceResponse, BalanceUpdateRequest, BalanceModifyRequest, ApiError, RateLimitInfo,
    RateLimitResponse,
};
use tracing::warn;

/// Unbelievaboat API client for handling Discord economy interactions
pub struct UnbelievaboatClient {
    http_client: HttpClient,
    api_token: String,
    base_url: String,
}

impl UnbelievaboatClient {
    const DEFAULT_BASE_URL: &'static str = "https://api.unbelievaboat.com/v1";

    /// Create a new Unbelievaboat API client
    pub fn new(api_token: String) -> Self {
        Self {
            http_client: HttpClient::new(),
            api_token,
            base_url: Self::DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Create a new client with custom base URL (for testing)
    pub fn with_base_url(api_token: String, base_url: String) -> Self {
        Self {
            http_client: HttpClient::new(),
            api_token,
            base_url,
        }
    }

    /// Create default headers with authorization
    fn create_headers(&self) -> Result<HeaderMap, String> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        
        let auth_value = HeaderValue::from_str(&format!("Bearer {}", self.api_token))
            .map_err(|e| format!("Failed to create auth header: {}", e))?;
        headers.insert(AUTHORIZATION, auth_value);
        
        Ok(headers)
    }

    /// Extract rate limit information from response headers
    fn extract_rate_limit_info(response: &reqwest::Response) -> RateLimitInfo {
        RateLimitInfo {
            limit: response
                .headers()
                .get("X-RateLimit-Limit")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok()),
            remaining: response
                .headers()
                .get("X-RateLimit-Remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok()),
            reset: response
                .headers()
                .get("X-RateLimit-Reset")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok()),
        }
    }

    /// Parse error response based on HTTP status code
    async fn handle_error_response(
        status: reqwest::StatusCode,
        response: reqwest::Response,
    ) -> ApiError {
        let status_code = status.as_u16();
        let body_text = response.text().await.unwrap_or_default();

        match status_code {
            400 => {
                // Try to parse JSON error
                if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&body_text) {
                    let message = err_json
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&body_text);
                    ApiError::BadRequest(message.to_string())
                } else {
                    ApiError::BadRequest(body_text)
                }
            }
            401 => ApiError::Unauthorized(body_text),
            403 => ApiError::Forbidden(body_text),
            404 => ApiError::NotFound(body_text),
            429 => {
                // Parse rate limit response
                if let Ok(rate_limit) = serde_json::from_str::<RateLimitResponse>(&body_text) {
                    let retry_after = rate_limit.retry_after.unwrap_or(1000);
                    let is_global = rate_limit.global.unwrap_or(false);
                    warn!(
                        "Rate limited (global: {}), retry after {} ms",
                        is_global, retry_after
                    );
                    ApiError::RateLimited {
                        retry_after,
                        is_global,
                    }
                } else {
                    warn!("Rate limited, but could not parse retry_after");
                    ApiError::RateLimited {
                        retry_after: 1000,
                        is_global: false,
                    }
                }
            }
            500..=599 => {
                warn!("Server error {}: {}", status_code, body_text);
                ApiError::ServerError(status_code as i32, body_text)
            }
            _ => ApiError::HttpError(status_code as i32, body_text),
        }
    }

    /// GET /users/{user_id}/balance
    /// 
    /// Retrieves the current balance (cash and bank) for a Discord user.
    /// 
    /// # Arguments
    /// * `guild_id` - The Discord guild ID
    /// * `user_id` - The Discord user ID
    /// 
    /// # Returns
    /// * `Ok(BalanceResponse)` - User's current balance
    /// * `Err(ApiError)` - Error with detailed error type and rate limit info
    pub async fn get_user_balance(
        &self,
        guild_id: u64,
        user_id: u64,
    ) -> Result<BalanceResponse, ApiError> {
        let url = format!("{}/users/{}/{}/balance", self.base_url, guild_id, user_id);
        let headers = self.create_headers()
            .map_err(|e| ApiError::RequestError(e))?;

        let response = self.http_client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ApiError::RequestError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(Self::handle_error_response(status, response).await);
        }

        response
            .json::<BalanceResponse>()
            .await
            .map_err(|e| ApiError::DeserializationError(format!("Failed to parse response: {}", e)))
    }

    /// PUT /users/{user_id}/balance
    /// 
    /// Sets the balance (cash and/or bank) for a Discord user. This is a complete override,
    /// not an increment/decrement operation.
    /// 
    /// # Arguments
    /// * `guild_id` - The Discord guild ID
    /// * `user_id` - The Discord user ID
    /// * `cash` - Optional: Set cash balance to this value
    /// * `bank` - Optional: Set bank balance to this value
    /// 
    /// # Returns
    /// * `Ok(BalanceResponse)` - Updated balance information
    /// * `Err(ApiError)` - Error with detailed error type and rate limit info
    pub async fn set_user_balance(
        &self,
        guild_id: u64,
        user_id: u64,
        cash: Option<i64>,
        bank: Option<i64>,
    ) -> Result<BalanceResponse, ApiError> {
        let url = format!("{}/users/{}/{}/balance", self.base_url, guild_id, user_id);
        let headers = self.create_headers()
            .map_err(|e| ApiError::RequestError(e))?;

        let body = BalanceUpdateRequest { cash, bank };

        let response = self.http_client
            .put(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::RequestError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(Self::handle_error_response(status, response).await);
        }

        response
            .json::<BalanceResponse>()
            .await
            .map_err(|e| ApiError::DeserializationError(format!("Failed to parse response: {}", e)))
    }

    /// PATCH /users/{user_id}/balance
    /// 
    /// Modifies the balance (cash and/or bank) for a Discord user. This operation
    /// increments/decrements the current balance, not sets it to a fixed value.
    /// 
    /// # Arguments
    /// * `guild_id` - The Discord guild ID
    /// * `user_id` - The Discord user ID
    /// * `cash` - Optional: Add/subtract this amount from cash (negative for subtraction)
    /// * `bank` - Optional: Add/subtract this amount from bank (negative for subtraction)
    /// 
    /// # Returns
    /// * `Ok(BalanceResponse)` - Updated balance information
    /// * `Err(ApiError)` - Error with detailed error type and rate limit info
    pub async fn modify_user_balance(
        &self,
        guild_id: u64,
        user_id: u64,
        cash: Option<i64>,
        bank: Option<i64>,
    ) -> Result<BalanceResponse, ApiError> {
        let url = format!("{}/users/{}/{}/balance", self.base_url, guild_id, user_id);
        let headers = self.create_headers()
            .map_err(|e| ApiError::RequestError(e))?;

        let body = BalanceModifyRequest { cash, bank };

        let response = self.http_client
            .patch(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::RequestError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(Self::handle_error_response(status, response).await);
        }

        response
            .json::<BalanceResponse>()
            .await
            .map_err(|e| ApiError::DeserializationError(format!("Failed to parse response: {}", e)))
    }
}


