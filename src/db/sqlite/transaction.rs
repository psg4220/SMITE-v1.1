use sqlx::sqlite::SqlitePool;

/// Create a new transaction record
pub async fn create_transaction(
    pool: &SqlitePool,
    uuid: &str,
    sender_id: i64,
    receiver_id: i64,
    amount: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO \"transaction\" (uuid, sender_id, receiver_id, amount) VALUES (?, ?, ?, ?)"
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
    pool: &SqlitePool,
    uuid: &str,
) -> Result<Option<(i64, i64, String, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, String, f64, String)>(
        "SELECT sender_id, receiver_id, strftime('%Y-%m-%d %H:%M:%S', date_created), CAST(amount AS REAL), uuid FROM \"transaction\" WHERE uuid = ?"
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await
}

/// Get transaction by UUID (legacy name)
pub async fn get_transaction(
    pool: &SqlitePool,
    uuid: &str,
) -> Result<Option<(String, i64, i64, f64)>, sqlx::Error> {
    sqlx::query_as::<_, (String, i64, i64, f64)>(
        "SELECT uuid, sender_id, receiver_id, amount FROM \"transaction\" WHERE uuid = ?"
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await
}

/// Get all transactions by sender
pub async fn get_transactions_by_sender(
    pool: &SqlitePool,
    sender_id: i64,
    limit: i64,
) -> Result<Vec<(String, i64, i64, f64)>, sqlx::Error> {
    sqlx::query_as::<_, (String, i64, i64, f64)>(
        "SELECT uuid, sender_id, receiver_id, amount FROM \"transaction\" WHERE sender_id = ? LIMIT ?"
    )
    .bind(sender_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Get all transactions by receiver
pub async fn get_transactions_by_receiver(
    pool: &SqlitePool,
    receiver_id: i64,
    limit: i64,
) -> Result<Vec<(String, i64, i64, f64)>, sqlx::Error> {
    sqlx::query_as::<_, (String, i64, i64, f64)>(
        "SELECT uuid, sender_id, receiver_id, amount FROM \"transaction\" WHERE receiver_id = ? LIMIT ?"
    )
    .bind(receiver_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Get all transactions for a user (as sender or receiver) across all their accounts - returns (sender_id, receiver_id, amount, date_created, uuid)
/// Get all transactions for a user (as sender or receiver) across all their accounts - returns (sender_id, receiver_id, amount, date_created, uuid, currency_ticker)
pub async fn get_user_transactions(
    pool: &SqlitePool,
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
        "SELECT t.sender_id, t.receiver_id, CAST(t.amount AS REAL), strftime('%Y-%m-%d %H:%M:%S', t.date_created), t.uuid, \
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

/// Get paginated transactions for a user (as sender or receiver) across all their accounts
/// Returns: Vec<(sender_id, receiver_id, amount, date_created, uuid, currency_ticker)>
/// Supports: page number, page size, automatic OFFSET calculation
pub async fn get_user_transactions_paginated(
    pool: &SqlitePool,
    account_id: i64,
    page: usize,
    page_size: usize,
) -> Result<(Vec<(i64, i64, f64, String, String, String)>, i64), sqlx::Error> {
    // First get all account IDs for this Discord ID (one per currency)
    let discord_id = account_id;
    let account_query = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM account WHERE discord_id = ?"
    )
    .bind(discord_id)
    .fetch_all(pool)
    .await?;

    if account_query.is_empty() {
        return Ok((vec![], 0));
    }

    let account_ids: Vec<i64> = account_query.iter().map(|row| row.0).collect();
    
    // First, get total count of transactions
    let mut count_query_str = String::from(
        "SELECT COUNT(*) as count FROM transaction t WHERE "
    );
    
    let or_conditions_count: Vec<String> = (0..account_ids.len())
        .map(|i| {
            if i == 0 {
                format!("(t.sender_id = ? OR t.receiver_id = ?)")
            } else {
                format!(" OR (t.sender_id = ? OR t.receiver_id = ?)")
            }
        })
        .collect();
    
    count_query_str.push_str(&or_conditions_count.join(""));
    
    let mut count_query = sqlx::query_as::<_, (i64,)>(&count_query_str);
    for &acct_id in &account_ids {
        count_query = count_query.bind(acct_id).bind(acct_id);
    }
    
    let (total_count,) = count_query.fetch_one(pool).await?;

    // Calculate offset
    let offset = (page - 1) * page_size;
    
    // Build the paginated query
    let mut query_str = String::from(
        "SELECT t.sender_id, t.receiver_id, CAST(t.amount AS REAL), strftime('%Y-%m-%d %H:%M:%S', t.date_created), t.uuid, \
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
    query_str.push_str(" ORDER BY t.date_created DESC LIMIT ? OFFSET ?");
    
    let mut query = sqlx::query_as::<_, (i64, i64, f64, String, String, String)>(&query_str);
    
    // Bind all account IDs (each appears twice: once for sender check, once for receiver check)
    for &acct_id in &account_ids {
        query = query.bind(acct_id).bind(acct_id);
    }
    query = query.bind(page_size as i64).bind(offset as i64);
    
    let transactions = query.fetch_all(pool).await?;
    
    Ok((transactions, total_count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS currency (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                guild_id INTEGER UNIQUE NOT NULL,
                name TEXT UNIQUE NOT NULL,
                ticker TEXT UNIQUE NOT NULL,
                date_created TEXT DEFAULT (datetime('now'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS account (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                discord_id INTEGER NOT NULL,
                currency_id INTEGER NOT NULL,
                balance REAL NOT NULL DEFAULT 0.0,
                date_created TEXT DEFAULT (datetime('now')),
                date_updated TEXT DEFAULT (datetime('now')),
                UNIQUE (discord_id, currency_id)
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS \"transaction\" (
                uuid TEXT PRIMARY KEY,
                sender_id INTEGER NOT NULL,
                receiver_id INTEGER NOT NULL,
                amount REAL NOT NULL,
                date_created TEXT DEFAULT (datetime('now'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_transaction() {
        let pool = setup_test_db().await;

        // Create currency and accounts
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test', 'TST')")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO account (discord_id, currency_id, balance) VALUES (100, 1, 1000.0)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO account (discord_id, currency_id, balance) VALUES (200, 1, 500.0)")
            .execute(&pool)
            .await
            .unwrap();

        let result = create_transaction(&pool, "test-uuid-123", 1, 2, 100.0).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_transaction() {
        let pool = setup_test_db().await;

        // Create currency and accounts
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test', 'TST')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO account (discord_id, currency_id, balance) VALUES (100, 1, 1000.0)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO account (discord_id, currency_id, balance) VALUES (200, 1, 500.0)")
            .execute(&pool)
            .await
            .unwrap();

        create_transaction(&pool, "test-uuid-456", 1, 2, 250.0).await.unwrap();

        let tx = get_transaction(&pool, "test-uuid-456").await.unwrap();
        assert!(tx.is_some());
        let (uuid, sender_id, receiver_id, amount) = tx.unwrap();
        assert_eq!(uuid, "test-uuid-456");
        assert_eq!(sender_id, 1);
        assert_eq!(receiver_id, 2);
        assert_eq!(amount, 250.0);
    }
}
