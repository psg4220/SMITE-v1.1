use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;

pub struct BalanceResult {
    pub user_id: i64,
    pub balance: String,
    pub currency_ticker: String,
}

pub async fn get_balance(
    ctx: &Context,
    msg: &Message,
    currency_ticker: Option<&str>,
) -> Result<BalanceResult, String> {
    let user_id = msg.author.id.get() as i64;
    
    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };
    
    let (currency_id, ticker) = if let Some(ticker) = currency_ticker {
        // Look up currency by ticker (searches across all guilds)
        let currency_data = db::currency::get_currency_by_ticker(&pool, ticker)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or(format!("Currency {} not found", ticker))?;
        (currency_data.0, currency_data.2)
    } else {
        // No ticker specified - requires guild context to get default currency
        let guild_id = msg
            .guild_id
            .ok_or("❌ Please specify a currency ticker (e.g., `$bal USD`). Default balance is only available in guilds.".to_string())?
            .get();
        
        // Get guild's default currency
        let currency_data = db::currency::get_currency_by_guild(&pool, guild_id as i64)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or("Guild has no currency set up".to_string())?;
        (currency_data.0, currency_data.2)
    };
    
    // Get balance
    let balance = db::account::get_account_balance(&pool, user_id, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("User has no account for this currency".to_string())?;
    
    Ok(BalanceResult {
        user_id,
        balance: format!("{:.2}", balance),
        currency_ticker: ticker,
    })
}

pub fn create_balance_embed(result: &BalanceResult) -> serenity::builder::CreateEmbed {
    serenity::builder::CreateEmbed::default()
        .title("💰 Balance")
        .field("User", format!("<@{}>", result.user_id), false)
        .field("Balance", format!("{} {}", result.balance, result.currency_ticker), false)
        .color(0x00b0f4)
}
