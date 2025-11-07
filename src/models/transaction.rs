//! Transaction models

/// Paginated list of transactions for display
#[derive(Debug)]
pub struct TransactionListResult {
    pub formatted_message: String,
    pub is_empty: bool,
}

/// Detailed transaction information
#[derive(Debug)]
pub struct TransactionDetailResult {
    pub sender_discord_id: i64,
    pub receiver_discord_id: i64,
    pub amount: f64,
    pub date: String,
}

/// Individual transaction item for display
#[derive(Debug, Clone)]
pub struct TransactionItem {
    pub id: String,
    pub sender: String,
    pub receiver: String,
    pub amount: String,
    pub currency: String,
    pub timestamp: String,
}
