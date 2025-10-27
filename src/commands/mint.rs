use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::services::mint_service;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    if args.len() < 2 {
        let help_embed = serenity::builder::CreateEmbed::default()
            .title("💰 Mint Command")
            .description("Mint currency to your account (Admin/Minter only)")
            .field("Usage", "`$mint <amount> <currency ticker>`", false)
            .field("Examples",
                "`$mint 100 BTC`\n\
                 `$mint 50 USD`\n\
                 `$mint 1000 EUR`",
                false)
            .field("Requirements",
                "• Admin or Minter role required\n\
                 • Guild only (no DMs)\n\
                 • Amount must be positive\n\
                 • Account auto-created if needed",
                false)
            .color(0x9900ff);

        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
            .await
            .map_err(|e| e.to_string())?;   
        return Ok(());
    }

    // Parse amount
    let amount: f64 = args[0]
        .parse()
        .map_err(|_| "Invalid amount".to_string())?;

    let currency_ticker = args[1].to_uppercase();
    let user_id = msg.author.id.get() as i64;

    match mint_service::execute_mint(ctx, msg, user_id, amount, &currency_ticker).await {
        Ok(result) => {
            let embed = mint_service::create_mint_embed(&result);
            msg.channel_id
                .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                .await
                .map_err(|e| e.to_string())?;
        }
        Err(e) => {
            msg.reply(ctx, format!("❌ Mint failed: {}", e)).await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

fn parse_user_id(input: &str) -> Result<i64, String> {

    let cleaned = input
        .trim_start_matches('<')
        .trim_start_matches('@')
        .trim_start_matches('!')
        .trim_end_matches('>');

    cleaned
        .parse::<i64>()
        .map_err(|_| "Invalid user ID or mention".to_string())
}
