#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;
    use crate::db::traits::DatabaseBackend;
    use crate::db::sqlite::SqliteBackend;
    use std::sync::Arc;

    async fn setup_test_backend() -> Arc<dyn DatabaseBackend> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // Create tables
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
            "CREATE TABLE IF NOT EXISTS account (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                discord_id INTEGER NOT NULL,
                currency_id INTEGER NOT NULL,
                balance REAL NOT NULL DEFAULT 0.0,
                date_created TEXT DEFAULT (datetime('now')),
                date_updated TEXT DEFAULT (datetime('now')),
                UNIQUE (discord_id, currency_id)
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS currency_swap (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                maker_id INTEGER NOT NULL,
                taker_id INTEGER,
                maker_currency_id INTEGER NOT NULL,
                taker_currency_id INTEGER NOT NULL,
                maker_amount REAL NOT NULL,
                taker_amount REAL NOT NULL,
                status TEXT DEFAULT 'pending',
                date_created TEXT DEFAULT (datetime('now')),
                date_updated TEXT DEFAULT (datetime('now'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS \"transaction\" (
                uuid TEXT PRIMARY KEY,
                sender_id INTEGER NOT NULL,
                receiver_id INTEGER NOT NULL,
                amount REAL NOT NULL,
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

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tax_account (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                currency_id INTEGER UNIQUE NOT NULL,
                balance REAL NOT NULL DEFAULT 0.0,
                tax_percentage INTEGER NOT NULL DEFAULT 0,
                date_created TEXT DEFAULT (datetime('now')),
                date_updated TEXT DEFAULT (datetime('now'))
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

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS swap_message (
                swap_id INTEGER PRIMARY KEY,
                channel_id INTEGER NOT NULL,
                message_id INTEGER NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        Arc::new(SqliteBackend::new(pool))
    }

    // Account tests
    #[tokio::test]
    async fn test_create_account() {
        let backend = setup_test_backend().await;
        
        // Create currency first
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        
        // Create account
        let account_id = backend.create_account(123, currency_id).await.unwrap();
        assert!(account_id > 0);
    }

    #[tokio::test]
    async fn test_get_account_balance() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        backend.create_account(123, currency_id).await.unwrap();
        
        let balance = backend.get_account_balance(123, currency_id).await.unwrap();
        assert_eq!(balance, Some(0.0));
    }

    #[tokio::test]
    async fn test_get_account_id() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        let created_id = backend.create_account(123, currency_id).await.unwrap();
        
        let fetched_id = backend.get_account_id(123, currency_id).await.unwrap();
        assert_eq!(fetched_id, Some(created_id));
    }

    #[tokio::test]
    async fn test_add_balance() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        backend.create_account(123, currency_id).await.unwrap();
        
        backend.add_balance(123, currency_id, 100.0).await.unwrap();
        backend.add_balance(123, currency_id, 50.0).await.unwrap();
        
        let balance = backend.get_account_balance(123, currency_id).await.unwrap();
        assert_eq!(balance, Some(150.0));
    }

    #[tokio::test]
    async fn test_get_total_balance() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        backend.create_account(123, currency_id).await.unwrap();
        backend.create_account(456, currency_id).await.unwrap();
        
        backend.add_balance(123, currency_id, 100.0).await.unwrap();
        backend.add_balance(456, currency_id, 200.0).await.unwrap();
        
        let total = backend.get_total_balance(currency_id).await.unwrap();
        assert_eq!(total, Some(300.0));
    }

    #[tokio::test]
    async fn test_get_discord_id_by_account_id() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        let account_id = backend.create_account(123, currency_id).await.unwrap();
        
        let discord_id = backend.get_discord_id_by_account_id(account_id).await.unwrap();
        assert_eq!(discord_id, Some(123));
    }

    // Currency tests
    #[tokio::test]
    async fn test_create_currency() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test Dollar", "TSD").await.unwrap();
        assert!(currency_id > 0);
    }

    #[tokio::test]
    async fn test_get_currency_by_id() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test Dollar", "TSD").await.unwrap();
        
        let result = backend.get_currency_by_id(currency_id).await.unwrap();
        assert!(result.is_some());
        let (id, guild_id, name, ticker) = result.unwrap();
        assert_eq!(id, currency_id);
        assert_eq!(guild_id, 1);
        assert_eq!(name, "Test Dollar");
        assert_eq!(ticker, "TSD");
    }

    #[tokio::test]
    async fn test_get_currency_by_ticker() {
        let backend = setup_test_backend().await;
        
        backend.create_currency(1, "Test Dollar", "TSD").await.unwrap();
        
        let result = backend.get_currency_by_ticker("TSD").await.unwrap();
        assert!(result.is_some());
        let (id, name, ticker) = result.unwrap();
        assert!(id > 0);
        assert_eq!(name, "Test Dollar");
        assert_eq!(ticker, "TSD");
    }

    #[tokio::test]
    async fn test_get_currency_by_guild() {
        let backend = setup_test_backend().await;
        
        backend.create_currency(123, "Test Dollar", "TSD").await.unwrap();
        
        let result = backend.get_currency_by_guild(123).await.unwrap();
        assert!(result.is_some());
        let (id, name, ticker) = result.unwrap();
        assert!(id > 0);
        assert_eq!(name, "Test Dollar");
        assert_eq!(ticker, "TSD");
    }

    #[tokio::test]
    async fn test_get_currencies_paginated() {
        let backend = setup_test_backend().await;
        
        backend.create_currency(1, "Currency A", "CRA").await.unwrap();
        backend.create_currency(2, "Currency B", "CRB").await.unwrap();
        backend.create_currency(3, "Currency C", "CRC").await.unwrap();
        
        let (currencies, total) = backend.get_currencies_paginated("oldest", 1, 2).await.unwrap();
        assert_eq!(total, 3);
        assert_eq!(currencies.len(), 2);
    }

    // Swap tests
    #[tokio::test]
    async fn test_create_swap_targeted() {
        let backend = setup_test_backend().await;
        
        let currency1 = backend.create_currency(1, "Currency A", "CRA").await.unwrap();
        let currency2 = backend.create_currency(2, "Currency B", "CRB").await.unwrap();
        let maker_account = backend.create_account(100, currency1).await.unwrap();
        let taker_account = backend.create_account(200, currency2).await.unwrap();
        
        backend.add_balance(100, currency1, 1000.0).await.unwrap();
        
        let swap_id = backend.create_swap_targeted(maker_account, currency1, currency2, 100.0, 50.0, taker_account)
            .await
            .unwrap();
        
        assert!(swap_id > 0);
        
        let balance = backend.get_account_balance(100, currency1).await.unwrap();
        assert_eq!(balance, Some(900.0));
    }

    #[tokio::test]
    async fn test_create_swap_open() {
        let backend = setup_test_backend().await;
        
        let currency1 = backend.create_currency(1, "Currency A", "CRA").await.unwrap();
        let currency2 = backend.create_currency(2, "Currency B", "CRB").await.unwrap();
        let maker_account = backend.create_account(100, currency1).await.unwrap();
        
        backend.add_balance(100, currency1, 1000.0).await.unwrap();
        
        let swap_id = backend.create_swap_open(maker_account, currency1, currency2, 100.0, 50.0)
            .await
            .unwrap();
        
        assert!(swap_id > 0);
        
        let swap = backend.get_swap_by_id(swap_id).await.unwrap().unwrap();
        assert!(swap.2.is_none()); // taker_id should be None
    }

    #[tokio::test]
    async fn test_get_swap_by_id() {
        let backend = setup_test_backend().await;
        
        let currency1 = backend.create_currency(1, "Currency A", "CRA").await.unwrap();
        let currency2 = backend.create_currency(2, "Currency B", "CRB").await.unwrap();
        let maker_account = backend.create_account(100, currency1).await.unwrap();
        
        backend.add_balance(100, currency1, 1000.0).await.unwrap();
        
        let swap_id = backend.create_swap_open(maker_account, currency1, currency2, 100.0, 50.0)
            .await
            .unwrap();
        
        let swap = backend.get_swap_by_id(swap_id).await.unwrap();
        assert!(swap.is_some());
        let swap = swap.unwrap();
        assert_eq!(swap.0, swap_id);
        assert_eq!(swap.5, 100.0); // maker_amount
        assert_eq!(swap.6, 50.0);  // taker_amount
        assert_eq!(swap.7, "pending");
    }

    #[tokio::test]
    async fn test_cancel_swap() {
        let backend = setup_test_backend().await;
        
        let currency1 = backend.create_currency(1, "Currency A", "CRA").await.unwrap();
        let currency2 = backend.create_currency(2, "Currency B", "CRB").await.unwrap();
        let maker_account = backend.create_account(100, currency1).await.unwrap();
        
        backend.add_balance(100, currency1, 1000.0).await.unwrap();
        
        let swap_id = backend.create_swap_open(maker_account, currency1, currency2, 100.0, 50.0)
            .await
            .unwrap();
        
        backend.cancel_swap(swap_id).await.unwrap();
        
        let balance = backend.get_account_balance(100, currency1).await.unwrap();
        assert_eq!(balance, Some(1000.0)); // Refunded
    }

    #[tokio::test]
    async fn test_store_and_get_swap_message() {
        let backend = setup_test_backend().await;
        
        backend.store_swap_message(1, 123456, 789012).await.unwrap();
        
        let result = backend.get_swap_message(1).await.unwrap();
        assert!(result.is_some());
        let (channel_id, message_id) = result.unwrap();
        assert_eq!(channel_id, 123456);
        assert_eq!(message_id, 789012);
    }

    #[tokio::test]
    async fn test_get_total_swap_maker_amount() {
        let backend = setup_test_backend().await;
        
        let currency1 = backend.create_currency(1, "Currency A", "CRA").await.unwrap();
        let currency2 = backend.create_currency(2, "Currency B", "CRB").await.unwrap();
        let maker_account = backend.create_account(100, currency1).await.unwrap();
        
        backend.add_balance(100, currency1, 1000.0).await.unwrap();
        
        backend.create_swap_open(maker_account, currency1, currency2, 100.0, 50.0).await.unwrap();
        backend.create_swap_open(maker_account, currency1, currency2, 50.0, 25.0).await.unwrap();
        
        let total = backend.get_total_swap_maker_amount(currency1).await.unwrap();
        assert_eq!(total, Some(150.0));
    }

    // Transaction tests
    #[tokio::test]
    async fn test_create_transaction() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        let sender = backend.create_account(100, currency_id).await.unwrap();
        let receiver = backend.create_account(200, currency_id).await.unwrap();
        
        let result = backend.create_transaction("test-uuid-1", sender, receiver, 100.0).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_transaction() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        let sender = backend.create_account(100, currency_id).await.unwrap();
        let receiver = backend.create_account(200, currency_id).await.unwrap();
        
        backend.create_transaction("test-uuid-2", sender, receiver, 250.0).await.unwrap();
        
        let tx = backend.get_transaction("test-uuid-2").await.unwrap();
        assert!(tx.is_some());
    }

    // Tradelog tests
    #[tokio::test]
    async fn test_add_price_log() {
        let backend = setup_test_backend().await;
        
        let currency1 = backend.create_currency(1, "Currency A", "CRA").await.unwrap();
        let currency2 = backend.create_currency(2, "Currency B", "CRB").await.unwrap();
        
        let log_id = backend.add_price_log(currency1, currency2, 2.5).await.unwrap();
        assert!(log_id > 0);
    }

    #[tokio::test]
    async fn test_get_latest_price() {
        let backend = setup_test_backend().await;
        
        let currency1 = backend.create_currency(1, "Currency A", "CRA").await.unwrap();
        let currency2 = backend.create_currency(2, "Currency B", "CRB").await.unwrap();
        
        backend.add_price_log(currency1, currency2, 3.14).await.unwrap();
        
        let price = backend.get_latest_price(currency1, currency2).await.unwrap();
        assert_eq!(price, Some(3.14));
    }

    #[tokio::test]
    async fn test_normalize_pair() {
        let backend = setup_test_backend().await;
        
        let currency1 = backend.create_currency(1, "Currency A", "CRA").await.unwrap();
        let currency2 = backend.create_currency(2, "Currency B", "CRB").await.unwrap();
        
        backend.add_price_log(currency1, currency2, 2.0).await.unwrap();
        
        let (base, quote, inverted) = backend.normalize_pair(currency1, currency2).await.unwrap();
        assert_eq!(base, currency1);
        assert_eq!(quote, currency2);
        assert!(!inverted);
    }

    // Tax tests
    #[tokio::test]
    async fn test_create_tax_account() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        
        let tax_id = backend.create_tax_account(currency_id, 5).await.unwrap();
        assert!(tax_id > 0);
    }

    #[tokio::test]
    async fn test_get_tax_percentage() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        backend.create_tax_account(currency_id, 10).await.unwrap();
        
        let percentage = backend.get_tax_percentage(currency_id).await.unwrap();
        assert_eq!(percentage, Some(10.0));
    }

    #[tokio::test]
    async fn test_set_tax_percentage() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        backend.create_tax_account(currency_id, 5).await.unwrap();
        
        backend.set_tax_percentage(currency_id, 15.0).await.unwrap();
        
        let percentage = backend.get_tax_percentage(currency_id).await.unwrap();
        assert_eq!(percentage, Some(15.0));
    }

    #[tokio::test]
    async fn test_add_tax() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        backend.create_tax_account(currency_id, 5).await.unwrap();
        
        backend.add_tax(currency_id, 100.0).await.unwrap();
        backend.add_tax(currency_id, 50.0).await.unwrap();
        
        let balance = backend.get_total_tax_balance(currency_id).await.unwrap();
        assert_eq!(balance, Some(150.0));
    }

    // API tests
    #[tokio::test]
    async fn test_store_api_token() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        
        let result = backend.store_api_token(currency_id, 1, "encrypted_token_value").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_api_token() {
        let backend = setup_test_backend().await;
        
        let currency_id = backend.create_currency(1, "Test", "TST").await.unwrap();
        
        backend.store_api_token(currency_id, 1, "my_secret_token").await.unwrap();
        
        let token = backend.get_api_token_by_type(currency_id, 1).await.unwrap();
        assert!(token.is_some());
        assert_eq!(token.unwrap(), "my_secret_token");
    }
}
