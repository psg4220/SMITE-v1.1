use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::services::create_currency_service;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    // Parse quoted currency name
    let (name, remaining_args) = match parse_quoted_arg(args) {
        Ok((n, r)) => (n, r),
        Err(e) if e == "__SHOW_HELP__" => {
            // No args - show help embed
            let help_embed = serenity::builder::CreateEmbed::default()
                .title("💱 Create Currency Command")
                .description("Create a new currency for your guild")
                .field("Usage", "`$cc \"<currency name>\" <ticker>`", false)
                .field("Examples", 
                    "`$cc \"Bitcoin\" BTC`\n\
                     `$cc \"US Dollar\" USDT`\n\
                     `$cc \"Central Reference Currency\" XCEN`", 
                    false)
                .field("Rules",
                    "• Currency name: Can have spaces (use quotes)\n\
                     • Ticker: 3-4 characters (auto-uppercase)\n\
                     • Per Guild: Only one currency per guild\n\
                     • Blacklist: Real-world currencies are reserved",
                    false)
                .color(0x00aaff);
            
            msg.channel_id
                .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
                .await
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
        Err(e) => {
            // Other errors
            let error_embed = serenity::builder::CreateEmbed::default()
                .title("❌ Error")
                .description(&e)
                .color(0xff0000);
            
            msg.channel_id
                .send_message(ctx, serenity::builder::CreateMessage::default().embed(error_embed))
                .await
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
    };
    
    if remaining_args.is_empty() {
        let error_embed = serenity::builder::CreateEmbed::default()
            .title("❌ Missing Ticker")
            .description("You need to provide a currency ticker!")
            .field("Usage", "`$cc \"<currency name>\" <ticker>`", false)
            .field("Example", "`$cc \"Bitcoin\" BTC`", false)
            .color(0xff0000);
        
        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(error_embed))
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    let ticker = remaining_args[0];

    match create_currency_service::execute_create_currency(ctx, msg, &name, ticker).await {
        Ok(result) => {
            let embed = create_currency_service::create_currency_embed(&result);
            msg.channel_id
                .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                .await
                .map_err(|e| e.to_string())?;
        }
        Err(e) => {
            let error_embed = serenity::builder::CreateEmbed::default()
                .title("❌ Failed to Create Currency")
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

/// Parse a quoted argument from the beginning of args
/// Returns (parsed_content, remaining_args)
/// Handles: "hello world" rest -> ("hello world", vec!["rest"])
/// Handles: hello rest -> ("hello", vec!["rest"])
/// Handles empty args -> error with special marker
fn parse_quoted_arg<'a>(args: &'a [&str]) -> Result<(String, Vec<&'a str>), String> {
    if args.is_empty() {
        return Err("__SHOW_HELP__".to_string()); // Special marker for help display
    }

    let first = args[0];
    
    // Check if it starts with a quote
    if first.starts_with('"') {
        // Find the closing quote
        let mut name_parts = vec![];
        let mut closing_quote_idx = None;
        
        for (i, arg) in args.iter().enumerate() {
            if i == 0 {
                // Remove opening quote from first part
                name_parts.push(arg.trim_start_matches('"'));
            } else {
                name_parts.push(arg);
            }
            
            // Check if this arg contains closing quote
            if arg.ends_with('"') {
                closing_quote_idx = Some(i);
                break;
            }
        }
        
        if let Some(idx) = closing_quote_idx {
            // Remove closing quote from last part
            if let Some(last) = name_parts.last_mut() {
                *last = last.trim_end_matches('"');
            }
            
            let name = name_parts.join(" ");
            let remaining: Vec<&str> = args[idx + 1..].to_vec();
            Ok((name, remaining))
        } else {
            Err("Unclosed quote in currency name".to_string())
        }
    } else {
        // No quotes, just take first arg
        let remaining: Vec<&str> = args[1..].to_vec();
        Ok((first.to_string(), remaining))
    }
}
