pub mod ping;
pub mod send;
pub mod balance;
pub mod swap;
pub mod mint;
pub mod create_currency;
pub mod transaction;


use serenity::model::channel::Message;
use serenity::prelude::Context;

pub async fn handle_message(ctx: &Context, msg: &Message) {
    if msg.author.bot {
        return;
    }

    let content = &msg.content;
    
    // Parse command and arguments
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    let command = parts[0];
    let args = &parts[1..];

    let result = match command {
        "$ping" => ping::execute(ctx, msg).await,
        "$send" | "$transfer" => send::execute(ctx, msg, args).await,
        "$balance" | "$bal" => balance::execute(ctx, msg, args).await,
        "$swap" | "$trade" => swap::execute(ctx, msg, args).await,
        "$mint" | "$print" | "$issue" => mint::execute(ctx, msg, args).await,
        "$create_currency" | "$cc" => create_currency::execute(ctx, msg, args).await,
        "$transaction" | "$tr" => transaction::execute(ctx, msg, args).await,
        _ => return,
    };

    if let Err(e) = result {
        let error_msg = e.to_string();
        eprintln!("❌ Error executing command {}: {}", command, error_msg);
        
        // Extract clean error message from database errors
        // Pattern: "error returned from database: 1644 (45000): Insufficient balance to accept swap"
        let clean_error = if error_msg.contains("error returned from database:") {
            // Find the last colon, everything after it is the actual error message
            if let Some(last_colon) = error_msg.rfind(": ") {
                error_msg[last_colon + 2..].trim().to_string()
            } else {
                error_msg.clone()
            }
        } else {
            error_msg.clone()
        };
        
        // Determine error type and create user-friendly message
        let user_message = if error_msg.contains("429") || error_msg.contains("rate limit") {
            "⚠️ **Rate Limited**: Discord is rate limiting us. Please try again in a moment.".to_string()
        } else if error_msg.contains("HTTP request") {
            "⚠️ **Network Error**: Having trouble connecting to Discord. Please try again.".to_string()
        } else if clean_error.len() > 0 {
            format!("❌ {}", clean_error)
        } else {
            "❌ An error occurred while executing the command.".to_string()
        };

        // Send error to user as Discord message embed
        let embed = serenity::builder::CreateEmbed::default()
            .title("Command Error")
            .description(user_message)
            .color(0xff0000);

        let _ = msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
            .await;
    }
}
