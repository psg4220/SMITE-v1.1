use sqlx::sqlite::SqlitePool;
use sqlx::Row;

/// Get encrypted API token for a currency
/// type_id: 1 = UnbelievaBoat
pub async fn get_api_token(     
    pool: &SqlitePool,                           
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
    pool: &SqlitePool,
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
            "CREATE TABLE IF NOT EXISTS api_type (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                date_created TEXT DEFAULT (datetime('now'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS api_token (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                currency_id INTEGER NOT NULL,
                api_type_id INTEGER NOT NULL,
                encrypted_token TEXT NOT NULL,
                date_created TEXT DEFAULT (datetime('now')),
                date_updated TEXT DEFAULT (datetime('now'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert default api type
        sqlx::query("INSERT INTO api_type (id, name) VALUES (1, 'unbelievaboat')")
            .execute(&pool)
            .await
            .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_store_api_token() {
        let pool = setup_test_db().await;

        // Create a currency
        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test', 'TST')")
            .execute(&pool)
            .await
            .unwrap();

        let result = store_api_token(&pool, 1, 1, "encrypted_token_value").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_api_token() {
        let pool = setup_test_db().await;

        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test', 'TST')")
            .execute(&pool)
            .await
            .unwrap();

        store_api_token(&pool, 1, 1, "my_secret_token").await.unwrap();

        let token = get_api_token(&pool, 1, 1).await.unwrap();
        assert!(token.is_some());
        assert_eq!(token.unwrap(), "my_secret_token");
    }

    #[tokio::test]
    async fn test_update_api_token() {
        let pool = setup_test_db().await;

        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test', 'TST')")
            .execute(&pool)
            .await
            .unwrap();

        store_api_token(&pool, 1, 1, "initial_token").await.unwrap();
        store_api_token(&pool, 1, 1, "updated_token").await.unwrap();

        let token = get_api_token(&pool, 1, 1).await.unwrap();
        assert_eq!(token.unwrap(), "updated_token");
    }

    #[tokio::test]
    async fn test_get_nonexistent_token() {
        let pool = setup_test_db().await;

        sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (1, 'Test', 'TST')")
            .execute(&pool)
            .await
            .unwrap();

        let token = get_api_token(&pool, 1, 1).await.unwrap();
        assert!(token.is_none());
    }
}
