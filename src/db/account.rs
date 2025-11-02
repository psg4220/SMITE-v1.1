use sqlx::mysql::MySqlPool;
use sqlx::Row;

/// Create a new account for a user
pub async fn create_account(
    pool: &MySqlPool,
    discord_id: i64,
    currency_id: i64,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query("INSERT INTO account (discord_id, currency_id, balance) VALUES (?, ?, 0.0)")
        .bind(discord_id)
        .bind(currency_id)
        .execute(pool)
        .await?;

    Ok(result.last_insert_id() as i64)
}

/// Get account balance by Discord user ID and currency ID
pub async fn get_account_balance(
    pool: &MySqlPool,
    discord_id: i64,
    currency_id: i64,
) -> Result<Option<f64>, sqlx::Error> {
    let row = sqlx::query("SELECT CAST(balance AS DOUBLE) as balance FROM account WHERE discord_id = ? AND currency_id = ?")
        .bind(discord_id)
        .bind(currency_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get::<f64, _>("balance")))
}

/// Get account ID by Discord user ID and currency ID
pub async fn get_account_id(
    pool: &MySqlPool,
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
    pool: &MySqlPool,
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
    pool: &MySqlPool,
    account_id: i64,
) -> Result<Option<(i64, i64, i64, f64)>, sqlx::Error> {
    let row = sqlx::query("SELECT id, discord_id, currency_id, CAST(balance AS DOUBLE) as balance FROM account WHERE id = ?")
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
    pool: &MySqlPool,
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
    pool: &MySqlPool,
    account_id: i64,
) -> Result<Option<i64>, sqlx::Error> {
    let row = sqlx::query("SELECT CAST(discord_id AS SIGNED) as discord_id FROM account WHERE id = ?")
        .bind(account_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get::<i64, _>("discord_id")))
}

/// Add balance to an account by discord_id and currency_id
pub async fn add_balance(
    pool: &MySqlPool,
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
