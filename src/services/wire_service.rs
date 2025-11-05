use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;
use crate::api::unbelievaboat::UnbelievaboatClient;
use crate::utils::{encrypt_token, decrypt_token};
use crate::utils::errors::WireError;

pub struct WireResult {
    pub smite_balance: f64,
    pub ub_balance: i64,
}

/// Set UnbelievaBoat API token for a currency (admin only)
pub async fn set_api_token(
    ctx: &Context,
    msg: &Message,
    token: &str,
) -> Result<(), WireError> {
    let guild_id = msg
        .guild_id
        .ok_or(WireError::InvalidConfig("Token management is guild-only.".to_string()))?
        .get() as i64;

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or(WireError::Database("Database not initialized".to_string()))?
            .clone()
    };

    // Get the guild's currency
    let currency_data = db::currency::get_currency_by_guild(&pool, guild_id)
        .await
        .map_err(|e| WireError::Database(format!("Database error: {}", e)))?
        .ok_or(WireError::InvalidConfig("No currency found for this guild".to_string()))?;

    let currency_id = currency_data.0;

    // Get encryption key from environment
    let encryption_key = std::env::var("TOKEN_ENCRYPTION_KEY")
        .map_err(|_| WireError::InvalidConfig("TOKEN_ENCRYPTION_KEY not set in environment".to_string()))?;

    // Encrypt the token (CryptoError is automatically converted via #[from])
    let encrypted_token = encrypt_token(token, &encryption_key)?;

    // Store encrypted token in database
    db::api::store_api_token(&pool, currency_id, 1, &encrypted_token)
        .await
        .map_err(|e| WireError::Database(format!("Failed to store token: {}", e)))?;

    Ok(())
}

