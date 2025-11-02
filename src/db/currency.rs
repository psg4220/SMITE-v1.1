use sqlx::mysql::MySqlPool;

/// Create a new currency for a guild
pub async fn create_currency(
    pool: &MySqlPool,
    guild_id: i64,
    name: &str,
    ticker: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (?, ?, ?)")
        .bind(guild_id)
        .bind(name)
        .bind(ticker)
        .execute(pool)
        .await?;

    Ok(result.last_insert_id() as i64)
}

/// Get currency by guild ID
pub async fn get_currency_by_guild(pool: &MySqlPool, guild_id: i64) -> Result<Option<(i64, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, ticker FROM currency WHERE guild_id = ?"
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await
}

/// Get currency by ID
pub async fn get_currency_by_id(pool: &MySqlPool, currency_id: i64) -> Result<Option<(i64, i64, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, String, String)>(
        "SELECT id, guild_id, name, ticker FROM currency WHERE id = ?"
    )
    .bind(currency_id)
    .fetch_optional(pool)
    .await
}

/// Get currency by ticker (searches across all guilds)
pub async fn get_currency_by_ticker(pool: &MySqlPool, ticker: &str) -> Result<Option<(i64, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, ticker FROM currency WHERE UPPER(ticker) = UPPER(?)"
    )
    .bind(ticker)
    .fetch_optional(pool)
    .await
}

/// Get currency by ticker including guild_id
pub async fn get_currency_by_ticker_with_guild(pool: &MySqlPool, ticker: &str) -> Result<Option<(i64, i64, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, String, String)>(
        "SELECT id, guild_id, name, ticker FROM currency WHERE UPPER(ticker) = UPPER(?)"
    )
    .bind(ticker)
    .fetch_optional(pool)
    .await
}
