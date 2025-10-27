use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::services::send_service;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    if args.len() < 3 {
        let help_embed = serenity::builder::CreateEmbed::default()
            .title("💸 Send Command")
            .description("Transfer currency to another user")
            .field("Usage", "`$send <@user or id> <amount> <currency>`", false)
            .field("Examples",
                "`$send @Alice 100 BTC`\n\
                 `$send 123456789 50 USD`",
                false)
            .field("Notes",
                "• Works in guilds and DMs\n\
                 • Specify user by @mention or Discord ID\n\
                 • Amount must be positive",
                false)
            .color(0x00ff00);

        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Parse receiver ID from mention or raw ID
    let receiver_id = parse_user_id(args[0])?;
    
    // Parse amount
    let amount: f64 = args[1].parse()
        .map_err(|_| "Invalid amount".to_string())?;
    
    if amount <= 0.0 {
        msg.reply(ctx, "Amount must be positive").await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    let currency_ticker = args[2].to_uppercase();

    match send_service::execute_send(ctx, msg, receiver_id, amount, &currency_ticker).await {
        Ok(result) => {
            let embed = send_service::create_send_embed(&result);
            msg.channel_id
                .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                .await
                .map_err(|e| e.to_string())?;
        }
        Err(e) => {
            msg.reply(ctx, format!("❌ Transfer failed: {}", e)).await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

fn parse_user_id(input: &str) -> Result<i64, String> {
    // Remove mention formatting: <@123456789> -> 123456789
    let cleaned = input
        .trim_start_matches('<')
        .trim_start_matches('@')
        .trim_start_matches('!')
        .trim_end_matches('>');
    
    cleaned.parse::<i64>()
        .map_err(|_| "Invalid user ID or mention".to_string())
}
