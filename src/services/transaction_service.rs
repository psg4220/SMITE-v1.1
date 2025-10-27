use sqlx::mysql::MySqlPool;
use crate::db;

pub struct TransactionListResult {
    pub formatted_message: String,
    pub is_empty: bool,
}

pub struct TransactionDetailResult {
    pub sender_discord_id: i64,
    pub receiver_discord_id: i64,
    pub amount: f64,
    pub date: String,
}

/// Get formatted transaction list (top 10 most recent)
pub async fn get_transaction_list(
    pool: &MySqlPool,
    user_id: i64,
) -> Result<TransactionListResult, String> {
    // Get the top 10 most recent transactions for the user (as sender or receiver)
    let transactions = db::transaction::get_user_transactions(pool, user_id, 10)
        .await
        .map_err(|e| format!("Failed to fetch transactions: {}", e))?;

    if transactions.is_empty() {
        return Ok(TransactionListResult {
            formatted_message: String::new(),
            is_empty: true,
        });
    }

    // Build transaction list using markdown (limited to 10 by database query)
    let mut message = String::from("**📋 Transaction History** (Most Recent)\n\n");

    for (idx, tx) in transactions.iter().enumerate() {
        // Get sender and receiver Discord IDs from account IDs
        let sender_discord_id = db::account::get_discord_id_by_account_id(pool, tx.0)
            .await
            .unwrap_or(None)
            .unwrap_or(0);
        let receiver_discord_id = db::account::get_discord_id_by_account_id(pool, tx.1)
            .await
            .unwrap_or(None)
            .unwrap_or(0);

        message.push_str(&format!(
            "**{}** <@{}> → <@{}> | `{:.2} {}`\n",
            idx + 1, sender_discord_id, receiver_discord_id, tx.2, tx.5
        ));
        message.push_str(&format!("→ `{}`\n\n", tx.4));
    }

    Ok(TransactionListResult {
        formatted_message: message,
        is_empty: false,
    })
}

/// Get formatted transaction details by UUID
pub async fn get_transaction_detail(
    pool: &MySqlPool,
    uuid: &str,
) -> Result<TransactionDetailResult, String> {
    // Fetch specific transaction
    let transaction = db::transaction::get_transaction_by_uuid(pool, uuid)
        .await
        .map_err(|e| format!("Failed to fetch transaction: {}", e))?
        .ok_or("❌ Transaction not found".to_string())?;

    // Get sender and receiver Discord IDs
    let sender_discord_id = db::account::get_discord_id_by_account_id(pool, transaction.0)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Sender not found".to_string())?;

    let receiver_discord_id = db::account::get_discord_id_by_account_id(pool, transaction.1)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Receiver not found".to_string())?;

    Ok(TransactionDetailResult {
        sender_discord_id,
        receiver_discord_id,
        amount: transaction.3,
        date: transaction.2,
    })
}
