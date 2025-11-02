use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::services::info_service;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        let help_embed = serenity::builder::CreateEmbed::default()
            .title("📊 Info Command")
            .description("Get detailed information about a currency")
            .field("Usage", "`$info <ticker>`", false)
            .field("Examples", 
                "`$info BTC`\n\
                 `$info USD`\n\
                 `$info XCEN`",
                false)
            .field("Information Displayed",
                "• Currency Name\n\
                 • Ticker Symbol\n\
                 • Total in Circulation\n\
                 • Creation Date\n\
                 • Circulation Breakdown",
                false)
            .color(0x00aaff);

        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    let ticker = args[0].to_uppercase();

    match info_service::execute_info(ctx, msg, &ticker).await {
        Ok(result) => {
            let embed = info_service::create_info_embed(&result);
            msg.channel_id
                .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                .await
                .map_err(|e| e.to_string())?;
        }
        Err(e) => {
            let error_embed = serenity::builder::CreateEmbed::default()
                .title("❌ Error")
                .description(&e)
                .color(0xff0000);
            
            msg.channel_id
                .send_message(ctx, serenity::builder::CreateMessage::default().embed(error_embed))
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
