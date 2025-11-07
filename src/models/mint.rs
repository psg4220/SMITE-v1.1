//! Mint/currency creation models

/// Result of minting currency
#[derive(Debug)]
pub struct MintResult {
    pub user_id: i64,
    pub amount: f64,
    pub new_balance: f64,
    pub currency_ticker: String,
}
