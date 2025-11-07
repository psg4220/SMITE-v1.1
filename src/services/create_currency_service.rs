use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;
use crate::blacklist;
use crate::models::CreateCurrencyResult;

pub async fn execute_create_currency(
    ctx: &Context,
    msg: &Message,
    name: &str,
    ticker: &str,
) -> Result<CreateCurrencyResult, String> {
    // Get guild ID (required)
    let guild_id = msg
        .guild_id
        .ok_or("This command can only be used in a guild".to_string())?;

    // Check permission - user must be admin
    crate::utils::check_user_roles(ctx, guild_id, msg.author.id, &["admin"])
        .await?;

    let guild_id = guild_id.get() as i64;

    // Validate ticker length (must be 3-4 characters)
    if ticker.len() < 3 || ticker.len() > 4 {
        return Err(format!(
            "Currency ticker must be 3-4 characters long, but got '{}' ({} chars)",
            ticker, ticker.len()
        ));
    }

    // Validate ticker characters (must be A-Z only)
    if ticker.chars().any(|c| !c.is_ascii_alphabetic()) {
        return Err(format!(
            "Currency ticker must only contain alphabetic characters (A-Z), but got '{}'",
            ticker
        ));
    }

    // Convert ticker to uppercase
    let ticker_upper = ticker.to_uppercase();

    // Check if ticker is blacklisted
    let blacklist_tickers = blacklist::get_blacklisted_tickers();
    if blacklist_tickers.contains(&ticker_upper) {
        return Err(format!(
            "❌ The ticker '{}' is reserved and cannot be used to prevent scams. Please choose a different ticker.",
            ticker_upper
        ));
    }

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    // Check if currency already exists in this guild
    match db::currency::get_currency_by_guild(&pool, guild_id as i64).await {
        Ok(Some(_)) => {
            return Err(
                "This guild already has a currency. Only one currency per guild is allowed."
                    .to_string(),
            );
        }
        Err(e) => {
            return Err(format!("Database error: {}", e));
        }
        Ok(None) => {}
    }

    // Create the currency
    let currency_id = db::currency::create_currency(&pool, guild_id as i64, name, &ticker_upper)
        .await
        .map_err(|e| format!("Failed to create currency: {}", e))?;

    Ok(CreateCurrencyResult {
        name: name.to_string(),
        ticker: ticker_upper,
    })
}

pub fn create_currency_embed(result: &CreateCurrencyResult) -> serenity::builder::CreateEmbed {
    serenity::builder::CreateEmbed::default()
        .title("💱 Currency Created")
        .field("Currency Name", &result.name, true)
        .field("Ticker", &result.ticker, true)
        .description("Your guild's official currency has been created!")
        .color(0x00ff00)
}
