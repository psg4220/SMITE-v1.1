use sqlx::sqlite::SqlitePool;
use sqlx::Row;

/// Create a new account for a user
pub async fn create_account(
    pool: &SqlitePool,
    discord_id: i64,
    currency_id: i64,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query("INSERT INTO account (discord_id, currency_id, balance) VALUES (?, ?, 0.0)")
        .bind(discord_id)
        .bind(currency_id)
        .execute(pool)
        .await?;

    Ok(result.last_insert_rowid() as i64)
}

/// Get account balance by Discord user ID and currency ID
pub async fn get_account_balance(
    pool: &SqlitePool,
    discord_id: i64,
    currency_id: i64,
) -> Result<Option<f64>, sqlx::Error> {
    let row = sqlx::query("SELECT CAST(balance AS REAL) as balance FROM account WHERE discord_id = ? AND currency_id = ?")
        .bind(discord_id)
        .bind(currency_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get::<f64, _>("balance")))
}

/// Get account ID by Discord user ID and currency ID
pub async fn get_account_id(
    pool: &SqlitePool,
    discord_id: i64,
    currency_id: i64,
) -> Result<Option<i64>, sqlx::Error> {
    let row = sqlx::query("SELECT id FROM account WHERE discord_id = ? AND currency_id = ?")
        .bind(discord_id)
        .bind(currency_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get::<i64, _>("id")))
}

/// Update account balance by account ID
pub async fn update_balance(
    pool: &SqlitePool,
    account_id: i64,
    amount: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE account SET balance = balance + ? WHERE id = ?")
        .bind(amount)
        .bind(account_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Get full account details by account ID
pub async fn get_account(
    pool: &SqlitePool,
    account_id: i64,
) -> Result<Option<(i64, i64, i64, f64)>, sqlx::Error> {
    let row = sqlx::query("SELECT id, discord_id, currency_id, CAST(balance AS REAL) as balance FROM account WHERE id = ?")
        .bind(account_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| (
        r.get::<i64, _>("id"),
        r.get::<i64, _>("discord_id"),
        r.get::<i64, _>("currency_id"),
        r.get::<f64, _>("balance"),
    )))
}

/// Set account balance to a specific amount by account ID
pub async fn set_balance(
    pool: &SqlitePool,
    account_id: i64,
    balance: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE account SET balance = ? WHERE id = ?")
        .bind(balance)
        .bind(account_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Get Discord user ID from account ID
pub async fn get_discord_id_by_account_id(
    pool: &SqlitePool,
    account_id: i64,
) -> Result<Option<i64>, sqlx::Error> {
    let row = sqlx::query("SELECT CAST(discord_id AS INTEGER) as discord_id FROM account WHERE id = ?")
        .bind(account_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get::<i64, _>("discord_id")))
}

/// Add balance to an account by discord_id and currency_id
pub async fn add_balance(
    pool: &SqlitePool,
    discord_id: i64,
    currency_id: i64,
    amount: f64,
) -> Result<(), sqlx::Error> {
    // First, try to get or create the account
    if let None = get_account_balance(pool, discord_id, currency_id).await? {
        create_account(pool, discord_id, currency_id).await?;
    }

    sqlx::query("UPDATE account SET balance = balance + ? WHERE discord_id = ? AND currency_id = ?")
        .bind(amount)
        .bind(discord_id)
        .bind(currency_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Get total balance across all accounts for a currency
pub async fn get_total_balance(
    pool: &SqlitePool,
    currency_id: i64,
) -> Result<Option<f64>, sqlx::Error> {
    let row = sqlx::query("SELECT CAST(SUM(CAST(balance AS REAL)) AS DOUBLE) as total FROM account WHERE currency_id = ?")
        .bind(currency_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.and_then(|r| r.get::<Option<f64>, _>("total")))
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

        // Create tables
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

        pool
    }

    #[tokio::test]
    async fn test_create_account() {
        let pool = setup_test_db().await;

        // Create a currency first
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test Currency', 'TST')")
            .execute(&pool)
            .await
            .unwrap();

        let account_id = create_account(&pool, 123456789, 1).await.unwrap();
        assert!(account_id > 0);
    }

    #[tokio::test]
    async fn test_get_account_balance() {
        let pool = setup_test_db().await;

        // Create currency and account
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test Currency', 'TST')")
            .execute(&pool)
            .await
            .unwrap();
        create_account(&pool, 123456789, 1).await.unwrap();

        let balance = get_account_balance(&pool, 123456789, 1).await.unwrap();
        assert_eq!(balance, Some(0.0));
    }

    #[tokio::test]
    async fn test_update_balance() {
        let pool = setup_test_db().await;

        // Create currency and account
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test Currency', 'TST')")
            .execute(&pool)
            .await
            .unwrap();
        let account_id = create_account(&pool, 123456789, 1).await.unwrap();

        // Update balance
        update_balance(&pool, account_id, 100.0).await.unwrap();

        let balance = get_account_balance(&pool, 123456789, 1).await.unwrap();
        assert_eq!(balance, Some(100.0));
    }

    #[tokio::test]
    async fn test_get_account_id() {
        let pool = setup_test_db().await;

        // Create currency and account
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test Currency', 'TST')")
            .execute(&pool)
            .await
            .unwrap();
        let created_id = create_account(&pool, 123456789, 1).await.unwrap();

        let fetched_id = get_account_id(&pool, 123456789, 1).await.unwrap();
        assert_eq!(fetched_id, Some(created_id));
    }

    #[tokio::test]
    async fn test_add_balance() {
        let pool = setup_test_db().await;

        // Create currency and account
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test Currency', 'TST')")
            .execute(&pool)
            .await
            .unwrap();
        create_account(&pool, 123456789, 1).await.unwrap();

        // Add balance
        add_balance(&pool, 123456789, 1, 50.0).await.unwrap();
        add_balance(&pool, 123456789, 1, 25.0).await.unwrap();

        let balance = get_account_balance(&pool, 123456789, 1).await.unwrap();
        assert_eq!(balance, Some(75.0));
    }
}
