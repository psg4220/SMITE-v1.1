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
            .field("Usage", "`$price <base>/<quote> [timeframe]`\n`$price chart <base>/<quote>`\n`$price list [filters] [page]`", false)
            .field("Examples",
                "`$price ABC/XYZ` (default 24h VWAP)\n\
                 `$price BTC/USD 1h` (1 hour VWAP)\n\
                 `$price chart BTC/USD` (generate price chart)\n\
                 `$price list` (all prices)\n\
                 `$price list BTC/` (all BTC pairs)",
                false)
            .field("Timeframes",
                "**Minutes:** 1m, 2m, 4m, 5m, 10m, 20m, 30m\n\
                 **Hours:** 1h, 4h, 8h, 12h, 16h, 24h\n\
                 **Days:** 1d, 2d, 4d, 7d, 14d\n\
                 **Months:** 1mnt, 3mnt, 12mnt\n\
                 **Years:** 1y",
                false)
            .color(0x00ff00);

        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Route to subcommand
    match args[0].to_lowercase().as_str() {
        "chart" => execute_chart(ctx, msg, &args[1..]).await,
        "list" => execute_list(ctx, msg, &args[1..]).await,
        _ => execute_price(ctx, msg, args).await,
    }
}

/// Execute the price display command
async fn execute_price(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    tracing::info!("💹 Get price for pair: {:?}", args);
    
    if args.is_empty() {
        return Err("❌ Usage: `$price BASE/QUOTE [timeframe]`".to_string());
    }

    // Parse and validate pair
    let (base_ticker, quote_ticker) = price_service::parse_price_pair(args[0])?;

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    // Parse timeframe (default to 24h)
    let timeframe_arg = args.get(1).copied().unwrap_or("24h");

    // Call service to get price data
    let price_data = price_service::get_price(&pool, &base_ticker, &quote_ticker, timeframe_arg).await?;

    // Format footer text
    let footer_text = format!("1 {} = {:.2} {} (Timeframe: {})", 
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
        embed = embed.field(&vwap_label, format!("**{:.2} {}**", vwap, price_data.quote_ticker), false);
    } else {
        embed = embed.field(&vwap_label, format!("No trades in {}", price_data.timeframe), false);
    }

    // Add Last Price field
    embed = embed
        .field("Last Price", format!("**{:.2} {}**", price_data.last_price, price_data.quote_ticker), false)
        .footer(CreateEmbedFooter::new(footer_text))
        .color(0x00ff00);

    msg.channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Execute the price chart command
async fn execute_chart(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    tracing::info!("🎨 Chart command received from user {} with args: {:?}", msg.author.id, args);
    
    if args.is_empty() {
        return Err("❌ Usage: `$price chart <base>/<quote> [timeframe]`".to_string());
    }

    // Parse and validate pair
    let (base_ticker, quote_ticker) = price_service::parse_price_pair(args[0])?;

    // Parse timeframe (default to "all")
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
        }
    };

    // Verify currencies exist
    tracing::info!("Verifying currencies exist in database");
    
    let _base_currency = match crate::db::currency::get_currency_by_ticker(&pool, &base_ticker).await {
        Ok(Some(currency)) => {
            tracing::info!("✓ Found base currency: {} (ID: {})", base_ticker, currency.0);
            currency
        },
        Ok(None) => {
            return Err(format!("❌ Base currency '{}' not found in database", base_ticker));
        },
        Err(e) => {
            return Err(format!("❌ Database error fetching {}: {}", base_ticker, e));
        }
    };

    let _quote_currency = match crate::db::currency::get_currency_by_ticker(&pool, &quote_ticker).await {
        Ok(Some(currency)) => {
            tracing::info!("✓ Found quote currency: {} (ID: {})", quote_ticker, currency.0);
            currency
        },
        Ok(None) => {
            return Err(format!("❌ Quote currency '{}' not found in database", quote_ticker));
        },
        Err(e) => {
            return Err(format!("❌ Database error fetching {}: {}", quote_ticker, e));
        }
    };

    // Get price history to verify data exists
    tracing::info!("Fetching price history for {}/{}", base_ticker, quote_ticker);
    let _price_history = match chart_service::get_price_history(&pool, &base_ticker, &quote_ticker).await {
        Ok(history) => {
            let count = history.len();
            tracing::info!("✓ Found {} price points for {}/{}", count, base_ticker, quote_ticker);
            if count < 2 {
                return Err(format!("❌ Not enough price data for {}/{} ({} point(s) found, need at least 2)", 
                    base_ticker, quote_ticker, count));
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
                return Err("❌ Chart generation failed: produced empty image data".to_string());
            }
            tracing::info!("✓ Chart generated successfully: {} bytes", data.len());
            data
        },
        Err(e) => {
            tracing::error!("Chart generation error: {}", e);
            return Err(e);
        }
    };

    // Create temporary file for the chart
    let filename = format!("chart_{}_{}_{}_.png", base_ticker, quote_ticker, timeframe);
    let temp_path = format!("/tmp/{}", filename);
    tracing::debug!("Writing chart to: {}", temp_path);
    
    // Write chart data
    match fs::write(&temp_path, &chart_data) {
        Ok(_) => tracing::debug!("✓ Chart file written successfully"),
        Err(e) => {
            return Err(format!("Failed to write chart file: {}", e));
        }
    };
    
    // Verify file exists
    let file_size = chart_data.len();
    if !std::path::Path::new(&temp_path).exists() {
        return Err(format!("❌ Chart file was not created at {}", temp_path));
    }
    tracing::debug!("✓ File exists: {} ({} bytes)", temp_path, file_size);
    
    // Create attachment
    tracing::info!("Creating attachment from file: {}", temp_path);
    let attachment = match serenity::all::CreateAttachment::path(&temp_path).await {
        Ok(att) => {
            tracing::debug!("✓ Attachment created");
            att
        },
        Err(e) => {
            return Err(format!("Failed to create attachment: {}", e));
        }
    };
    
    // Send message with attachment
    tracing::info!("Sending chart message to channel {}", msg.channel_id);
    let message = serenity::builder::CreateMessage::default().add_file(attachment);
    
    match msg.channel_id.send_message(ctx, message).await {
        Ok(_) => {
            tracing::info!("✓✓✓ Chart message sent successfully for pair {}/{} ({} bytes)", base_ticker, quote_ticker, file_size);
        },
        Err(e) => {
            return Err(format!("Failed to send chart: {}", e));
        }
    };
    
    // Clean up temporary file
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

/// Execute the price list command
async fn execute_list(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    tracing::info!("💹 Price list command called with args: {:?}", args);
    
    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    // Parse arguments
    let (filter_base, filter_quote, page_num) = price_service::parse_price_list_args(args);

    // Get prices from service
    let prices = price_service::get_price_list(
        &pool,
        filter_base.as_deref(),
        filter_quote.as_deref(),
    )
    .await?;

    if prices.is_empty() {
        return Err("❌ No price data found".to_string());
    }

    // Format page and get description
    let items_per_page = 10;
    let (description, page_num, total_pages) = price_service::format_price_list_page(&prices, page_num, items_per_page)?;

    // Create embed
    let embed = serenity::builder::CreateEmbed::default()
        .title("💹 Price List")
        .description(description)
        .footer(CreateEmbedFooter::new(format!(
            "Page {}/{}",
            page_num, total_pages
        )))
        .color(0x00ff00);

    msg.channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}