use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;
use crate::services::permission_service;

pub struct SendResult {
    pub sender_id: i64,
    pub receiver_id: i64,
    pub amount: String,
    pub currency_ticker: String,
    pub transaction_uuid: String,
}

pub async fn execute_send(
    ctx: &Context,
    msg: &Message,
    receiver_id: i64,
    amount: f64,
    currency_ticker: &str,
) -> Result<SendResult, String> {
    // Check permission (guild required, no special roles needed)
    let perm_ctx = permission_service::check_permission(
        ctx,
        msg,
        &[],
    )
    .await?;

    let guild_id = perm_ctx.guild_id;

    let sender_id = msg.author.id.get() as i64;
    
    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };
    
    // Get guild's currency
    let guild_id = perm_ctx.guild_id;
    let currency_id = db::currency::get_currency_by_guild(&pool, guild_id as i64)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Guild has no currency set up".to_string())?
        .0;
    
    // Get sender and receiver account IDs
    let sender_account_id = db::account::get_account_id(&pool, sender_id, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Sender has no account".to_string())?;
    
    let receiver_account_id = db::account::get_account_id(&pool, receiver_id, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Receiver has no account".to_string())?;
    
    // Verify sender has sufficient balance
    let sender_balance = db::account::get_account_balance(&pool, sender_id, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Sender has no account".to_string())?;
    
    if sender_balance < amount {
        return Err("Insufficient balance".to_string());
    }
    
    // Execute transfer
    db::account::update_balance(&pool, sender_account_id, -amount).await
        .map_err(|e| format!("Failed to update sender balance: {}", e))?;
    db::account::update_balance(&pool, receiver_account_id, amount).await
        .map_err(|e| format!("Failed to update receiver balance: {}", e))?;
    
    // Log transaction
    let transaction_uuid = uuid::Uuid::new_v4().to_string();
    let _transaction = db::transaction::create_transaction(
        &pool,
        &transaction_uuid,
        sender_account_id,
        receiver_account_id,
        amount,
    ).await
    .map_err(|e| format!("Failed to log transaction: {}", e))?;
    
    Ok(SendResult {
        sender_id,
        receiver_id,
        amount: format!("{:.2}", amount),
        currency_ticker: currency_ticker.to_string(),
        transaction_uuid,
    })
}

pub fn create_send_embed(result: &SendResult) -> serenity::builder::CreateEmbed {
    serenity::builder::CreateEmbed::default()
        .title("💸 Transfer Successful")
        .field("From", format!("<@{}>", result.sender_id), false)
        .field("To", format!("<@{}>", result.receiver_id), false)
        .field("Amount", format!("{} {}", result.amount, result.currency_ticker), false)
        .footer(serenity::builder::CreateEmbedFooter::new(format!("Transaction ID: {}", result.transaction_uuid)))
        .color(0x00ff00)
}
