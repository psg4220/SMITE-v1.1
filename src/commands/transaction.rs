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
            // Check if page number is specified (e.g., "p2" or "2")
            let mut page_num = 1;
            if args.len() > 1 {
                let page_arg = args[1].to_lowercase();
                let page_str = if page_arg.starts_with('p') {
                    &page_arg[1..]
                } else {
                    &page_arg
                };
                
                page_num = page_str.parse::<usize>()
                    .map_err(|_| "Invalid page number. Use: `$transaction list` or `$transaction list p2`".to_string())?;
            }

            // Fetch the requested page
            let (mut embeds, total_pages) = transaction_service::create_transaction_pages(&pool, user_id, page_num)
                .await?;

            if embeds.is_empty() {
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

            // Add page navigation info to footer if multiple pages exist
            if total_pages > 1 {
                embeds[0] = embeds[0].clone().footer(
                    serenity::builder::CreateEmbedFooter::new(
                        format!("Page {}/{}", page_num, total_pages)
                    )
                );
            }

            // Send the embed for the requested page
            msg.channel_id
                .send_message(ctx, serenity::builder::CreateMessage::default().embed(embeds[0].clone()))
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
