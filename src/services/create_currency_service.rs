use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;
use crate::services::permission_service;

pub struct CreateCurrencyResult {
    pub currency_id: i64,
    pub name: String,
    pub ticker: String,
}

pub async fn execute_create_currency(
    ctx: &Context,
    msg: &Message,
    name: &str,
    ticker: &str,
) -> Result<CreateCurrencyResult, String> {
    // Check permission (guild required, Admin role required)
    let perm_ctx = permission_service::check_permission(
        ctx,
        msg,
        &["Admin"],
    )
    .await?;

    let guild_id = perm_ctx.guild_id;

    // Validate ticker length (must be exactly 4 characters)
    if ticker.len() != 4 {
        return Err(format!(
            "Currency ticker must be exactly 4 characters long, but got '{}'",
            ticker
        ));
    }

    // Convert ticker to uppercase
    let ticker_upper = ticker.to_uppercase();

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
        currency_id,
        name: name.to_string(),
        ticker: ticker_upper,
    })
}

pub fn create_currency_embed(result: &CreateCurrencyResult) -> serenity::builder::CreateEmbed {
    serenity::builder::CreateEmbed::default()
        .title("💱 Currency Created")
        .field("Currency Name", &result.name, true)
        .field("Ticker", &result.ticker, true)
        .field("Currency ID", result.currency_id.to_string(), false)
        .description("Your guild's official currency has been created!")
        .color(0x00ff00)
}
