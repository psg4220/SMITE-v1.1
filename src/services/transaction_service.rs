use sqlx::mysql::MySqlPool;
use serenity::builder::CreateEmbed;
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

/// Get all transactions for pagination (no limit)
pub async fn get_transaction_list_for_pagination(
    pool: &MySqlPool,
    user_id: i64,
) -> Result<Vec<(i64, i64, f64, String, String, String)>, String> {
    // Get all transactions for the user (as sender or receiver)
    db::transaction::get_user_transactions(pool, user_id, 1000)
        .await
        .map_err(|e| format!("Failed to fetch transactions: {}", e))
}

/// Create paginated embeds for transactions (10 per page)
pub async fn create_transaction_pages(
    pool: &MySqlPool,
    user_id: i64,
) -> Result<Vec<CreateEmbed>, String> {
    let transactions = get_transaction_list_for_pagination(pool, user_id).await?;

    if transactions.is_empty() {
        let embed = CreateEmbed::default()
            .title("📋 Transaction History")
            .description("No transactions found")
            .color(0xffa500);
        return Ok(vec![embed]);
    }

    let mut pages = Vec::new();
    const TRANSACTIONS_PER_PAGE: usize = 10;

    for (page_idx, chunk) in transactions.chunks(TRANSACTIONS_PER_PAGE).enumerate() {
        let mut description = String::new();

        for tx in chunk {
            // tx is (sender_id, receiver_id, amount, date, uuid, currency_ticker)
            let sender_discord_id = db::account::get_discord_id_by_account_id(pool, tx.0)
                .await
                .unwrap_or(None)
                .unwrap_or(tx.0);
            let receiver_discord_id = db::account::get_discord_id_by_account_id(pool, tx.1)
                .await
                .unwrap_or(None)
                .unwrap_or(tx.1);

            description.push_str(&format!(
                "<@{}> → <@{}> | `{:.2} {}`\n",
                sender_discord_id, receiver_discord_id, tx.2, tx.5
            ));
            description.push_str(&format!("└─ `{}`\n\n", tx.4));
        }

        let embed = CreateEmbed::default()
            .title("📋 Transaction History")
            .description(description)
            .footer(serenity::builder::CreateEmbedFooter::new(
                format!("Page {} of {}", page_idx + 1, (transactions.len() + TRANSACTIONS_PER_PAGE - 1) / TRANSACTIONS_PER_PAGE)
            ))
            .color(0x00ff00);

        pages.push(embed);
    }

    Ok(pages)
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
