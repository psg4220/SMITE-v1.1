use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::api::unbelievaboat::UnbelievaboatClient;
use crate::utils::{encrypt_token, decrypt_token};
use crate::utils::errors::WireError;
use crate::models::{WireDirection, WireResult};
use tracing;

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
    let currency_data = pool.get_currency_by_guild(guild_id)
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
    pool.store_api_token(currency_id, 1, &encrypted_token)
        .await
        .map_err(|e| WireError::Database(format!("Failed to store token: {}", e)))?;

    Ok(())
}

/// Core wire transfer function for both directions
/// NOTE: Wire transfers are currently only supported with MySQL backend due to transaction requirements
async fn execute_wire_transfer(
    _ctx: &Context,
    _msg: &Message,
    _direction: WireDirection,
    _amount: f64,
    _currency_ticker: &str,
) -> Result<WireResult, WireError> {
    // Wire transfers require database transactions which are not yet supported
    // in the database-agnostic trait. This feature is temporarily disabled.
    Err(WireError::InvalidConfig(
        "Wire transfers are temporarily disabled. This feature requires MySQL backend with transaction support.".to_string()
    ))
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