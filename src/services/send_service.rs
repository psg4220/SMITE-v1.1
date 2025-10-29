use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;
use crate::services::permission_service;

pub struct SendResult {
    pub sender_id: i64,
    pub receiver_ids: Vec<i64>,
    pub amount: String,
    pub currency_ticker: String,
    pub total_amount: String,
}

pub async fn execute_send(
    ctx: &Context,
    msg: &Message,
    receiver_id: i64,
    amount: f64,
    currency_ticker: &str,
) -> Result<(i64, String), String> {
    // Check permission (guild required, no special roles needed)
    let perm_ctx = permission_service::check_permission(
        ctx,
        msg,
        &[],
    )
    .await?;

    let guild_id = perm_ctx.guild_id;
    let sender_id = msg.author.id.get() as i64;

    // Prevent self transfer
    if sender_id == receiver_id {
        return Err("Cannot transfer to yourself".to_string());
    }
    
    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };
    
    // Get currency by ticker
    let (currency_id, currency_name, _) = db::currency::get_currency_by_ticker(&pool, currency_ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| format!("Currency '{}' not found", currency_ticker))?;
    
    // Get sender and receiver account IDs
    let sender_account_id = db::account::get_account_id(&pool, sender_id, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Sender has no account".to_string())?;
    
    // Get or create receiver account
    let receiver_account_id = match db::account::get_account_id(&pool, receiver_id, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
    {
        Some(account_id) => account_id,
        None => {
            // Create account for receiver
            db::account::create_account(&pool, receiver_id, currency_id)
                .await
                .map_err(|e| format!("Failed to create receiver account: {}", e))?
        }
    };
    
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
    
    Ok((receiver_id, transaction_uuid))
}

pub fn create_send_embed(result: &SendResult) -> serenity::builder::CreateEmbed {
    let mut recipients_str = String::new();
    for receiver_id in &result.receiver_ids {
        recipients_str.push_str(&format!("<@{}>\n", receiver_id));
    }
    
    serenity::builder::CreateEmbed::default()
        .title("💸 Transfer Successful")
        .field("From", format!("<@{}>", result.sender_id), false)
        .field("To", recipients_str, false)
        .field("Amount", format!("{} {}", result.amount, result.currency_ticker), false)
        .field("Total", format!("{} {}", result.total_amount, result.currency_ticker), false)
        .color(0x00ff00)
}
