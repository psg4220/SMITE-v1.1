use sqlx::sqlite::SqlitePool;
use sqlx::Row;

/// Normalize currency pair to canonical order (alphabetically by ticker)
/// Returns (base_currency_id, quote_currency_id, is_reversed)
pub async fn normalize_pair(
    pool: &SqlitePool,
    currency_id_1: i64,
    currency_id_2: i64,
) -> Result<(i64, i64, bool), sqlx::Error> {
    let ticker1 = sqlx::query_scalar::<_, String>("SELECT ticker FROM currency WHERE id = ?")
        .bind(currency_id_1)
        .fetch_optional(pool)
        .await?
        .unwrap_or_default();
    
    let ticker2 = sqlx::query_scalar::<_, String>("SELECT ticker FROM currency WHERE id = ?")
        .bind(currency_id_2)
        .fetch_optional(pool)
        .await?
        .unwrap_or_default();
    
    if ticker1 <= ticker2 {
        Ok((currency_id_1, currency_id_2, false))
    } else {
        Ok((currency_id_2, currency_id_1, true))
    }
}

/// Add a price log entry for a currency pair
/// base_currency_id and quote_currency_id should be in canonical order (alphabetically sorted by ticker)
pub async fn add_price_log(
    pool: &SqlitePool,
    base_currency_id: i64,
    quote_currency_id: i64,
    price: f64,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query("INSERT INTO tradelog (base_currency_id, quote_currency_id, price) VALUES (?, ?, ?)")
        .bind(base_currency_id)
        .bind(quote_currency_id)
        .bind(price)
        .execute(pool)
        .await?;

    Ok(result.last_insert_rowid() as i64)
}

/// Get latest price for a currency pair (returns price, base_id, quote_id, and whether order was reversed from request)
/// Returns: (price, is_reversed)
pub async fn get_latest_price_for_pair(
    pool: &SqlitePool,
    base_currency_id: i64,
    quote_currency_id: i64,
) -> Result<Option<(f64, bool)>, sqlx::Error> {
    // First, get the price as a string to handle DECIMAL properly
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT CAST(price AS TEXT) as price_str FROM tradelog WHERE base_currency_id = ? AND quote_currency_id = ? ORDER BY date_created DESC LIMIT 1"
    )
    .bind(base_currency_id)
    .bind(quote_currency_id)
    .fetch_optional(pool)
    .await?;

    // Convert the string back to f64
    match row {
        Some((price_str,)) => {
            let price = price_str.parse::<f64>()
                .map_err(|e| sqlx::Error::Decode(e.into()))?;
            Ok(Some((price, false)))
        },
        None => Ok(None)
    }
}

/// Get latest price with reverse checking (if not found in canonical order, tries reversed)
/// Returns: (price, is_reversed)
pub async fn get_latest_price_bidirectional(
    pool: &SqlitePool,
    currency_id_1: i64,
    currency_id_2: i64,
) -> Result<Option<(f64, bool)>, sqlx::Error> {
    // Try canonical order first
    if let Some((price, _)) = get_latest_price_for_pair(pool, currency_id_1, currency_id_2).await? {
        return Ok(Some((price, false)));
    }
    
    // Try reversed order
    if let Some((price, _)) = get_latest_price_for_pair(pool, currency_id_2, currency_id_1).await? {
        return Ok(Some((1.0 / price, true)));
    }
    
    Ok(None)
}

/// Get all price logs for a currency pair
pub async fn get_price_logs_for_pair(
    pool: &SqlitePool,
    base_currency_id: i64,
    quote_currency_id: i64,
) -> Result<Vec<(i64, i64, i64, f64)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, i64, f64)>(
        "SELECT id, base_currency_id, quote_currency_id, price FROM tradelog WHERE base_currency_id = ? AND quote_currency_id = ? ORDER BY date_created DESC"
    )
    .bind(base_currency_id)
    .bind(quote_currency_id)
    .fetch_all(pool)
    .await
}

/// Get price logs for a currency pair with timestamps (for charting)
/// Returns: (id, price, date_created as string)
pub async fn get_price_logs_with_timestamps(
    pool: &SqlitePool,
    base_currency_id: i64,
    quote_currency_id: i64,
) -> Result<Vec<(i64, f64, String)>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, CAST(price AS TEXT) as price_str, strftime('%Y-%m-%d %H:%M:%S', date_created) as date_str FROM tradelog WHERE base_currency_id = ? AND quote_currency_id = ? ORDER BY date_created ASC"
    )
    .bind(base_currency_id)
    .bind(quote_currency_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let id: i64 = row.get(0);
            let price_str: String = row.get(1);
            let date_str: String = row.get(2);
            let price = price_str.parse::<f64>().ok()?;
            Some((id, price, date_str))
        })
        .collect())
}

/// Get price logs for a currency pair within a date range
pub async fn get_price_logs_in_range(
    pool: &SqlitePool,
    base_currency_id: i64,
    quote_currency_id: i64,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<(i64, f64, String)>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, CAST(price AS TEXT) as price_str, strftime('%Y-%m-%d %H:%M:%S', date_created) as date_str FROM tradelog WHERE base_currency_id = ? AND quote_currency_id = ? AND date_created BETWEEN ? AND ? ORDER BY date_created ASC"
    )
    .bind(base_currency_id)
    .bind(quote_currency_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter()
        .filter_map(|row| {
            use sqlx::Row;
            let id: i64 = row.get(0);
            let price_str: String = row.get(1);
            let date_str: String = row.get(2);
            let price = price_str.parse::<f64>().ok()?;
            Some((id, price, date_str))
        })
        .collect())
}

