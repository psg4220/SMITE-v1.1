use serenity::all::CreateEmbedFooter;
use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::services::{price_service, chart_service};
use std::fs;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    tracing::info!("💹 Price command called with args: {:?}", args);
    
    if args.is_empty() {
        let help_embed = serenity::builder::CreateEmbed::default()
            .title("💹 Price Command")
            .description("Display the VWAP and last traded price for a currency pair, or generate a price chart")
            .field("Usage", "`$price <base>/<quote> [timeframe]`\n`$price chart <base>/<quote>`", false)
            .field("Examples",
                "`$price ABC/XYZ` (default 24h VWAP)\n\
                 `$price BTC/USD 1h` (1 hour VWAP)\n\
                 `$price EUR/USD 7d` (7 day VWAP)\n\
                 `$price chart ABC/XYZ` (generate price chart)",
                false)
            .field("Timeframes",
                "**Minutes:** 1m, 2m, 4m, 5m, 10m, 20m, 30m\n\
                 **Hours:** 1h, 4h, 8h, 12h, 16h, 24h\n\
                 **Days:** 1d, 2d, 4d, 7d, 14d\n\
                 **Months:** 1mnt, 3mnt, 12mnt\n\
                 **Years:** 1y",
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

    // Check if the first argument is "chart"
    if args[0].to_lowercase() == "chart" {
        return execute_chart(ctx, msg, &args[1..]).await;
    }

    let pair_str = args[0];
    let parts: Vec<&str> = pair_str.split('/').collect();
    
    if parts.len() != 2 {
        return Err("❌ Invalid pair format. Use: `$price BASE/QUOTE`".to_string());
    }

    let base_ticker = parts[0].trim().to_uppercase();
    let quote_ticker = parts[1].trim().to_uppercase();

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    // Parse timeframe argument (default to 24h if not provided)
    let timeframe_arg = args.get(1).copied().unwrap_or("24h");

    // Call service to get price data
    let price_data = price_service::get_price(&pool, &base_ticker, &quote_ticker, timeframe_arg).await?;

    // Format footer text
    let footer_text = format!("1 {} = {:.8} {} (Timeframe: {})", 
        price_data.base_ticker,
        price_data.last_price, 
        price_data.quote_ticker,
        price_data.timeframe
    );

    // Build embed
    let mut embed = serenity::builder::CreateEmbed::default()
        .title("💹 Trading Price")
        .field("Pair", format!("{}/{}", price_data.base_ticker, price_data.quote_ticker), false)
        .field("Timeframe", format!("**{}**", price_data.timeframe), false);

    // Add VWAP field if available
    let vwap_label = format!("VWAP ({})", price_data.timeframe);
    if let Some(vwap) = price_data.vwap {
        embed = embed.field(&vwap_label, format!("**{:.8} {}**", vwap, price_data.quote_ticker), false);
    } else {
        embed = embed.field(&vwap_label, format!("No trades in {}", price_data.timeframe), false);
    }

    // Add Last Price field
    embed = embed
        .field("Last Price", format!("**{:.8} {}**", price_data.last_price, price_data.quote_ticker), false)
        .footer(CreateEmbedFooter::new(footer_text))
        .color(0x00ff00);

    msg.channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Generate and send a price chart for a currency pair
async fn execute_chart(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    tracing::info!("🎨 Chart command received from user {} with args: {:?}", msg.author.id, args);
    
    if args.is_empty() {
        tracing::warn!("No arguments provided for chart command");
        return Err("❌ Usage: `$price chart <base>/<quote> [timeframe]`\nTimeframes: 1d, 2d, 4d, 7d, 1w, 2w, 4w, 1M, 3M, 1y, all".to_string());
    }

    let pair_str = args[0];
    tracing::info!("Parsing pair: {}", pair_str);
    
    let parts: Vec<&str> = pair_str.split('/').collect();
    
    if parts.len() != 2 {
        tracing::warn!("Invalid pair format: {} (split into {} parts)", pair_str, parts.len());
        return Err("❌ Invalid pair format. Use: `$price chart BASE/QUOTE [timeframe]`".to_string());
    }

    let base_ticker = parts[0].trim().to_uppercase();
    let quote_ticker = parts[1].trim().to_uppercase();
    
    // Parse timeframe argument (default to "all" if not provided)
    let timeframe = args.get(1).copied().unwrap_or("all");
    tracing::info!("Parsed pair: {} / {} with timeframe: {}", base_ticker, quote_ticker, timeframe);

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };
    tracing::debug!("Got database pool");

    // Show typing indicator while generating chart
    match msg.channel_id.broadcast_typing(ctx.http.as_ref()).await {
        Ok(_) => tracing::debug!("Broadcast typing indicator"),
        Err(e) => {
            tracing::warn!("Failed to broadcast typing: {}", e);
            // Continue anyway
        }
    };

    // First verify currencies exist
    tracing::info!("Verifying currencies exist in database");
    
    let _base_currency = match crate::db::currency::get_currency_by_ticker(&pool, &base_ticker).await {
        Ok(Some(currency)) => {
            tracing::info!("✓ Found base currency: {} (ID: {})", base_ticker, currency.0);
            currency
        },
        Ok(None) => {
            let msg_text = format!("❌ Base currency '{}' not found in database", base_ticker);
            tracing::warn!("{}", msg_text);
            return Err(msg_text);
        },
        Err(e) => {
            let msg_text = format!("❌ Database error fetching {}: {}", base_ticker, e);
            tracing::error!("{}", msg_text);
            return Err(msg_text);
        }
    };

    let _quote_currency = match crate::db::currency::get_currency_by_ticker(&pool, &quote_ticker).await {
        Ok(Some(currency)) => {
            tracing::info!("✓ Found quote currency: {} (ID: {})", quote_ticker, currency.0);
            currency
        },
        Ok(None) => {
            let msg_text = format!("❌ Quote currency '{}' not found in database", quote_ticker);
            tracing::warn!("{}", msg_text);
            return Err(msg_text);
        },
        Err(e) => {
            let msg_text = format!("❌ Database error fetching {}: {}", quote_ticker, e);
            tracing::error!("{}", msg_text);
            return Err(msg_text);
        }
    };

    // Try to get price history to verify data exists
    tracing::info!("Fetching price history for {}/{}", base_ticker, quote_ticker);
    let _price_history = match chart_service::get_price_history(&pool, &base_ticker, &quote_ticker).await {
        Ok(history) => {
            let count = history.len();
            tracing::info!("✓ Found {} price points for {}/{}", count, base_ticker, quote_ticker);
            if count < 2 {
                let msg_text = format!("❌ Not enough price data for {}/{} ({} point(s) found, need at least 2)", 
                    base_ticker, quote_ticker, count);
                tracing::warn!("{}", msg_text);
                return Err(msg_text);
            }
            history
        },
        Err(e) => {
            tracing::error!("Price history error for {}/{}: {}", base_ticker, quote_ticker, e);
            return Err(e);
        }
    };

    // Generate the chart
    tracing::info!("Generating chart image for {}/{} with timeframe {}", base_ticker, quote_ticker, timeframe);
    let chart_data = match chart_service::generate_chart_with_timeframe(&pool, &base_ticker, &quote_ticker, timeframe, 1024, 768).await {
        Ok(data) => {
            if data.is_empty() {
                let msg_text = "❌ Chart generation failed: produced empty image data".to_string();
                tracing::error!("{}", msg_text);
                return Err(msg_text);
            }
            tracing::info!("✓ Chart generated successfully: {} bytes", data.len());
            data
        },
        Err(e) => {
            tracing::error!("Chart generation error: {}", e);
            return Err(e);
        }
    };

    // Create a unique temporary file for the chart with descriptive name
    let filename = format!("chart_{}_{}_{}_.png", base_ticker, quote_ticker, timeframe);
    let temp_path = format!("/tmp/{}", filename);
    tracing::debug!("Writing chart to: {}", temp_path);
    
    // Write chart data to temporary file
    match fs::write(&temp_path, &chart_data) {
        Ok(_) => tracing::debug!("✓ Chart file written successfully"),
        Err(e) => {
            let msg_text = format!("Failed to write chart file: {}", e);
            tracing::error!("{}", msg_text);
            return Err(msg_text);
        }
    };
    
    // Verify file was written
    let file_size = chart_data.len();
    if !std::path::Path::new(&temp_path).exists() {
        let msg_text = format!("❌ Chart file was not created at {}", temp_path);
        tracing::error!("{}", msg_text);
        return Err(msg_text);
    }
    tracing::debug!("✓ File exists: {} ({} bytes)", temp_path, file_size);
    
    // Create attachment from file path
    tracing::info!("Creating attachment from file: {}", temp_path);
    let attachment = match serenity::all::CreateAttachment::path(&temp_path).await {
        Ok(att) => {
            tracing::debug!("✓ Attachment created");
            att
        },
        Err(e) => {
            let msg_text = format!("Failed to create attachment: {}", e);
            tracing::error!("{}", msg_text);
            return Err(msg_text);
        }
    };
    
    // Create the message with just the file attachment (no embed)
    tracing::info!("Sending chart message to channel {}", msg.channel_id);
    let message = serenity::builder::CreateMessage::default()
        .add_file(attachment);
    
    match msg.channel_id.send_message(ctx, message).await {
        Ok(_) => {
            tracing::info!("✓✓✓ Chart message sent successfully for pair {}/{} ({} bytes)", base_ticker, quote_ticker, file_size);
        },
        Err(e) => {
            let msg_text = format!("Failed to send chart: {}", e);
            tracing::error!("{}", msg_text);
            return Err(msg_text);
        }
    };
    
    // Clean up temporary file after a brief delay to ensure Discord has received it
    let temp_path_clone = temp_path.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        match fs::remove_file(&temp_path_clone) {
            Ok(_) => tracing::debug!("✓ Temporary chart file deleted: {}", temp_path_clone),
            Err(e) => tracing::warn!("Failed to delete temporary chart file {}: {}", temp_path_clone, e),
        }
    });

    Ok(())
}