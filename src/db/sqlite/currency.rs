use sqlx::sqlite::SqlitePool;
use sqlx::Row;

/// Create a new currency for a guild
pub async fn create_currency(
    pool: &SqlitePool,
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

    Ok(result.last_insert_rowid() as i64)
}

/// Get currency by guild ID
pub async fn get_currency_by_guild(pool: &SqlitePool, guild_id: i64) -> Result<Option<(i64, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, ticker FROM currency WHERE guild_id = ?"
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await
}

/// Get currency by ID
pub async fn get_currency_by_id(pool: &SqlitePool, currency_id: i64) -> Result<Option<(i64, i64, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, String, String)>(
        "SELECT id, guild_id, name, ticker FROM currency WHERE id = ?"
    )
    .bind(currency_id)
    .fetch_optional(pool)
    .await
}

/// Get currency by ticker (searches across all guilds)
pub async fn get_currency_by_ticker(pool: &SqlitePool, ticker: &str) -> Result<Option<(i64, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, ticker FROM currency WHERE UPPER(ticker) = UPPER(?)"
    )
    .bind(ticker)
    .fetch_optional(pool)
    .await
}

/// Get currency by ticker including guild_id
pub async fn get_currency_by_ticker_with_guild(pool: &SqlitePool, ticker: &str) -> Result<Option<(i64, i64, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, String, String)>(
        "SELECT id, guild_id, name, ticker FROM currency WHERE UPPER(ticker) = UPPER(?)"
    )
    .bind(ticker)
    .fetch_optional(pool)
    .await
}

/// Get currency creation date
pub async fn get_currency_date(
    pool: &SqlitePool,
    currency_id: i64,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT strftime('%Y-%m-%d', date_created) as date_str FROM currency WHERE id = ?")
        .bind(currency_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get::<String, _>("date_str")))
}

/// Get all currencies
pub async fn get_all_currencies(pool: &SqlitePool) -> Result<Vec<(i64, String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, ticker FROM currency"
    )
    .fetch_all(pool)
    .await
}

/// Get all currencies for a guild with optional sorting
/// sort_by: "oldest" (default) or "recent"
pub async fn get_currencies_by_guild_sorted(
    pool: &SqlitePool,
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
    pool: &SqlitePool,
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

        pool
    }

    #[tokio::test]
    async fn test_create_currency() {
        let pool = setup_test_db().await;

        let currency_id = create_currency(&pool, 123456789, "Test Dollar", "TSD").await.unwrap();
        assert!(currency_id > 0);
    }

    #[tokio::test]
    async fn test_get_currency_by_id() {
        let pool = setup_test_db().await;

        let currency_id = create_currency(&pool, 123456789, "Test Dollar", "TSD").await.unwrap();

        let result = get_currency_by_id(&pool, currency_id).await.unwrap();
        assert!(result.is_some());
        let (id, guild_id, name, ticker) = result.unwrap();
        assert_eq!(id, currency_id);
        assert_eq!(guild_id, 123456789);
        assert_eq!(name, "Test Dollar");
        assert_eq!(ticker, "TSD");
    }

    #[tokio::test]
    async fn test_get_currency_by_ticker() {
        let pool = setup_test_db().await;

        create_currency(&pool, 123456789, "Test Dollar", "TSD").await.unwrap();

        let result = get_currency_by_ticker(&pool, "TSD").await.unwrap();
        assert!(result.is_some());
        let (id, name, ticker) = result.unwrap();
        assert!(id > 0);
        assert_eq!(name, "Test Dollar");
        assert_eq!(ticker, "TSD");
    }

    #[tokio::test]
    async fn test_get_currency_by_guild() {
        let pool = setup_test_db().await;

        create_currency(&pool, 123456789, "Test Dollar", "TSD").await.unwrap();

        let result = get_currency_by_guild(&pool, 123456789).await.unwrap();
        assert!(result.is_some());
        let (id, name, ticker) = result.unwrap();
        assert!(id > 0);
        assert_eq!(name, "Test Dollar");
        assert_eq!(ticker, "TSD");
    }

    #[tokio::test]
    async fn test_get_currencies_paginated() {
        let pool = setup_test_db().await;

        // Create multiple currencies
        create_currency(&pool, 1, "Currency A", "CRA").await.unwrap();
        create_currency(&pool, 2, "Currency B", "CRB").await.unwrap();
        create_currency(&pool, 3, "Currency C", "CRC").await.unwrap();

        let (currencies, total) = get_currencies_paginated(&pool, "oldest", 1, 2).await.unwrap();
        assert_eq!(total, 3);
        assert_eq!(currencies.len(), 2);
    }
}
