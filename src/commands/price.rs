use serenity::all::{CreateEmbedFooter, EmbedFooter};
use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        let help_embed = serenity::builder::CreateEmbed::default()
            .title("💹 Price Command")
            .description("Display the last traded price for a currency pair")
            .field("Usage", "`$price <base>/<quote>`", false)
            .field("Examples",
                "`$price ABC/XYZ` (shows last trade price)\n\
                 `$price BTC/USD` (can also query in reverse order)",
                false)
            .field("Notes",
                "• The system stores pairs in canonical (alphabetical) order\n\
                 • Pairs are automatically normalized: ABC/XYZ or XYZ/ABC both work\n\
                 • Price = quote_amount / base_amount",
                false)
            .color(0x00ff00);

        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    let pair_str = args[0];
    let parts: Vec<&str> = pair_str.split('/').collect();
    
    if parts.len() != 2 {
        return Err("❌ Invalid pair format. Use: `$price BASE/QUOTE`".to_string());
    }

    let base_ticker = parts[0].trim().to_uppercase();
    let quote_ticker = parts[1].trim().to_uppercase();

    if base_ticker.is_empty() || quote_ticker.is_empty() {
        return Err("❌ Base and quote currencies cannot be empty".to_string());
    }

    if base_ticker == quote_ticker {
        return Err("❌ Base and quote currencies must be different".to_string());
    }

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    // Get guild ID, default to 0 for DMs
    let guild_id = msg.guild_id.map(|id| id.get() as i64).unwrap_or(0);

    // Get currency IDs by tickers
    let base_currency = db::currency::get_currency_by_ticker(&pool, &base_ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("❌ Currency '{}' not found", base_ticker))?;

    let quote_currency = db::currency::get_currency_by_ticker(&pool, &quote_ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("❌ Currency '{}' not found", quote_ticker))?;

    let base_currency_id = base_currency.0;
    let quote_currency_id = quote_currency.0;

    // Get the canonical order
    let (canonical_base_id, canonical_quote_id, is_reversed) = 
        db::tradelog::normalize_pair(&pool, base_currency_id, quote_currency_id)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

    // Get the latest price
    let price_result = db::tradelog::get_latest_price_for_pair(&pool, canonical_base_id, canonical_quote_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let (canonical_price, _) = price_result
        .ok_or("❌ No trading history found for this pair. Please execute a swap first.")?;

    // Calculate the price for the requested order
    let displayed_price = if is_reversed {
        1.0 / canonical_price
    } else {
        canonical_price
    };

    // Display the price
    let footer_text = format!("1 {} = {} {}", 
        base_ticker, 
        format!("{:.2}", if is_reversed { 1.0 / displayed_price } else { displayed_price }), 
        quote_ticker
    );

    let embed = serenity::builder::CreateEmbed::default()
        .title("💹 Trading Price")
        .field("Pair", format!("{}/{}", base_ticker, quote_ticker), false)
        .field("Price", format!("**{} {}**", format!("{:.2}", displayed_price), quote_ticker), false)
        .footer(CreateEmbedFooter::new(footer_text))
        .color(0x00ff00);

    msg.channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
