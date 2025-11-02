use sqlx::mysql::MySqlPool;
use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;

/// Set tax percentage for a currency
pub async fn set_tax(
    pool: &MySqlPool,
    currency_id: i64,
    tax_percentage: i32,
    ticker: &str,
) -> Result<String, String> {
    // Validate percentage
    if tax_percentage < 0 || tax_percentage > 100 {
        return Err("❌ Tax percentage must be between 0 and 100".to_string());
    }

    // Check if tax account exists
    match db::tax::get_tax_account(pool, currency_id).await {
        Ok(Some(_)) => {
            // Update existing tax account
            db::tax::set_tax_percentage(pool, currency_id, tax_percentage)
                .await
                .map_err(|e| format!("Database error: {}", e))?;
            
            Ok(format!("✅ Tax set to {}% for {}", tax_percentage, ticker))
        },
        Ok(None) => {
            // Create new tax account
            db::tax::create_tax_account(pool, currency_id, tax_percentage)
                .await
                .map_err(|e| format!("Database error: {}", e))?;
            
            Ok(format!("✅ Tax account created with {}% tax for {}", tax_percentage, ticker))
        },
        Err(e) => {
            Err(format!("Database error: {}", e))
        }
    }
}

/// Collect tax from a currency's tax account
pub async fn collect_tax(
    pool: &MySqlPool,
    user_id: i64,
    currency_id: i64,
    amount: Option<String>,
) -> Result<String, String> {
    // Get tax account
    let tax_account = db::tax::get_tax_account(pool, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("❌ No tax account found for this currency")?;

    let current_balance = tax_account.2;

    if current_balance <= 0.0 {
        return Err("❌ No taxes to collect".to_string());
    }

    // Determine collection amount
    let collect_amount = if let Some(amt_str) = amount {
        if amt_str.to_lowercase() == "all" {
            current_balance
        } else {
            amt_str.parse::<f64>()
                .map_err(|_| "❌ Invalid amount".to_string())?
        }
    } else {
        current_balance
    };

    if collect_amount <= 0.0 {
        return Err("❌ Collection amount must be positive".to_string());
    }

    if collect_amount > current_balance {
        return Err(format!(
            "❌ Insufficient tax balance. Available: {:.8}",
            current_balance
        ))?;
    }

    // Collect tax
    let collected = db::tax::collect_tax(pool, currency_id, collect_amount)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // Add collected amount to user's account for this currency
    db::account::add_balance(pool, user_id, currency_id, collected)
        .await
        .map_err(|e| format!("Failed to add tax to account: {}", e))?;

    Ok(format!(
        "✅ Collected {:.8} tax and added to your account",
        collected
    ))
}

/// Get tax information for a currency
pub async fn get_tax_info(
    pool: &MySqlPool,
    currency_id: i64,
) -> Result<String, String> {
    match db::tax::get_tax_account(pool, currency_id).await {
        Ok(Some(tax_account)) => {
            let balance = tax_account.2;
            let percentage = tax_account.3;
            
            Ok(format!(
                "💰 **Tax Account Info**\n\
                 Percentage: **{}%**\n\
                 Balance: **{:.8}**",
                percentage, balance
            ))
        },
        Ok(None) => {
            Err("❌ No tax account set for this currency".to_string())
        },
        Err(e) => {
            Err(format!("Database error: {}", e))
        }
    }
}
