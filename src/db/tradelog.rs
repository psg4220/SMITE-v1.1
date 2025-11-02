use sqlx::mysql::MySqlPool;
use sqlx::Row;

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

/// Get price logs for a currency pair with timestamps (for charting)
/// Returns: (id, price, date_created as string)
pub async fn get_price_logs_with_timestamps(
    pool: &MySqlPool,
    base_currency_id: i64,
    quote_currency_id: i64,
) -> Result<Vec<(i64, f64, String)>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, CAST(price AS CHAR) as price_str, DATE_FORMAT(date_created, '%Y-%m-%d %H:%i:%s') as date_str FROM tradelog WHERE base_currency_id = ? AND quote_currency_id = ? ORDER BY date_created ASC"
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

/// Calculate VWAP (Volume Weighted Average Price) for a currency pair
/// Queries accepted swaps from currency_swap table within the specified timeframe
/// Timeframe examples: "1 MINUTE", "1 HOUR", "1 DAY", "7 DAY", "30 DAY", "1 YEAR"
/// Returns the VWAP as f64, or None if no accepted swaps exist in the timeframe
pub async fn calculate_vwap(
    pool: &MySqlPool,
    base_currency_id: i64,
    quote_currency_id: i64,
    timeframe: &str,
) -> Result<Option<f64>, sqlx::Error> {
    // Query accepted swaps for this currency pair
    // Price = taker_amount / maker_amount (quote / base)
    // Volume = maker_amount (base currency volume)
    // VWAP = Σ((taker_amount / maker_amount) × maker_amount) / Σ(maker_amount)
    //      = Σ(taker_amount) / Σ(maker_amount)
    let sql = format!(
        "SELECT 
            COALESCE(CAST(SUM(CAST(cs.taker_amount AS DECIMAL(20,8))) AS CHAR), '0') as total_taker,
            COALESCE(CAST(SUM(CAST(cs.maker_amount AS DECIMAL(20,8))) AS CHAR), '0') as total_maker
         FROM currency_swap cs
         WHERE cs.maker_currency_id = ? 
           AND cs.taker_currency_id = ?
           AND cs.status = 'accepted'
           AND cs.date_created >= DATE_SUB(NOW(), INTERVAL {})",
        timeframe
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
