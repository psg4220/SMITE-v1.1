use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::services::wire_service;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    if args.is_empty() || args[0] == "help" {
        let help_embed = serenity::builder::CreateEmbed::default()
            .title("💳 Wire Command")
            .description("Bridge between SMITE economy and UnbelievaBoat balance")
            .field("Usage",
                "`$wire in <amount> <currency>` - Transfer from UnbelievaBoat to SMITE\n\
                 `$wire out <amount> <currency>` - Transfer from SMITE to UnbelievaBoat\n\
                 `$wire set token <token>` - Set UnbelievaBoat API token (admin only, guild only)",
                false)
            .field("Examples",
                "`$wire in 100 ABC` - Remove 100 ABC from UnbelievaBoat, add to SMITE account\n\
                 `$wire out 100 ABC` - Remove 100 ABC from SMITE account, add to UnbelievaBoat\n\
                 `$wire set token sk_live_xxx...` - Store encrypted token (admin only)",
                false)
            .field("Notes",
                "• `wire in/out` works in DMs or guilds\n\
                 • Cannot go negative on either side\n\
                 • Currency must exist in SMITE\n\
                 • Token management requires admin role (guild only)",
                false)
            .color(0x00b0f4);

        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Handle token setting (admin only, guild only)
    if args[0] == "set" {
        if args.len() < 3 || args[1] != "token" {
            return Err("❌ Usage: `$wire set token <token>`".to_string());
        }

        let guild_id = msg
            .guild_id
            .ok_or("Token management is guild-only. Use this command in a guild.".to_string())?;

        // Check if user is admin
        crate::utils::check_user_roles(ctx, guild_id, msg.author.id, &["admin"])
            .await?;

        let token = args[2..].join(" ");
        
        match wire_service::set_api_token(ctx, msg, &token).await {
            Ok(_) => {
                let embed = serenity::builder::CreateEmbed::default()
                    .title("✅ Token Set Successfully")
                    .description("UnbelievaBoat API token has been encrypted and stored.")
                    .color(0x00ff00);

                msg.channel_id
                    .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Err(e) => {
                let error_embed = e.to_embed();

                msg.channel_id
                    .send_message(ctx, serenity::builder::CreateMessage::default().embed(error_embed))
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        return Ok(());
    }

    // Handle wire in/out operations
    if args.len() < 3 {
        return Err("❌ Usage: `$wire in/out <amount> <currency>`".to_string());
    }

    let direction = args[0];
    let amount_str = args[1];
    let currency_ticker = args[2].to_uppercase();

    let amount: f64 = amount_str
        .parse()
        .map_err(|_| "❌ Invalid amount. Please provide a valid number.".to_string())?;

    if amount <= 0.0 {
        return Err("❌ Amount must be greater than 0.".to_string());
    }

    match direction {
        "in" => {
            match wire_service::wire_in(ctx, msg, amount, &currency_ticker).await {
                Ok(result) => {
                    let embed = serenity::builder::CreateEmbed::default()
                        .title("✅ Wire In Successful")
                        .description(format!(
                            "Transferred {} {} from UnbelievaBoat to SMITE",
                            amount, currency_ticker
                        ))
                        .field("UnbelievaBoat Balance", format!("{} bank remaining", result.ub_balance), false)
                        .field("SMITE Balance", format!("{} {}", result.smite_balance, currency_ticker), false)
                        .color(0x00ff00);

                    msg.channel_id
                        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Err(e) => {
                    let error_embed = e.to_embed();

                    msg.channel_id
                        .send_message(ctx, serenity::builder::CreateMessage::default().embed(error_embed))
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        "out" => {
            match wire_service::wire_out(ctx, msg, amount, &currency_ticker).await {
                Ok(result) => {
                    let embed = serenity::builder::CreateEmbed::default()
                        .title("✅ Wire Out Successful")
                        .description(format!(
                            "Transferred {} {} from SMITE to UnbelievaBoat",
                            amount, currency_ticker
                        ))
                        .field("SMITE Balance", format!("{} {} remaining", result.smite_balance, currency_ticker), false)
                        .field("UnbelievaBoat Balance", format!("{} bank", result.ub_balance), false)
                        .color(0x00ff00);

                    msg.channel_id
                        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Err(e) => {
                    let error_embed = e.to_embed();

                    msg.channel_id
                        .send_message(ctx, serenity::builder::CreateMessage::default().embed(error_embed))
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        _ => {
            return Err("❌ Direction must be `in` or `out`.".to_string());
        }
    }

    Ok(())
}
