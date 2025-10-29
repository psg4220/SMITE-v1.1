use sqlx::mysql::MySqlPool;

/// Normalize currency pair to canonical order (alphabetically by ticker)
/// Returns (base_currency_id, quote_currency_id, is_reversed)
pub async fn normalize_pair(
    pool: &MySqlPool,
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
    pool: &MySqlPool,
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

    Ok(result.last_insert_id() as i64)
}

/// Get latest price for a currency pair (returns price, base_id, quote_id, and whether order was reversed from request)
/// Returns: (price, is_reversed)
pub async fn get_latest_price_for_pair(
    pool: &MySqlPool,
    base_currency_id: i64,
    quote_currency_id: i64,
) -> Result<Option<(f64, bool)>, sqlx::Error> {
    // First, get the price as a string to handle DECIMAL properly
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT CAST(price AS CHAR) as price_str FROM tradelog WHERE base_currency_id = ? AND quote_currency_id = ? ORDER BY date_created DESC LIMIT 1"
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
    pool: &MySqlPool,
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
    pool: &MySqlPool,
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

/// Get price logs for a currency pair within a date range
pub async fn get_price_logs_in_range(
    pool: &MySqlPool,
    base_currency_id: i64,
    quote_currency_id: i64,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<(i64, i64, i64, f64)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, i64, f64)>(
        "SELECT id, base_currency_id, quote_currency_id, price FROM tradelog WHERE base_currency_id = ? AND quote_currency_id = ? AND date_created BETWEEN ? AND ? ORDER BY date_created DESC"
    )
    .bind(base_currency_id)
    .bind(quote_currency_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
}
