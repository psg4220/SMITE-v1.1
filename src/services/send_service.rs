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
    pub tax_amount: String,
}

pub async fn execute_send(
    ctx: &Context,
    msg: &Message,
    receiver_id: i64,
    amount: f64,
    currency_ticker: &str,
) -> Result<(i64, String, f64), String> {
    // Check permission (guild required, no special roles needed)
    let perm_ctx = permission_service::check_permission(
        ctx,
        msg,
        &[],
    )
    .await?;

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
    
    // Calculate tax
    let tax_percentage = db::tax::get_tax_percentage(&pool, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .unwrap_or(0);
    
    let tax_amount = if tax_percentage > 0 {
        (amount * tax_percentage as f64) / 100.0
    } else {
        0.0
    };
    
    let total_deduction = amount + tax_amount;
    
    if sender_balance < total_deduction {
        return Err(format!(
            "❌ Insufficient balance\n\nAmount: {:.2} {}\nTax: {:.2} {}\nTotal: {:.2} {}\nAvailable: {:.2} {}",
            amount, currency_ticker,
            tax_amount, currency_ticker,
            total_deduction, currency_ticker,
            sender_balance, currency_ticker
        ));
    }
    
    // Execute transfer - deduct both amount and tax from sender
    db::account::update_balance(&pool, sender_account_id, -total_deduction).await
        .map_err(|e| format!("Failed to update sender balance: {}", e))?;
    
    // Send only the amount (without tax) to receiver
    db::account::update_balance(&pool, receiver_account_id, amount).await
        .map_err(|e| format!("Failed to update receiver balance: {}", e))?;
    
    // Add tax to tax account if tax was deducted
    if tax_amount > 0.0 {
        db::tax::add_tax(&pool, currency_id, tax_amount)
            .await
            .map_err(|e| format!("Failed to record tax: {}", e))?;
    }
    
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
    
    Ok((receiver_id, transaction_uuid, tax_amount))
}

pub fn create_send_embed(result: &SendResult) -> serenity::builder::CreateEmbed {
    let mut recipients_str = String::new();
    for receiver_id in &result.receiver_ids {
        recipients_str.push_str(&format!("<@{}>\n", receiver_id));
    }
    
    let mut embed = serenity::builder::CreateEmbed::default()
        .title("💸 Transfer Successful")
        .field("From", format!("<@{}>", result.sender_id), false)
        .field("To", recipients_str, false)
        .color(0x00ff00);
    
    // Parse amounts to display breakdown
    if let (Ok(amount), Ok(tax)) = (result.amount.parse::<f64>(), result.tax_amount.parse::<f64>()) {
        let total_charged = amount + tax;
        
        if tax > 0.0 {
            let breakdown = format!(
                "**Amount Sent**: {} {}\n**Tax Deducted**: {} {}\n**Total Charged**: {:.2} {}",
                result.amount, result.currency_ticker,
                result.tax_amount, result.currency_ticker,
                total_charged, result.currency_ticker
            );
            embed = embed.field("Transfer Breakdown", breakdown, false);
        } else {
            embed = embed.field("Amount", format!("{} {}", result.amount, result.currency_ticker), false);
        }
    }
    
    embed
}
