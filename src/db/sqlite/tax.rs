use sqlx::sqlite::SqlitePool;

/// Get tax account with currency guild_id
pub async fn get_tax_account_with_guild(
    pool: &SqlitePool,
    currency_id: i64,
) -> Result<Option<(i64, i64, f64, i32, i64)>, sqlx::Error> {
    let result: Option<(i64, i64, String, i32, i64)> = sqlx::query_as(
        "SELECT ta.id, ta.currency_id, CAST(ta.balance AS CHAR) as balance_str, ta.tax_percentage, c.guild_id 
         FROM tax_account ta 
         JOIN currency c ON ta.currency_id = c.id 
         WHERE ta.currency_id = ?"
    )
    .bind(currency_id)
    .fetch_optional(pool)
    .await?;

    // Convert the string back to f64
    match result {
        Some((id, curr_id, balance_str, tax_pct, guild_id)) => {
            let balance = balance_str.parse::<f64>()
                .map_err(|e| sqlx::Error::Decode(e.into()))?;
            Ok(Some((id, curr_id, balance, tax_pct, guild_id)))
        },
        None => Ok(None),
    }
}

/// Get or create tax account for a currency
pub async fn get_tax_account(
    pool: &SqlitePool,
    currency_id: i64,
) -> Result<Option<(i64, i64, f64, i32)>, sqlx::Error> {
    let result: Option<(i64, i64, String, i32)> = sqlx::query_as(
        "SELECT id, currency_id, CAST(balance AS CHAR) as balance_str, tax_percentage FROM tax_account WHERE currency_id = ?"
    )
    .bind(currency_id)
    .fetch_optional(pool)
    .await?;

    // Convert the string back to f64
    match result {
        Some((id, curr_id, balance_str, tax_pct)) => {
            let balance = balance_str.parse::<f64>()
                .map_err(|e| sqlx::Error::Decode(e.into()))?;
            Ok(Some((id, curr_id, balance, tax_pct)))
        },
        None => Ok(None),
    }
}

/// Create a new tax account for a currency
pub async fn create_tax_account(
    pool: &SqlitePool,
    currency_id: i64,
    tax_percentage: i32,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO tax_account (currency_id, balance, tax_percentage) VALUES (?, 0, ?)"
    )
    .bind(currency_id)
    .bind(tax_percentage)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid() as i64)
}

/// Update tax percentage for a currency
pub async fn set_tax_percentage(
    pool: &SqlitePool,
    currency_id: i64,
    tax_percentage: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE tax_account SET tax_percentage = ? WHERE currency_id = ?"
    )
    .bind(tax_percentage)
    .bind(currency_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Add tax to an account
pub async fn add_tax(
    pool: &SqlitePool,
    currency_id: i64,
    amount: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE tax_account SET balance = balance + ? WHERE currency_id = ?"
    )
    .bind(amount)
    .bind(currency_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Collect (withdraw) tax from an account
pub async fn collect_tax(
    pool: &SqlitePool,
    currency_id: i64,
    amount: f64,
) -> Result<f64, sqlx::Error> {
    // Get current balance - cast DECIMAL to CHAR for proper handling
    let tax_account: (i64, i64, String, i32) = sqlx::query_as(
        "SELECT id, currency_id, CAST(balance AS CHAR) as balance_str, tax_percentage FROM tax_account WHERE currency_id = ?"
    )
    .bind(currency_id)
    .fetch_one(pool)
    .await?;

    let current_balance = tax_account.2.parse::<f64>()
        .map_err(|e| sqlx::Error::Decode(e.into()))?;
    
    let collect_amount = if amount >= current_balance {
        current_balance
    } else {
        amount
    };

    // Deduct from tax account
    sqlx::query(
        "UPDATE tax_account SET balance = balance - ? WHERE currency_id = ?"
    )
    .bind(collect_amount)
    .bind(currency_id)
    .execute(pool)
    .await?;

    Ok(collect_amount)
}

/// Get tax percentage for a currency
pub async fn get_tax_percentage(
    pool: &SqlitePool,
    currency_id: i64,
) -> Result<Option<i32>, sqlx::Error> {
    let result = sqlx::query_scalar::<_, i32>(
        "SELECT tax_percentage FROM tax_account WHERE currency_id = ?"
    )
    .bind(currency_id)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

/// Get total tax balance for a currency
pub async fn get_total_tax_balance(
    pool: &SqlitePool,
    currency_id: i64,
) -> Result<Option<f64>, sqlx::Error> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT CAST(balance AS CHAR) as balance_str FROM tax_account WHERE currency_id = ?"
    )
    .bind(currency_id)
    .fetch_optional(pool)
    .await?;

    match result {
        Some((balance_str,)) => {
            let balance = balance_str.parse::<f64>()
                .map_err(|e| sqlx::Error::Decode(e.into()))?;
            Ok(Some(balance))
        },
        None => Ok(Some(0.0)),
    }
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
            "CREATE TABLE IF NOT EXISTS tax_account (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                currency_id INTEGER UNIQUE NOT NULL,
                balance REAL NOT NULL DEFAULT 0.0,
                tax_percentage INTEGER NOT NULL DEFAULT 0,
                date_created TEXT DEFAULT (datetime('now')),
                date_updated TEXT DEFAULT (datetime('now'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_tax_account() {
        let pool = setup_test_db().await;

        // Create a currency first
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test', 'TST')")
            .execute(&pool)
            .await
            .unwrap();

        let tax_id = create_tax_account(&pool, 1, 5).await.unwrap();
        assert!(tax_id > 0);
    }

    #[tokio::test]
    async fn test_get_tax_percentage() {
        let pool = setup_test_db().await;

        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test', 'TST')")
            .execute(&pool)
            .await
            .unwrap();
        create_tax_account(&pool, 1, 10).await.unwrap();

        let percentage = get_tax_percentage(&pool, 1).await.unwrap();
        assert_eq!(percentage, Some(10));
    }

    #[tokio::test]
    async fn test_set_tax_percentage() {
        let pool = setup_test_db().await;

        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test', 'TST')")
            .execute(&pool)
            .await
            .unwrap();
        create_tax_account(&pool, 1, 5).await.unwrap();

        set_tax_percentage(&pool, 1, 15).await.unwrap();

        let percentage = get_tax_percentage(&pool, 1).await.unwrap();
        assert_eq!(percentage, Some(15));
    }

    #[tokio::test]
    async fn test_add_tax() {
        let pool = setup_test_db().await;

        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test', 'TST')")
            .execute(&pool)
            .await
            .unwrap();
        create_tax_account(&pool, 1, 5).await.unwrap();

        add_tax(&pool, 1, 100.0).await.unwrap();
        add_tax(&pool, 1, 50.0).await.unwrap();

        let balance = get_total_tax_balance(&pool, 1).await.unwrap();
        assert_eq!(balance, Some(150.0));
    }
}
