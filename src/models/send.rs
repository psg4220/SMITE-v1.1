//! Send/transfer command models

/// Result of sending funds
#[derive(Debug)]
pub struct SendResult {
    pub sender_id: i64,
    pub receiver_ids: Vec<i64>,
    pub amount: String,
    pub currency_ticker: String,
    pub tax_amount: String,
}
