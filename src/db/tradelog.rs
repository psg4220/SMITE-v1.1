use sqlx::mysql::MySqlPool;

/// Add a price log entry for a currency
pub async fn add_price_log(
    pool: &MySqlPool,
    currency_id: i64,
    price: f64,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query("INSERT INTO tradelog (currency_id, price) VALUES (?, ?)")
        .bind(currency_id)
        .bind(price)
        .execute(pool)
        .await?;

    Ok(result.last_insert_id() as i64)
}

/// Get all price logs for a currency
pub async fn get_price_logs(
    pool: &MySqlPool,
    currency_id: i64,
) -> Result<Vec<(i64, i64, f64)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, f64)>(
        "SELECT id, currency_id, price FROM tradelog WHERE currency_id = ? ORDER BY date_created DESC"
    )
    .bind(currency_id)
    .fetch_all(pool)
    .await
}

/// Get latest price for a currency
pub async fn get_latest_price(
    pool: &MySqlPool,
    currency_id: i64,
) -> Result<Option<f64>, sqlx::Error> {
    sqlx::query_scalar::<_, f64>(
        "SELECT price FROM tradelog WHERE currency_id = ? ORDER BY date_created DESC LIMIT 1"
    )
    .bind(currency_id)
    .fetch_optional(pool)
    .await
}

/// Get price logs for a currency within a date range
pub async fn get_price_logs_in_range(
    pool: &MySqlPool,
    currency_id: i64,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<(i64, i64, f64)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, f64)>(
        "SELECT id, currency_id, price FROM tradelog WHERE currency_id = ? AND date_created BETWEEN ? AND ? ORDER BY date_created DESC"
    )
    .bind(currency_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
}
