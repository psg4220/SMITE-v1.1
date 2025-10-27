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
        
        // Log specific error types for debugging
        if error_msg.contains("429") || error_msg.contains("rate limit") {
            eprintln!("   ⚠️  This appears to be a Discord rate limit issue");
        } else if error_msg.contains("HTTP request") {
            eprintln!("   ⚠️  Network/HTTP error - Could be Discord or connection issue");
        } else if error_msg.contains("Database") || error_msg.contains("database") {
            eprintln!("   ⚠️  Database error - Check database connection");
        } else if error_msg.contains("not found") || error_msg.contains("404") {
            eprintln!("   ⚠️  Resource not found - User/channel/message may have been deleted");
        } else if error_msg.contains("Permission") || error_msg.contains("permission") {
            eprintln!("   ⚠️  Permission denied - Bot may lack required Discord permissions");
        }
    }
}
