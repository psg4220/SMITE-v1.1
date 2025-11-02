use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::services::send_service;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    if args.len() < 2 {
        let help_embed = serenity::builder::CreateEmbed::default()
            .title("💸 Send Command")
            .description("Transfer currency to one or more users")
            .field("Usage", 
                "`$send <@user or id> <amount> <currency>`\n\
                 `$send (@user1 @user2 ... @userN) <amount> <currency>`",
                false)
            .field("Examples",
                "`$send @Alice 100 BTC` (send to one user)\n\
                 `$send (@Alice @Bob @Charlie) 50 USD` (send to multiple users)\n\
                 `$send (@Alice 123456789 @Bob) 75 ETH` (mixed IDs and mentions)",
                false)
            .field("Notes",
                "• Works in guilds and DMs\n\
                 • For multiple users, wrap them in parentheses: `(@user1 @user2 ...)`\n\
                 • Each user receives the same amount\n\
                 • Amount must be positive",
                false)
            .color(0x00ff00);

        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    let mut recipients = Vec::new();
    let mut amount_idx = 0;
    
    // Check if first argument starts with '(' for multiple recipients
    if args[0].starts_with('(') {
        // Multiple recipients mode: parse until we find ')'
        let mut found_end = false;
        for (idx, arg) in args.iter().enumerate() {
            let clean_arg = if idx == 0 {
                arg.trim_start_matches('(').trim()
            } else {
                arg.trim()
            };
            
            let clean_arg = if clean_arg.ends_with(')') {
                found_end = true;
                clean_arg.trim_end_matches(')').trim()
            } else {
                clean_arg
            };
            
            if !clean_arg.is_empty() {
                match parse_user_id(clean_arg) {
                    Ok(user_id) => recipients.push(user_id),
                    Err(_) => return Err(format!("❌ Invalid user ID or mention: {}", clean_arg)),
                }
            }
            
            if found_end {
                amount_idx = idx + 1;
                break;
            }
        }
        
        if !found_end {
            return Err("❌ Missing closing parenthesis for recipients list".to_string());
        }
    } else {
        // Single recipient mode
        match parse_user_id(args[0]) {
            Ok(user_id) => recipients.push(user_id),
            Err(e) => return Err(format!("❌ Invalid recipient: {}", e)),
        }
        amount_idx = 1;
    }
    
    // Validate we have at least one recipient
    if recipients.is_empty() {
        return Err("❌ Please specify at least one recipient".to_string());
    }
    
    // Parse amount and currency
    if amount_idx + 1 >= args.len() {
        return Err("❌ Please specify amount and currency".to_string());
    }
    
    let amount: f64 = args[amount_idx].parse()
        .map_err(|_| "❌ Invalid amount".to_string())?;
    
    if amount <= 0.0 {
        return Err("❌ Amount must be positive".to_string());
    }

    let currency_ticker = args[amount_idx + 1].to_uppercase();

    // Process each recipient and collect results
    let mut successful_recipients = Vec::new();
    let mut failed_recipients = Vec::new();
    let mut total_sent = 0.0;
    let mut total_tax = 0.0;
    
    for recipient_id in recipients {
        match send_service::execute_send(ctx, msg, recipient_id, amount, &currency_ticker).await {
            Ok((_receiver_id, _transaction_uuid, tax_amount)) => {
                successful_recipients.push(recipient_id);
                total_sent += amount;
                total_tax += tax_amount;
            }
            Err(e) => {
                failed_recipients.push((recipient_id, e));
            }
        }
    }
    
    // Send success embed if any transfers succeeded
    if !successful_recipients.is_empty() {
        let result = send_service::SendResult {
            sender_id: msg.author.id.get() as i64,
            receiver_ids: successful_recipients.clone(),
            amount: format!("{:.2}", amount),
            currency_ticker: currency_ticker.clone(),
            total_amount: format!("{:.2}", total_sent),
            tax_amount: format!("{:.8}", total_tax),
        };
        
        let embed = send_service::create_send_embed(&result);
        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
            .await
            .map_err(|e| e.to_string())?;
    }
    
    // If any transfers failed, report them in an embed
    if !failed_recipients.is_empty() {
        let title = if successful_recipients.is_empty() {
            "❌ All Transfers Failed"
        } else {
            "⚠️ Some Transfers Failed"
        };

        let description = if successful_recipients.is_empty() {
            "All transfers could not be completed".to_string()
        } else {
            format!("Completed {} of {} transfers:", successful_recipients.len(), successful_recipients.len() + failed_recipients.len())
        };

        let mut embed = serenity::builder::CreateEmbed::default()
            .title(title)
            .description(description)
            .color(0xff3333); // Red color for errors

        // Add error details for each failed recipient
        for (recipient_id, error) in &failed_recipients {
            embed = embed.field(
                format!("<@{}>", recipient_id),
                error,
                false
            );
        }

        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
            .await
            .map_err(|e| e.to_string())?;
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
