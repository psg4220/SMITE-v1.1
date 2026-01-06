use sqlx::mysql::MySqlPool;
use sqlx::Row;

/// Get encrypted API token for a currency
/// type_id: 1 = UnbelievaBoat
pub async fn get_api_token(     
    pool: &MySqlPool,                           
    currency_id: i64,
    api_type_id: i32,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT encrypted_token FROM api_token WHERE currency_id = ? AND api_type_id = ?")
        .bind(currency_id)
        .bind(api_type_id as i8)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get::<String, _>("encrypted_token")))
}

/// Store encrypted API token for a currency
/// api_type_id: 1 = UnbelievaBoat
pub async fn store_api_token(
    pool: &MySqlPool,
    currency_id: i64,
    api_type_id: i32,
    encrypted_token: &str,
) -> Result<(), sqlx::Error> {
    // Check if token already exists
    let existing = sqlx::query("SELECT id FROM api_token WHERE currency_id = ? AND api_type_id = ?")
        .bind(currency_id)
        .bind(api_type_id as i8)
        .fetch_optional(pool)
        .await?;

    if existing.is_some() {
        // Update existing token
        sqlx::query("UPDATE api_token SET encrypted_token = ?, date_updated = CURRENT_TIMESTAMP WHERE currency_id = ? AND api_type_id = ?")
            .bind(encrypted_token)
            .bind(currency_id)
            .bind(api_type_id as i8)
            .execute(pool)
            .await?;
    } else {
        // Insert new token
        sqlx::query("INSERT INTO api_token (currency_id, api_type_id, encrypted_token) VALUES (?, ?, ?)")
            .bind(currency_id)
            .bind(api_type_id as i8)
            .bind(encrypted_token)
            .execute(pool)
            .await?;
    }

    Ok(())
}
