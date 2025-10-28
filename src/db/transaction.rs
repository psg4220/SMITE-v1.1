use sqlx::mysql::MySqlPool;

/// Create a new transaction record
pub async fn create_transaction(
    pool: &MySqlPool,
    uuid: &str,
    sender_id: i64,
    receiver_id: i64,
    amount: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO transaction (uuid, sender_id, receiver_id, amount) VALUES (?, ?, ?, ?)"
    )
    .bind(uuid)
    .bind(sender_id)
    .bind(receiver_id)
    .bind(amount)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get transaction by UUID - returns (sender_id, receiver_id, date_created, amount, uuid)
pub async fn get_transaction_by_uuid(
    pool: &MySqlPool,
    uuid: &str,
) -> Result<Option<(i64, i64, String, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, String, f64, String)>(
        "SELECT sender_id, receiver_id, DATE_FORMAT(date_created, '%Y-%m-%d %H:%i:%s'), CAST(amount AS DOUBLE), uuid FROM transaction WHERE uuid = ?"
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await
}

/// Get transaction by UUID (legacy name)
pub async fn get_transaction(
    pool: &MySqlPool,
    uuid: &str,
) -> Result<Option<(String, i64, i64, f64)>, sqlx::Error> {
    sqlx::query_as::<_, (String, i64, i64, f64)>(
        "SELECT uuid, sender_id, receiver_id, amount FROM transaction WHERE uuid = ?"
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await
}

/// Get all transactions by sender
pub async fn get_transactions_by_sender(
    pool: &MySqlPool,
    sender_id: i64,
) -> Result<Vec<(String, i64, i64, f64)>, sqlx::Error> {
    sqlx::query_as::<_, (String, i64, i64, f64)>(
        "SELECT uuid, sender_id, receiver_id, amount FROM transaction WHERE sender_id = ?"
    )
    .bind(sender_id)
    .fetch_all(pool)
    .await
}

/// Get all transactions by receiver
pub async fn get_transactions_by_receiver(
    pool: &MySqlPool,
    receiver_id: i64,
) -> Result<Vec<(String, i64, i64, f64)>, sqlx::Error> {
    sqlx::query_as::<_, (String, i64, i64, f64)>(
        "SELECT uuid, sender_id, receiver_id, amount FROM transaction WHERE receiver_id = ?"
    )
    .bind(receiver_id)
    .fetch_all(pool)
    .await
}

/// Get all transactions for a user (as sender or receiver) across all their accounts - returns (sender_id, receiver_id, amount, date_created, uuid)
/// Get all transactions for a user (as sender or receiver) across all their accounts - returns (sender_id, receiver_id, amount, date_created, uuid, currency_ticker)
pub async fn get_user_transactions(
    pool: &MySqlPool,
    account_id: i64,
    limit: u32,
) -> Result<Vec<(i64, i64, f64, String, String, String)>, sqlx::Error> {
    // First need to get all account IDs for this Discord ID (one per currency)
    let discord_id = account_id;
    let account_query = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM account WHERE discord_id = ?"
    )
    .bind(discord_id)
    .fetch_all(pool)
    .await?;

    if account_query.is_empty() {
        return Ok(vec![]);
    }

    // Get all transactions where user is sender or receiver in ANY of their accounts, ordered by date descending, with limit
    let account_ids: Vec<i64> = account_query.iter().map(|row| row.0).collect();
    
    // Build a query that checks if sender_id or receiver_id match any of the user's account IDs
    let mut query_str = String::from(
        "SELECT t.sender_id, t.receiver_id, CAST(t.amount AS DOUBLE), DATE_FORMAT(t.date_created, '%Y-%m-%d %H:%i:%s'), t.uuid, \
         COALESCE((SELECT c.ticker FROM currency c JOIN account a ON a.currency_id = c.id WHERE a.id = t.sender_id LIMIT 1), '') AS ticker \
         FROM transaction t \
         WHERE "
    );
    
    // Add conditions for all account IDs
    let or_conditions: Vec<String> = (0..account_ids.len())
        .map(|i| {
            if i == 0 {
                format!("(t.sender_id = ? OR t.receiver_id = ?)")
            } else {
                format!(" OR (t.sender_id = ? OR t.receiver_id = ?)")
            }
        })
        .collect();
    
    query_str.push_str(&or_conditions.join(""));
    query_str.push_str(" ORDER BY t.date_created DESC LIMIT ?");
    
    let mut query = sqlx::query_as::<_, (i64, i64, f64, String, String, String)>(&query_str);
    
    // Bind all account IDs (each appears twice: once for sender check, once for receiver check)
    for &acct_id in &account_ids {
        query = query.bind(acct_id).bind(acct_id);
    }
    query = query.bind(limit as i64);
    
    query.fetch_all(pool).await
}