/// Transfer from UnbelievaBoat to SMITE
/// Subtracts from UnbelievaBoat bank, adds to SMITE account
/// ATOMIC: All DB operations wrapped in a transaction; compensating transaction on API failure
pub async fn wire_in(
    ctx: &Context,
    msg: &Message,
    amount: f64,
    currency_ticker: &str,
) -> Result<WireResult, WireError> {
    let user_id = msg.author.id.get() as i64;
    
    // Get guild_id if available (DM-friendly)
    let guild_id = msg.guild_id.map(|id| id.get());

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or(WireError::Database("Database not initialized".to_string()))?
            .clone()
    };

    // Verify currency exists in SMITE
    let currency_data = db::currency::get_currency_by_ticker(&pool, currency_ticker)
        .await
        .map_err(|e| WireError::Database(format!("Database error: {}", e)))?
        .ok_or(WireError::InvalidConfig(format!("Currency {} not found in SMITE", currency_ticker)))?;

    let currency_id = currency_data.0;

    // Get UnbelievaBoat API token from database
    let encrypted_token = db::api::get_api_token(&pool, currency_id, 1)
        .await
        .map_err(|e| WireError::Database(format!("Database error: {}", e)))?
        .ok_or(WireError::InvalidConfig("UnbelievaBoat API token not configured for this currency".to_string()))?;

    // Decrypt the token (CryptoError is automatically converted via #[from])
    let encryption_key = std::env::var("TOKEN_ENCRYPTION_KEY")
        .map_err(|_| WireError::InvalidConfig("TOKEN_ENCRYPTION_KEY not set in environment".to_string()))?;
    let ub_token = decrypt_token(&encrypted_token, &encryption_key)?;

    // Initialize UnbelievaBoat client
    let ub_client = UnbelievaboatClient::new(ub_token);

    // Need guild_id for UnbelievaBoat API
    let guild_id = guild_id.ok_or(WireError::InvalidConfig(
        "Wire operations in DMs require the currency to be global. Please use this command in a guild.".to_string()
    ))?;

    // Get current UnbelievaBoat balance
    let ub_balance = ub_client
        .get_user_balance(guild_id, msg.author.id.get())
        .await
        .map_err(|e| WireError::Api(format!("Failed to fetch UnbelievaBoat balance: {}", e)))?;

    let ub_bank_amount = ub_balance.bank;

    // Check if user has enough in UnbelievaBoat
    if ub_bank_amount < amount as i64 {
        return Err(WireError::InsufficientBalance(format!(
            "Insufficient UnbelievaBoat balance. You have {} but need {}",
            ub_bank_amount, amount as i64
        )));
    }

    // START ATOMIC TRANSACTION: All DB operations in one transaction
    let mut tx = pool.begin().await
        .map_err(|e| WireError::Transaction(format!("Failed to start transaction: {}", e)))?;

    // Get or create SMITE account (within transaction)
    let account_id = match sqlx::query_scalar::<_, i64>(
        "SELECT id FROM account WHERE discord_id = ? AND currency_id = ?"
    )
    .bind(user_id)
    .bind(currency_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| WireError::Database(format!("Database error: {}", e)))?
    {
        Some(id) => id,
        None => {
            // Create new account
            let result = sqlx::query(
                "INSERT INTO account (discord_id, currency_id, balance) VALUES (?, ?, 0.0)"
            )
            .bind(user_id)
            .bind(currency_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| WireError::Database(format!("Failed to create account: {}", e)))?;
            
            result.last_insert_id() as i64
        }
    };

    // Get current SMITE balance (within transaction)
    let current_smite_balance: f64 = sqlx::query_scalar(
        "SELECT CAST(balance AS DOUBLE) as balance FROM account WHERE id = ?"
    )
    .bind(account_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| WireError::Database(format!("Failed to fetch balance: {}", e)))?;

    // Add to SMITE account (within transaction)
    let new_smite_balance = current_smite_balance + amount;
    sqlx::query(
        "UPDATE account SET balance = ? WHERE id = ?"
    )
    .bind(new_smite_balance)
    .bind(account_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| WireError::Database(format!("Failed to update balance: {}", e)))?;

    // COMMIT TRANSACTION before external API call
    tx.commit().await
        .map_err(|e| WireError::Transaction(format!("Failed to commit transaction: {}", e)))?;

    // NOW make external API call (outside transaction)
    let new_ub_bank = ub_bank_amount - amount as i64;
    match ub_client
        .set_user_balance(guild_id, msg.author.id.get(), None, Some(new_ub_bank))
        .await
    {
        Ok(_) => {
            // Success - both systems updated
            Ok(WireResult {
                smite_balance: new_smite_balance,
                ub_balance: new_ub_bank,
            })
        }
        Err(api_error) => {
            // API call failed - ROLLBACK the database change with compensating transaction
            let mut compensating_tx = pool.begin().await
                .map_err(|e| WireError::CompensationFailed(format!("Failed to start compensating transaction: {}", e)))?;
            
            // Restore original SMITE balance
            sqlx::query(
                "UPDATE account SET balance = ? WHERE id = ?"
            )
            .bind(current_smite_balance)
            .bind(account_id)
            .execute(&mut *compensating_tx)
            .await
            .map_err(|e| WireError::CompensationFailed(format!("Failed to compensate balance during rollback: {}", e)))?;
            
            compensating_tx.commit().await
                .map_err(|e| WireError::CompensationFailed(format!("Failed to commit compensating transaction: {}", e)))?;
            
            // Return error to user
            Err(WireError::Api(format!(
                "UnbelievaBoat API failed. Your balance has been restored. Please verify your token is correct. Error: {}",
                api_error
            )))
        }
    }
}

