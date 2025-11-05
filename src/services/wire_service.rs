use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;
use crate::api::unbelievaboat::UnbelievaboatClient;
use crate::utils::{encrypt_token, decrypt_token};
use crate::utils::encryption::CryptoError;

pub struct WireResult {
    pub smite_balance: f64,
    pub ub_balance: i64,
}

/// Set UnbelievaBoat API token for a currency (admin only)
pub async fn set_api_token(
    ctx: &Context,
    msg: &Message,
    token: &str,
) -> Result<(), String> {
    let guild_id = msg
        .guild_id
        .ok_or("Token management is guild-only.".to_string())?
        .get() as i64;

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    // Get the guild's currency
    let currency_data = db::currency::get_currency_by_guild(&pool, guild_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("No currency found for this guild".to_string())?;

    let currency_id = currency_data.0;

    // Get encryption key from environment
    let encryption_key = std::env::var("TOKEN_ENCRYPTION_KEY")
        .map_err(|_| "TOKEN_ENCRYPTION_KEY not set in environment".to_string())?;

    // Encrypt the token
    let encrypted_token = encrypt_token(token, &encryption_key)
        .map_err(|e| format!("Encryption error: {}", e))?;

    // Store encrypted token in database
    db::api::store_api_token(&pool, currency_id, 1, &encrypted_token)
        .await
        .map_err(|e| format!("Failed to store token: {}", e))?;

    Ok(())
}

/// Transfer from UnbelievaBoat to SMITE
/// Subtracts from UnbelievaBoat bank, adds to SMITE account
pub async fn wire_in(
    ctx: &Context,
    msg: &Message,
    amount: f64,
    currency_ticker: &str,
) -> Result<WireResult, String> {
    let user_id = msg.author.id.get() as i64;
    
    // Get guild_id if available (DM-friendly)
    let guild_id = msg.guild_id.map(|id| id.get());

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    // Verify currency exists in SMITE
    let currency_data = db::currency::get_currency_by_ticker(&pool, currency_ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("Currency {} not found in SMITE", currency_ticker))?;

    let currency_id = currency_data.0;

    // Get UnbelievaBoat API token from database
    let encrypted_token = db::api::get_api_token(&pool, currency_id, 1)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("UnbelievaBoat API token not configured for this currency".to_string())?;

    // Decrypt the token
    let encryption_key = std::env::var("TOKEN_ENCRYPTION_KEY")
        .map_err(|_| "TOKEN_ENCRYPTION_KEY not set in environment".to_string())?;
    let ub_token = decrypt_token(&encrypted_token, &encryption_key)
        .map_err(|e| format!("Decryption error: {}", e))?;

    // Initialize UnbelievaBoat client
    let ub_client = UnbelievaboatClient::new(ub_token);

    // Need guild_id for UnbelievaBoat API
    let guild_id = guild_id.ok_or(
        "Wire operations in DMs require the currency to be global. Please use this command in a guild.".to_string()
    )?;

    // Get current UnbelievaBoat balance
    let ub_balance = ub_client
        .get_user_balance(guild_id, msg.author.id.get())
        .await
        .map_err(|e| format!("Failed to fetch UnbelievaBoat balance: {}", e))?;

    let ub_bank_amount = ub_balance.bank;

    // Check if user has enough in UnbelievaBoat
    if ub_bank_amount < amount as i64 {
        return Err(format!(
            "Insufficient UnbelievaBoat balance. You have {} but need {}",
            ub_bank_amount, amount as i64
        ));
    }

    // Get or create SMITE account
    let account_id = match db::account::get_account_id(&pool, user_id, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
    {
        Some(id) => id,
        None => {
            db::account::create_account(&pool, user_id, currency_id)
                .await
                .map_err(|e| format!("Database error: {}", e))?
        }
    };

    // Get current SMITE balance
    let current_smite_balance = db::account::get_account_balance(&pool, user_id, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .unwrap_or(0.0);

    // Subtract from UnbelievaBoat
    let new_ub_bank = ub_bank_amount - amount as i64;
    ub_client
        .set_user_balance(guild_id, msg.author.id.get(), None, Some(new_ub_bank))
        .await
        .map_err(|e| format!("Failed to update UnbelievaBoat balance: {}", e))?;

    // Add to SMITE
    let new_smite_balance = current_smite_balance + amount;
    db::account::update_balance(&pool, account_id, amount)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(WireResult {
        smite_balance: new_smite_balance,
        ub_balance: new_ub_bank,
    })
}

/// Transfer from SMITE to UnbelievaBoat
/// Subtracts from SMITE account, adds to UnbelievaBoat bank
pub async fn wire_out(
    ctx: &Context,
    msg: &Message,
    amount: f64,
    currency_ticker: &str,
) -> Result<WireResult, String> {
    let user_id = msg.author.id.get() as i64;
    
    // Get guild_id if available (DM-friendly)
    let guild_id = msg.guild_id.map(|id| id.get());

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    // Verify currency exists in SMITE
    let currency_data = db::currency::get_currency_by_ticker(&pool, currency_ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("Currency {} not found in SMITE", currency_ticker))?;

    let currency_id = currency_data.0;

    // Get UnbelievaBoat API token from database
    let encrypted_token = db::api::get_api_token(&pool, currency_id, 1)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("UnbelievaBoat API token not configured for this currency".to_string())?;

    // Decrypt the token
    let encryption_key = std::env::var("TOKEN_ENCRYPTION_KEY")
        .map_err(|_| "TOKEN_ENCRYPTION_KEY not set in environment".to_string())?;
    let ub_token = decrypt_token(&encrypted_token, &encryption_key)
        .map_err(|e| format!("Decryption error: {}", e))?;

    // Initialize UnbelievaBoat client
    let ub_client = UnbelievaboatClient::new(ub_token);

    // Need guild_id for UnbelievaBoat API
    let guild_id = guild_id.ok_or(
        "Wire operations in DMs require the currency to be global. Please use this command in a guild.".to_string()
    )?;

    // Get current SMITE balance
    let current_smite_balance = db::account::get_account_balance(&pool, user_id, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("You don't have an account in {}", currency_ticker))?;

    // Check if user has enough in SMITE
    if current_smite_balance < amount {
        return Err(format!(
            "Insufficient SMITE balance. You have {} but need {}",
            current_smite_balance, amount
        ));
    }

    // Get current UnbelievaBoat balance
    let ub_balance = ub_client
        .get_user_balance(guild_id, msg.author.id.get())
        .await
        .map_err(|e| format!("Failed to fetch UnbelievaBoat balance: {}", e))?;

    let ub_bank_amount = ub_balance.bank;
    let new_ub_bank = ub_bank_amount + amount as i64;

    // Get account ID
    let account_id = db::account::get_account_id(&pool, user_id, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Account not found".to_string())?;

    // Subtract from SMITE
    let new_smite_balance = current_smite_balance - amount;
    db::account::update_balance(&pool, account_id, -amount)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // Add to UnbelievaBoat
    ub_client
        .set_user_balance(guild_id, msg.author.id.get(), None, Some(new_ub_bank))
        .await
        .map_err(|e| format!("Failed to update UnbelievaBoat balance: {}", e))?;

    Ok(WireResult {
        smite_balance: new_smite_balance,
        ub_balance: new_ub_bank,
    })
}
