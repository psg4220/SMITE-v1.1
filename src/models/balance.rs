//! Balance command models

/// Result of a balance query
#[derive(Debug, Clone)]
pub struct BalanceResult {
    pub user_id: i64,
    pub balance: String,
    pub currency_ticker: String,
}