/// Transfer from SMITE to UnbelievaBoat
/// Subtracts from SMITE account, adds to UnbelievaBoat bank
/// ATOMIC: All DB operations wrapped in a transaction; compensating transaction on API failure
pub async fn wire_out(
    ctx: &Context,
    msg: &Message,
    amount: f64,
    currency_ticker: &str,
) -> Result<WireResult, WireError> {
    let user_id = msg.author.id.get() as i64;
    
    // Get guild_id if available (DM-friendly)
    let guild_id = msg.guild_id.map(|id| id.get());

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or(WireError::Database("Database not initialized".to_string()))?
            .clone()
    };

    // Verify currency exists in SMITE
    let currency_data = db::currency::get_currency_by_ticker(&pool, currency_ticker)
        .await
        .map_err(|e| WireError::Database(format!("Database error: {}", e)))?
        .ok_or(WireError::InvalidConfig(format!("Currency {} not found in SMITE", currency_ticker)))?;

    let currency_id = currency_data.0;

    // Get UnbelievaBoat API token from database
    let encrypted_token = db::api::get_api_token(&pool, currency_id, 1)
        .await
        .map_err(|e| WireError::Database(format!("Database error: {}", e)))?
        .ok_or(WireError::InvalidConfig("UnbelievaBoat API token not configured for this currency".to_string()))?;

    // Decrypt the token (CryptoError is automatically converted via #[from])
    let encryption_key = std::env::var("TOKEN_ENCRYPTION_KEY")
        .map_err(|_| WireError::InvalidConfig("TOKEN_ENCRYPTION_KEY not set in environment".to_string()))?;
    let ub_token = decrypt_token(&encrypted_token, &encryption_key)?;

    // Initialize UnbelievaBoat client
    let ub_client = UnbelievaboatClient::new(ub_token);

    // Need guild_id for UnbelievaBoat API
    let guild_id = guild_id.ok_or(WireError::InvalidConfig(
        "Wire operations in DMs require the currency to be global. Please use this command in a guild.".to_string()
    ))?;

    // START ATOMIC TRANSACTION: All DB operations in one transaction
    let mut tx = pool.begin().await
        .map_err(|e| WireError::Transaction(format!("Failed to start transaction: {}", e)))?;

    // Get current SMITE balance (within transaction)
    let current_smite_balance: f64 = sqlx::query_scalar(
        "SELECT CAST(balance AS DOUBLE) as balance FROM account WHERE discord_id = ? AND currency_id = ?"
    )
    .bind(user_id)
    .bind(currency_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| WireError::Database(format!("Database error: {}", e)))?
    .ok_or(WireError::InvalidConfig(format!("You don't have an account in {}", currency_ticker)))?;

    // Check if user has enough in SMITE
    if current_smite_balance < amount {
        // Rollback will happen automatically when tx is dropped
        return Err(WireError::InsufficientBalance(format!(
            "Insufficient SMITE balance. You have {} but need {}",
            current_smite_balance, amount
        )));
    }

    // Get account ID (within transaction)
    let account_id: i64 = sqlx::query_scalar(
        "SELECT id FROM account WHERE discord_id = ? AND currency_id = ?"
    )
    .bind(user_id)
    .bind(currency_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| WireError::Database(format!("Database error: {}", e)))?;

    // Subtract from SMITE account (within transaction)
    let new_smite_balance = current_smite_balance - amount;
    sqlx::query(
        "UPDATE account SET balance = ? WHERE id = ?"
    )
    .bind(new_smite_balance)
    .bind(account_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| WireError::Database(format!("Failed to update balance: {}", e)))?;

    // COMMIT TRANSACTION before external API call
    tx.commit().await
        .map_err(|e| WireError::Transaction(format!("Failed to commit transaction: {}", e)))?;

    // NOW make external API call (outside transaction)
    // Get current UnbelievaBoat balance
    let ub_balance = ub_client
        .get_user_balance(guild_id, msg.author.id.get())
        .await
        .map_err(|e| WireError::Api(format!("Failed to fetch UnbelievaBoat balance: {}", e)))?;

    let ub_bank_amount = ub_balance.bank;
    let new_ub_bank = ub_bank_amount + amount as i64;

    // Update UnbelievaBoat balance
    match ub_client
        .set_user_balance(guild_id, msg.author.id.get(), None, Some(new_ub_bank))
        .await
    {
        Ok(_) => {
            // Success - both systems updated
            Ok(WireResult {
                smite_balance: new_smite_balance,
                ub_balance: new_ub_bank,
            })
        }
        Err(api_error) => {
            // API call failed - ROLLBACK the database change with compensating transaction
            let mut compensating_tx = pool.begin().await
                .map_err(|e| WireError::CompensationFailed(format!("Failed to start compensating transaction: {}", e)))?;
            
            // Restore original SMITE balance
            sqlx::query(
                "UPDATE account SET balance = ? WHERE id = ?"
            )
            .bind(current_smite_balance)
            .bind(account_id)
            .execute(&mut *compensating_tx)
            .await
            .map_err(|e| WireError::CompensationFailed(format!("Failed to compensate balance during rollback: {}", e)))?;
            
            compensating_tx.commit().await
                .map_err(|e| WireError::CompensationFailed(format!("Failed to commit compensating transaction: {}", e)))?;
            
            // Return error to user
            Err(WireError::Api(format!(
                "UnbelievaBoat API failed. Your balance has been restored. Please verify your token is correct. Error: {}",
                api_error
            )))
        }
    }
}