/// Calculate VWAP (Volume Weighted Average Price) for a currency pair
pub async fn calculate_vwap(
    pool: &SqlitePool,
    base_currency_id: i64,
    quote_currency_id: i64,
    timeframe: &str,
) -> Result<Option<f64>, sqlx::Error> {
    // Convert MySQL interval to SQLite modifier
    // "1 DAY" -> "-1 day"
    let sqlite_modifier = format!("-{}", timeframe.to_lowercase());

    let sql = format!(
        "SELECT 
            COALESCE(CAST(SUM(CAST(cs.taker_amount AS REAL)) AS TEXT), '0') as total_taker,
            COALESCE(CAST(SUM(CAST(cs.maker_amount AS REAL)) AS TEXT), '0') as total_maker
         FROM currency_swap cs
         WHERE cs.maker_currency_id = ? 
           AND cs.taker_currency_id = ?
           AND cs.status = 'accepted'
           AND cs.date_created >= datetime('now', '{}')",
        sqlite_modifier
    );
    
    let result: Option<(String, String)> = sqlx::query_as(&sql)
        .bind(base_currency_id)
        .bind(quote_currency_id)
        .fetch_optional(pool)
        .await?;

    match result {
        Some((total_taker_str, total_maker_str)) => {
            let total_taker: f64 = total_taker_str.parse().unwrap_or(0.0);
            let total_maker: f64 = total_maker_str.parse().unwrap_or(0.0);

            if total_maker > 0.0 {
                Ok(Some(total_taker / total_maker))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

/// Get all latest prices, optionally filtered by base or quote ticker
pub async fn get_latest_prices_with_filter(
    pool: &SqlitePool,
    filter_base: Option<&str>,
    filter_quote: Option<&str>,
) -> Result<Vec<(String, String, f64)>, sqlx::Error> {
    let mut query = String::from(
        "SELECT 
            c1.ticker as base_ticker,
            c2.ticker as quote_ticker,
            (SELECT CAST(price AS TEXT) FROM tradelog 
             WHERE base_currency_id = tl.base_currency_id 
             AND quote_currency_id = tl.quote_currency_id 
             ORDER BY date_created DESC LIMIT 1) as latest_price,
            MAX(tl.date_created) as max_date
         FROM tradelog tl
         JOIN currency c1 ON tl.base_currency_id = c1.id
         JOIN currency c2 ON tl.quote_currency_id = c2.id
         WHERE 1=1"
    );

    if let Some(base) = filter_base {
        query.push_str(&format!(" AND c1.ticker = UPPER('{}')", base.to_uppercase()));
    }

    if let Some(quote) = filter_quote {
        query.push_str(&format!(" AND c2.ticker = UPPER('{}')", quote.to_uppercase()));
    }

    query.push_str(" GROUP BY tl.base_currency_id, tl.quote_currency_id ORDER BY max_date DESC");

    let rows = sqlx::query(&query).fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let base_ticker: String = row.get(0);
            let quote_ticker: String = row.get(1);
            let price_str: String = row.get(2);
            let price = price_str.parse::<f64>().ok()?;
            Some((base_ticker, quote_ticker, price))
        })
        .collect())
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
            "CREATE TABLE IF NOT EXISTS tradelog (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                base_currency_id INTEGER NOT NULL,
                quote_currency_id INTEGER NOT NULL,
                price REAL NOT NULL,
                date_created TEXT DEFAULT (datetime('now'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_add_price_log() {
        let pool = setup_test_db().await;

        // Create currencies
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Currency A', 'CRA')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (2, 'Currency B', 'CRB')")
            .execute(&pool)
            .await
            .unwrap();

        let log_id = add_price_log(&pool, 1, 2, 2.5).await.unwrap();
        assert!(log_id > 0);
    }

    #[tokio::test]
    async fn test_get_latest_price_for_pair() {
        let pool = setup_test_db().await;

        // Create currencies
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Currency A', 'CRA')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (2, 'Currency B', 'CRB')")
            .execute(&pool)
            .await
            .unwrap();

        // Add a single price log
        add_price_log(&pool, 1, 2, 3.14).await.unwrap();

        let result = get_latest_price_for_pair(&pool, 1, 2).await.unwrap();
        assert!(result.is_some());
        let (price, _inverted) = result.unwrap();
        assert_eq!(price, 3.14);
    }

    #[tokio::test]
    async fn test_normalize_pair() {
        let pool = setup_test_db().await;

        // Create currencies
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Currency A', 'CRA')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (2, 'Currency B', 'CRB')")
            .execute(&pool)
            .await
            .unwrap();

        // Add a price log to establish the pair order
        add_price_log(&pool, 1, 2, 2.0).await.unwrap();

        // Test normalize_pair returns the original order
        let (base, quote, inverted) = normalize_pair(&pool, 1, 2).await.unwrap();
        assert_eq!(base, 1);
        assert_eq!(quote, 2);
        assert!(!inverted);

        // Test inverted order
        let (base, quote, inverted) = normalize_pair(&pool, 2, 1).await.unwrap();
        assert_eq!(base, 1);
        assert_eq!(quote, 2);
        assert!(inverted);
    }
}
