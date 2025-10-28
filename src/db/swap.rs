use sqlx::mysql::MySqlPool;

/// Get a swap by ID (direct query)
/// Returns: (id, maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status)
pub async fn get_swap_by_id(
    pool: &MySqlPool,
    swap_id: i64,
) -> Result<Option<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "SELECT CAST(id AS SIGNED), CAST(maker_id AS SIGNED), CAST(taker_id AS SIGNED), CAST(maker_currency_id AS SIGNED), 
                CAST(taker_currency_id AS SIGNED), CAST(maker_amount AS DOUBLE), CAST(taker_amount AS DOUBLE), status 
         FROM currency_swap WHERE id = ?"
    )
    .bind(swap_id)
    .fetch_optional(pool)
    .await
}

/// Get all pending swaps for a maker (direct query)
pub async fn get_pending_swaps_for_maker(
    pool: &MySqlPool,
    maker_account_id: i64,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "SELECT CAST(id AS SIGNED), CAST(maker_id AS SIGNED), CAST(taker_id AS SIGNED), CAST(maker_currency_id AS SIGNED), 
                CAST(taker_currency_id AS SIGNED), CAST(maker_amount AS DOUBLE), CAST(taker_amount AS DOUBLE), status 
         FROM currency_swap WHERE maker_id = ? AND status = 'pending'"
    )
    .bind(maker_account_id)
    .fetch_all(pool)
    .await
}

/// Get all pending swaps for a taker (direct query)
pub async fn get_pending_swaps_for_taker(
    pool: &MySqlPool,
    taker_account_id: i64,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "SELECT CAST(id AS SIGNED), CAST(maker_id AS SIGNED), CAST(taker_id AS SIGNED), CAST(maker_currency_id AS SIGNED), 
                CAST(taker_currency_id AS SIGNED), CAST(maker_amount AS DOUBLE), CAST(taker_amount AS DOUBLE), status 
         FROM currency_swap WHERE taker_id = ? AND status = 'pending'"
    )
    .bind(taker_account_id)
    .fetch_all(pool)
    .await
}

/// Get all open swaps (swaps where taker_id is NULL) - direct query
pub async fn get_open_swaps(
    pool: &MySqlPool,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "SELECT CAST(id AS SIGNED), CAST(maker_id AS SIGNED), CAST(taker_id AS SIGNED), CAST(maker_currency_id AS SIGNED), 
                CAST(taker_currency_id AS SIGNED), CAST(maker_amount AS DOUBLE), CAST(taker_amount AS DOUBLE), status 
         FROM currency_swap WHERE taker_id IS NULL AND status = 'pending'"
    )
    .fetch_all(pool)
    .await
}

/// Create a new currency swap (targeted swap)
pub async fn create_swap(
    pool: &MySqlPool,
    maker_id: i64,
    maker_currency_id: i64,
    taker_currency_id: i64,
    maker_amount: f64,
    taker_amount: f64,
    taker_id: i64,
) -> Result<i64, sqlx::Error> {
    // Acquire a single connection to maintain session variables
    let mut conn = pool.acquire().await?;

    sqlx::query(
        "CALL sp_create_swap(?, ?, ?, ?, ?, ?)"
    )
    .bind(maker_id)
    .bind(maker_currency_id)
    .bind(taker_currency_id)
    .bind(maker_amount)
    .bind(taker_amount)
    .bind(taker_id)
    .execute(&mut *conn)
    .await?;

    let swap_id: i64 = sqlx::query_scalar("SELECT CAST(@swap_id AS SIGNED)")
        .fetch_one(&mut *conn)
        .await?;

    Ok(swap_id)
}

/// Create an open currency swap (any user can accept)
pub async fn create_swap_open(
    pool: &MySqlPool,
    maker_id: i64,
    maker_currency_id: i64,
    taker_currency_id: i64,
    maker_amount: f64,
    taker_amount: f64,
) -> Result<i64, sqlx::Error> {
    // Acquire a single connection to maintain session variables
    let mut conn = pool.acquire().await?;

    sqlx::query(
        "CALL sp_create_swap_open(?, ?, ?, ?, ?)"
    )
    .bind(maker_id)
    .bind(maker_currency_id)
    .bind(taker_currency_id)
    .bind(maker_amount)
    .bind(taker_amount)
    .execute(&mut *conn)
    .await?;

    let swap_id: i64 = sqlx::query_scalar("SELECT CAST(@swap_id AS SIGNED)")
        .fetch_one(&mut *conn)
        .await?;

    Ok(swap_id)
}

/// Accept a swap as the taker
pub async fn accept_swap(
    pool: &MySqlPool,
    swap_id: i64,
    taker_id: i64,
    uuid1: &str,
    uuid2: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("CALL sp_accept_swap(?, ?, ?, ?)")
        .bind(swap_id)
        .bind(taker_id)
        .bind(uuid1)
        .bind(uuid2)
        .execute(pool)
        .await?;

    Ok(())
}

/// Complete a swap
pub async fn complete_swap(pool: &MySqlPool, swap_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("CALL sp_complete_swap(?)")
        .bind(swap_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Cancel a swap
pub async fn cancel_swap(pool: &MySqlPool, swap_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("CALL sp_cancel_swap(?)")
        .bind(swap_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Get a swap by ID
pub async fn get_swap(
    pool: &MySqlPool,
    swap_id: i64,
) -> Result<Option<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "CALL sp_get_swap(?)"
    )
    .bind(swap_id)
    .fetch_optional(pool)
    .await
}

/// Get pending swaps by maker ID
pub async fn get_pending_swaps_by_maker(
    pool: &MySqlPool,
    maker_id: i64,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "CALL sp_get_pending_swaps_by_maker(?)"
    )
    .bind(maker_id)
    .fetch_all(pool)
    .await
}

/// Get all swaps by maker ID
pub async fn get_swaps_by_maker(
    pool: &MySqlPool,
    maker_id: i64,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "CALL sp_get_swaps_by_maker(?)"
    )
    .bind(maker_id)
    .fetch_all(pool)
    .await
}

/// Get all swaps by taker ID
pub async fn get_swaps_by_taker(
    pool: &MySqlPool,
    taker_id: i64,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "CALL sp_get_swaps_by_taker(?)"
    )
    .bind(taker_id)
    .fetch_all(pool)
    .await
}

/// Get all pending swaps (admin view)
pub async fn get_all_pending_swaps(
    pool: &MySqlPool,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "CALL sp_get_all_pending_swaps()"
    )
    .fetch_all(pool)
    .await
}

/// Get all open swaps (swaps without a taker)
pub async fn get_all_open_swaps(
    pool: &MySqlPool,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "CALL sp_get_all_open_swaps()"
    )
    .fetch_all(pool)
    .await
}

/// Store swap message ID for later editing
pub async fn store_swap_message(
    pool: &MySqlPool,
    swap_id: i64,
    channel_id: i64,
    message_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO swap_message (swap_id, channel_id, message_id) VALUES (?, ?, ?)"
    )
    .bind(swap_id)
    .bind(channel_id)
    .bind(message_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get swap message ID by swap ID
pub async fn get_swap_message(
    pool: &MySqlPool,
    swap_id: i64,
) -> Result<Option<(i64, i64, i64)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT swap_id, channel_id, message_id FROM swap_message WHERE swap_id = ?"
    )
    .bind(swap_id)
    .fetch_optional(pool)
    .await
}
