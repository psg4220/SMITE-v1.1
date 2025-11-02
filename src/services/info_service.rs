use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;

pub struct CurrencyInfo {
    pub name: String,
    pub ticker: String,
    pub total_in_circulation: f64,
    pub account_balance_total: f64,
    pub tax_balance_total: f64,
    pub swap_maker_total: f64,
    pub date_created: String,
}

pub async fn execute_info(
    ctx: &Context,
    msg: &Message,
    ticker: &str,
) -> Result<CurrencyInfo, String> {
    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    // Get currency by ticker
    let currency = db::currency::get_currency_by_ticker(&pool, ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("❌ Currency '{}' not found", ticker))?;

    let currency_id = currency.0;
    let currency_name = currency.1;
    let currency_ticker = currency.2;

    // Get total balance across all accounts
    let account_balance_total = db::account::get_total_balance(&pool, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .unwrap_or(0.0);

    // Get total tax balance
    let tax_balance_total = db::tax::get_total_tax_balance(&pool, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .unwrap_or(0.0);

    // Get total maker amounts in pending/open swaps
    let swap_maker_total = db::swap::get_total_swap_maker_amount(&pool, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .unwrap_or(0.0);

    // Calculate total in circulation
    let total_in_circulation = account_balance_total + tax_balance_total + swap_maker_total;

    // Get creation date
    let date_created = db::currency::get_currency_date(&pool, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(CurrencyInfo {
        name: currency_name,
        ticker: currency_ticker,
        total_in_circulation,
        account_balance_total,
        tax_balance_total,
        swap_maker_total,
        date_created,
    })
}

pub fn create_info_embed(info: &CurrencyInfo) -> serenity::builder::CreateEmbed {
    serenity::builder::CreateEmbed::default()
        .title(format!("📊 {} ({})", info.name, info.ticker))
        .field("Total in Circulation", format!("{:.2} {}", info.total_in_circulation, info.ticker), false)
        .field("Circulation Breakdown", 
            format!(
                "🏦 **User Accounts:** {:.2} {}\n💰 **Tax Reserves:** {:.2} {}\n💱 **Pending Swaps:** {:.2} {}",
                info.account_balance_total, info.ticker,
                info.tax_balance_total, info.ticker,
                info.swap_maker_total, info.ticker
            ),
            false)
        .field("Created", &info.date_created, false)
        .color(0x00ff00)
}
