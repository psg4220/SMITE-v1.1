use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::services::transaction_service;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        let help_embed = serenity::builder::CreateEmbed::default()
            .title("📋 Transaction Command")
            .description("View transaction details or history")
            .field("Usage",
                "`$transaction <uuid>` (view specific transaction)\n\
                 `$transaction list` (view all transactions)",
                false)
            .field("Examples",
                "`$transaction a1b2c3d4-e5f6-7890-abcd-ef1234567890`\n\
                 `$transaction list`",
                false)
            .field("Notes",
                "• Works in guilds and DMs\n\
                 • Transaction UUID is shown in transfer receipts\n\
                 • List shows your recent transactions",
                false)
            .color(0x00ff00);

        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    let user_id = msg.author.id.get() as i64;

    match args[0].to_lowercase().as_str() {
        "list" => {
            let result = transaction_service::get_transaction_list(&pool, user_id)
                .await?;

            if result.is_empty {
                let embed = serenity::builder::CreateEmbed::default()
                    .title("📋 Transaction History")
                    .description("No transactions found")
                    .color(0xffa500);

                msg.channel_id
                    .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }

            msg.channel_id
                .send_message(ctx, serenity::builder::CreateMessage::default().content(result.formatted_message))
                .await
                .map_err(|e| e.to_string())?;
        }
        _ => {
            // Treat first arg as UUID
            let uuid = args[0];

            let result = transaction_service::get_transaction_detail(&pool, uuid)
                .await?;

            let embed = serenity::builder::CreateEmbed::default()
                .title("📜 Transaction Receipt")
                .field("From", format!("<@{}>", result.sender_discord_id), true)
                .field("To", format!("<@{}>", result.receiver_discord_id), true)
                .field("Amount", format!("{:.2}", result.amount), true)
                .field("Date", result.date, false)
                .footer(serenity::builder::CreateEmbedFooter::new(format!("ID: {}", uuid)))
                .color(0x00ff00);

            msg.channel_id
                .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
