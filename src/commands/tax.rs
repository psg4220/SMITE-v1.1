use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::services::{tax_service, permission_service};
use tracing::debug;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        let help_embed = serenity::builder::CreateEmbed::default()
            .title("💰 Tax Command")
            .description("Manage currency taxes and collect them")
            .field("Usage", 
                "`$tax set <currency_ticker> <percentage>` - Set tax % for a currency\n\
                 `$tax collect <currency_ticker> [amount|all]` - Collect taxes\n\
                 `$tax info <currency_ticker>` - View tax info",
                false)
            .field("Examples",
                "`$tax set ABC 20` - Set 20% tax on ABC\n\
                 `$tax collect ABC 100` - Collect 100 ABC tax\n\
                 `$tax collect ABC all` - Collect all ABC taxes\n\
                 `$tax info ABC` - View ABC tax status",
                false)
            .field("Permissions", "Only **admin** and **tax collector** roles can use this command", false)
            .color(0xffa500);

        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Check permissions - only admin and tax collector can use
    let perm_result = permission_service::check_permission(
        ctx,
        msg,
        &["admin", "tax collector"],
    ).await;

    if perm_result.is_err() {
        return Err("❌ You do not have permission to use tax commands. Required roles: **admin** or **tax collector**".to_string());
    }

    let subcommand = args[0].to_lowercase();

    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    match subcommand.as_str() {
        "set" => execute_set(ctx, msg, &pool, &args[1..]).await,
        "collect" => execute_collect(ctx, msg, &pool, &args[1..]).await,
        "info" => execute_info(ctx, msg, &pool, &args[1..]).await,
        _ => Err(format!("❌ Unknown subcommand: '{}'. Use: set, collect, or info", subcommand)),
    }
}

/// Set tax percentage for a currency
async fn execute_set(
    ctx: &Context,
    msg: &Message,
    pool: &sqlx::mysql::MySqlPool,
    args: &[&str],
) -> Result<(), String> {
    if args.len() < 2 {
        return Err("❌ Usage: `$tax set <currency_ticker> <percentage>`".to_string());
    }

    let ticker = args[0].to_uppercase();
    let percentage_str = args[1];

    let percentage: i32 = percentage_str
        .parse()
        .map_err(|_| "❌ Percentage must be a valid integer (0-100)".to_string())?;

    // Get currency by ticker with guild_id
    let currency = crate::db::currency::get_currency_by_ticker_with_guild(pool, &ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("❌ Currency '{}' not found", ticker))?;

    let currency_id = currency.0;
    let currency_guild_id = currency.1; // guild_id from currency tuple

    debug!("Tax command for currency: {} (ID: {}), Guild ID: {}", ticker, currency_id, currency_guild_id);

    // Get user's id
    let user_id = msg.author.id;
    let currency_guild_id_obj = serenity::model::prelude::GuildId::new(currency_guild_id as u64);
    
    // Get user's roles in the currency guild
    let user_roles = permission_service::get_user_role_names(ctx, currency_guild_id_obj, user_id)
        .await?;
    
    // Check if user has admin or tax collector role
    let has_required_role = user_roles.iter().any(|r| {
        r.to_lowercase() == "admin" || r.to_lowercase() == "tax collector"
    });
    
    if !has_required_role {
        return Err("❌ You do not have admin or tax collector role in the currency's guild".to_string());
    }

    // Set tax
    let response = tax_service::set_tax(pool, currency_id, percentage, &ticker).await?;

    let embed = serenity::builder::CreateEmbed::default()
        .title("💰 Tax Set")
        .description(response)
        .color(0x00ff00);

    msg.channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Collect taxes from a currency
async fn execute_collect(
    ctx: &Context,
    msg: &Message,
    pool: &sqlx::mysql::MySqlPool,
    args: &[&str],
) -> Result<(), String> {
    if args.is_empty() {
        return Err("❌ Usage: `$tax collect <currency_ticker> [amount|all]`".to_string());
    }

    let ticker = args[0].to_uppercase();
    let amount = args.get(1).copied();

    // Get currency by ticker with guild_id
    let currency = crate::db::currency::get_currency_by_ticker_with_guild(pool, &ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("❌ Currency '{}' not found", ticker))?;

    let currency_id = currency.0;
    let currency_guild_id = currency.1;

    // Check that the user has admin or tax collector role in the CURRENCY's guild
    let user_id = msg.author.id;
    let currency_guild_id_obj = serenity::model::prelude::GuildId::new(currency_guild_id as u64);
    
    // Get user's roles in the currency guild
    let user_roles = permission_service::get_user_role_names(ctx, currency_guild_id_obj, user_id)
        .await?;
    
    // Check if user has admin or tax collector role
    let has_required_role = user_roles.iter().any(|r| {
        r.to_lowercase() == "admin" || r.to_lowercase() == "tax collector"
    });
    
    if !has_required_role {
        return Err("❌ You do not have admin or tax collector role in the currency's guild".to_string());
    }

    let collector_id = msg.author.id.get() as i64;

    // Collect tax
    let response = tax_service::collect_tax(pool, collector_id, currency_id, amount.map(|s| s.to_string())).await?;

    let embed = serenity::builder::CreateEmbed::default()
        .title("💰 Tax Collected")
        .description(response)
        .color(0x00ff00);

    msg.channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// View tax information for a currency
async fn execute_info(
    ctx: &Context,
    msg: &Message,
    pool: &sqlx::mysql::MySqlPool,
    args: &[&str],
) -> Result<(), String> {
    if args.is_empty() {
        return Err("❌ Usage: `$tax info <currency_ticker>`".to_string());
    }

    let ticker = args[0].to_uppercase();

    // Get currency by ticker
    let currency = crate::db::currency::get_currency_by_ticker(pool, &ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("❌ Currency '{}' not found", ticker))?;

    let currency_id = currency.0;

    // Get tax info
    let response = tax_service::get_tax_info(pool, currency_id).await?;

    let embed = serenity::builder::CreateEmbed::default()
        .title(&ticker)
        .description(response)
        .color(0x00b0f4);

    msg.channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
