//! Wire Service - Core bridge logic for SMITE ↔ UnbelievaBoat transfers
//!
//! DISCLAIMER: This implementation does NOT violate UnbelievaBoat's Terms of Service.
//! This module uses the official UnbelievaBoat API with explicit authentication tokens.
//! Wire transfers are NOT automation - they result from intentional, manual user commands.
//! Every transfer requires explicit user invocation; there are no background processes
//! or scheduled tasks performing automated transactions. API calls are direct responses
//! to user-initiated commands, making this a legitimate integration, not a violation.

use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;
use crate::api::unbelievaboat::UnbelievaboatClient;
use crate::utils::{encrypt_token, decrypt_token};
use crate::utils::errors::WireError;
use tracing;

/// Direction of wire transfer
#[derive(Debug, Clone, Copy)]
pub enum WireDirection {
    /// Transfer from UnbelievaBoat to SMITE (add to SMITE)
    In,
    /// Transfer from SMITE to UnbelievaBoat (subtract from SMITE)
    Out,
}

pub struct WireResult {
    pub smite_balance: f64,
    pub ub_balance: i64,
}

/// Set UnbelievaBoat API token for a currency (admin only, DM-only for security)
/// User must have admin permissions in the target guild
pub async fn set_api_token(
    ctx: &Context,
    msg: &Message,
    guild_id_arg: Option<u64>,
    token: &str,
) -> Result<(), WireError> {
    // Determine guild ID - must be provided since command is DM-only
    let guild_id = guild_id_arg
        .ok_or(WireError::InvalidConfig(
            "Guild ID is required. Use: `$wire set token <guild_id> <token>`".to_string()
        ))? as i64;

    // Verify user has admin permissions in the target guild
    let target_guild_id = serenity::model::prelude::GuildId::new(guild_id as u64);
    crate::utils::check_user_roles(ctx, target_guild_id, msg.author.id, &["admin"])
        .await
        .map_err(|e| WireError::InvalidConfig(e))?;

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

/// Core wire transfer function for both directions
/// ATOMIC: All DB operations wrapped in a transaction; compensating transaction on API failure
async fn execute_wire_transfer(
    ctx: &Context,
    msg: &Message,
    direction: WireDirection,
    amount: f64,
    currency_ticker: &str,
) -> Result<WireResult, WireError> {
    let user_id = msg.author.id.get() as i64;

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or(WireError::Database("Database not initialized".to_string()))?
            .clone()
    };

    // Verify currency exists in SMITE
    let (currency_id, currency_guild_id, _, _) = db::currency::get_currency_by_ticker_with_guild(&pool, currency_ticker)
        .await
        .map_err(|e| WireError::Database(format!("Database error: {}", e)))?
        .ok_or(WireError::InvalidConfig(format!("Currency {} not found in SMITE", currency_ticker)))?;

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

    // Use currency's guild_id for UnbelievaBoat API
    // This ensures we're always talking to the correct UnbelievaBoat guild
    let guild_id = currency_guild_id as u64;

    // DIRECTION-SPECIFIC LOGIC: Check source balance and prepare for transfer
    match direction {
        WireDirection::In => {
            // wire_in: Check UnbelievaBoat balance (source of funds)
            crate::utils::rate_limit_ub_api().await;

            let ub_bank_amount = match ub_client
                .get_user_balance(guild_id, msg.author.id.get())
                .await
            {
                Ok(ub_balance) => ub_balance.bank,
                Err(crate::api::unbelievaboat::models::ApiError::NotFound(_)) => 0,
                Err(e) => return Err(WireError::Api(format!("Failed to fetch UnbelievaBoat balance: {}", e))),
            };

            if ub_bank_amount < amount as i64 {
                return Err(WireError::InsufficientBalance(format!(
                    "Insufficient UnbelievaBoat balance. You have {} but need {}",
                    ub_bank_amount, amount as i64
                )));
            }
        }
        WireDirection::Out => {
            // wire_out: Check SMITE balance (source of funds)
            let mut tx = pool.begin().await
                .map_err(|e| WireError::Transaction(format!("Failed to start transaction: {}", e)))?;

            let current_smite_balance: f64 = match sqlx::query_scalar(
                "SELECT CAST(balance AS DOUBLE) as balance FROM account WHERE discord_id = ? AND currency_id = ?"
            )
            .bind(user_id)
            .bind(currency_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| WireError::Database(format!("Database error: {}", e)))?
            {
                Some(balance) => balance,
                None => 0.0,
            };

            if current_smite_balance < amount {
                return Err(WireError::InsufficientBalance(format!(
                    "Insufficient SMITE balance. You have {} but need {}",
                    current_smite_balance, amount
                )));
            }

            // Rollback the balance check transaction
            tx.rollback().await
                .map_err(|e| WireError::Transaction(format!("Failed to rollback: {}", e)))?;
        }
    }

    // START ATOMIC TRANSACTION: All DB operations in one transaction
    let mut tx = pool.begin().await
        .map_err(|e| WireError::Transaction(format!("Failed to start transaction: {}", e)))?;

    // Get or create account (or fetch existing for wire_out)
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
            match direction {
                WireDirection::In => {
                    // wire_in: Create account if it doesn't exist
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
                WireDirection::Out => {
                    // wire_out: Account must exist
                    return Err(WireError::InsufficientBalance(format!(
                        "Insufficient SMITE balance. You have 0 but need {}",
                        amount
                    )));
                }
            }
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

    // DIRECTION-SPECIFIC: Update SMITE balance
    let new_smite_balance = match direction {
        WireDirection::In => current_smite_balance + amount,
        WireDirection::Out => current_smite_balance - amount,
    };

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

    // NOW make external API calls (outside transaction)
    crate::utils::rate_limit_ub_api().await;

    // DIRECTION-SPECIFIC: API calls and calculations
    match direction {
        WireDirection::In => {
            // wire_in: Subtract from UnbelievaBoat bank
            let ub_bank_amount = ub_client
                .get_user_balance(guild_id, msg.author.id.get())
                .await
                .map_err(|e| WireError::Api(format!("Failed to fetch UnbelievaBoat balance: {}", e)))?
                .bank;

            let new_ub_bank = ub_bank_amount - amount as i64;

            crate::utils::rate_limit_ub_api().await;

            match ub_client
                .set_user_balance(guild_id, msg.author.id.get(), None, Some(new_ub_bank))
                .await
            {
                Ok(_) => {
                    tracing::info!("wire_in SUCCESS: transferred {} {}", amount, currency_ticker);
                    Ok(WireResult {
                        smite_balance: new_smite_balance,
                        ub_balance: new_ub_bank,
                    })
                }
                Err(api_error) => {
                    compensate_smite_balance(&pool, account_id, current_smite_balance, api_error).await
                }
            }
        }
        WireDirection::Out => {
            // wire_out: Add to UnbelievaBoat bank
            let ub_balance = match ub_client
                .get_user_balance(guild_id, msg.author.id.get())
                .await
            {
                Ok(balance) => balance,
                Err(api_error) => {
                    return compensate_smite_balance(&pool, account_id, current_smite_balance, api_error).await;
                }
            };

            let ub_bank_amount = ub_balance.bank;
            let new_ub_bank = ub_bank_amount + amount as i64;

            crate::utils::rate_limit_ub_api().await;

            match ub_client
                .set_user_balance(guild_id, msg.author.id.get(), None, Some(new_ub_bank))
                .await
            {
                Ok(_) => {
                    tracing::info!("wire_out SUCCESS: transferred {} {}", amount, currency_ticker);
                    Ok(WireResult {
                        smite_balance: new_smite_balance,
                        ub_balance: new_ub_bank,
                    })
                }
                Err(api_error) => {
                    compensate_smite_balance(&pool, account_id, current_smite_balance, api_error).await
                }
            }
        }
    }
}

/// Transfer from UnbelievaBoat to SMITE
/// Subtracts from UnbelievaBoat bank, adds to SMITE account
pub async fn wire_in(
    ctx: &Context,
    msg: &Message,
    amount: f64,
    currency_ticker: &str,
) -> Result<WireResult, WireError> {
    execute_wire_transfer(ctx, msg, WireDirection::In, amount, currency_ticker).await
}

/// Transfer from SMITE to UnbelievaBoat
/// Subtracts from SMITE account, adds to UnbelievaBoat bank
pub async fn wire_out(
    ctx: &Context,
    msg: &Message,
    amount: f64,
    currency_ticker: &str,
) -> Result<WireResult, WireError> {
    execute_wire_transfer(ctx, msg, WireDirection::Out, amount, currency_ticker).await
}

/// Helper function to compensate SMITE balance on API failure
/// Used by both wire_in and wire_out to restore original balance when UnbelievaBoat API fails
async fn compensate_smite_balance(
    pool: &sqlx::MySqlPool,
    account_id: i64,
    original_balance: f64,
    api_error: crate::api::unbelievaboat::models::ApiError,
) -> Result<WireResult, WireError> {
    tracing::error!("API ERROR: {}, attempting compensation (account_id: {}, restore_balance: {})", api_error, account_id, original_balance);
    
    let mut compensating_tx = pool.begin().await
        .map_err(|e| {
            tracing::error!("Failed to start compensating transaction: {}", e);
            WireError::CompensationFailed(format!("Failed to start compensating transaction: {}", e))
        })?;
    
    // Restore original balance
    let rows_affected = sqlx::query(
        "UPDATE account SET balance = ? WHERE id = ?"
    )
    .bind(original_balance)
    .bind(account_id)
    .execute(&mut *compensating_tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute compensation UPDATE: {}", e);
        WireError::CompensationFailed(format!("Failed to compensate balance: {}", e))
    })?
    .rows_affected();

    if rows_affected == 0 {
        tracing::warn!("Compensation UPDATE found 0 rows (account_id: {})", account_id);
    }
    
    compensating_tx.commit().await
        .map_err(|e| {
            tracing::error!("Failed to commit compensating transaction: {}", e);
            WireError::CompensationFailed(format!("Failed to commit compensation: {}", e))
        })?;

    tracing::info!("Compensating transaction committed successfully (account_id: {}, restored_balance: {})", account_id, original_balance);
    
    Err(WireError::Api(format!(
        "UnbelievaBoat API failed. Your balance has been restored. Error: {}",
        api_error
    )))
}