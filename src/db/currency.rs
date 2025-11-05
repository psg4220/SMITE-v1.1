use sqlx::mysql::MySqlPool;
use sqlx::Row;

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

/// Get currency creation date
pub async fn get_currency_date(
    pool: &MySqlPool,
    currency_id: i64,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT DATE_FORMAT(date_created, '%Y-%m-%d') as date_str FROM currency WHERE id = ?")
        .bind(currency_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get::<String, _>("date_str")))
}

/// Get all currencies for a guild with optional sorting
/// sort_by: "oldest" (default) or "recent"
pub async fn get_currencies_by_guild_sorted(
    pool: &MySqlPool,
    guild_id: i64,
    sort_by: &str,
) -> Result<Vec<(i64, String, String)>, sqlx::Error> {
    let query = if sort_by.to_lowercase() == "recent" {
        "SELECT id, name, ticker FROM currency WHERE guild_id = ? ORDER BY date_created DESC"
    } else {
        "SELECT id, name, ticker FROM currency WHERE guild_id = ? ORDER BY date_created ASC"
    };

    sqlx::query_as::<_, (i64, String, String)>(query)
        .bind(guild_id)
        .fetch_all(pool)
        .await
}

/// Get paginated currencies (all currencies) with optional sorting
/// sort_by: "oldest" (default) or "recent"
/// Returns: (currencies, total_count)
pub async fn get_currencies_paginated(
    pool: &MySqlPool,
    sort_by: &str,
    page: usize,
    page_size: usize,
) -> Result<(Vec<(i64, String, String)>, i64), sqlx::Error> {
    // Get total count
    let count_row = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM currency"
    )
    .fetch_one(pool)
    .await?;

    // Calculate offset
    let offset = (page - 1) * page_size;

    // Get paginated results
    let query = if sort_by.to_lowercase() == "recent" {
        "SELECT id, name, ticker FROM currency ORDER BY date_created DESC LIMIT ? OFFSET ?"
    } else {
        "SELECT id, name, ticker FROM currency ORDER BY date_created ASC LIMIT ? OFFSET ?"
    };

    let currencies = sqlx::query_as::<_, (i64, String, String)>(query)
        .bind(page_size as i64)
        .bind(offset as i64)
        .fetch_all(pool)
        .await?;

    Ok((currencies, count_row))
}
