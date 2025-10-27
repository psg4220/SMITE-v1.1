use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::services::balance_service;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    let currency_ticker = if args.is_empty() {
        None
    } else if args[0] == "help" {
        let help_embed = serenity::builder::CreateEmbed::default()
            .title("💰 Balance Command")
            .description("Check your account balance")
            .field("Usage", "`$balance [currency ticker]` or `$bal [currency ticker]`", false)
            .field("Examples",
                "`$balance` (shows guild default - guild only)\n\
                 `$balance BTC` (shows Bitcoin balance - works in DM!)\n\
                 `$bal USD` (alias for balance)",
                false)
            .field("Notes",
                "• Use ticker to check balance in DMs (e.g., `$bal USD`)\n\
                 • Without ticker, only works in guilds (shows default)\n\
                 • Currency ticker is case-insensitive",
                false)
            .color(0x00ff00);

        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    } else {
        Some(args[0].to_uppercase())
    };

    match balance_service::get_balance(ctx, msg, currency_ticker.as_deref()).await {
        Ok(result) => {
            let embed = balance_service::create_balance_embed(&result);
            msg.channel_id
                .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                .await
                .map_err(|e| e.to_string())?;
        }
        Err(e) => {
            msg.reply(ctx, format!("❌ Failed to get balance: {}", e)).await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
