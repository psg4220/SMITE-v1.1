use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::services::swap_service;
use crate::utils::extract_clean_error;

pub async fn execute(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        let help_embed = serenity::builder::CreateEmbed::default()
            .title("🔄 Swap Command")
            .description("Trade currencies with other users")
            .field("Usage",
                "`$swap <amount> <currency> [<@user or id> <amount> <currency>]`\n\
                 `$swap accept [swap_id]`\n\
                 `$swap deny [swap_id]`\n\
                 `$swap status <swap_id>`",
                false)
            .field("Examples",
                "**Create targeted swap:**\n\
                 `$swap 100 BTC @Alice 50 USD`\n\n\
                 **Create open swap:**\n\
                 `$swap 100 EUR`\n\n\
                 **Accept/Deny:**\n\
                 `$swap accept 123` (accept swap ID 123)\n\
                 `$swap deny 123` (deny swap ID 123)\n\n\
                 **Check status:**\n\
                 `$swap status 123` (view swap info)",
                false)
            .field("Notes",
                "• Guild only (no DMs)\n\
                 • Specify amounts and currencies\n\
                 • Amounts must be positive\n\
                 • Your Discord ID is used as the maker\n\
                 • Use `$swap status` to check swap details",
                false)
            .color(0xffa500);

        msg.channel_id
            .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    match args[0] {
        "status" => {
            let swap_id = if args.len() > 1 {
                args[1].parse::<i64>()
                    .map_err(|_| "Invalid swap ID".to_string())?
            } else {
                return Err("Please specify a swap ID: `$swap status <id>`".to_string());
            };
            
            match swap_service::get_swap_status(ctx, msg, swap_id).await {
                Ok(embed) => {
                    msg.channel_id
                        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Err(e) => {
                    let clean_error = extract_clean_error(&e);
                    let error_embed = serenity::builder::CreateEmbed::default()
                        .title("Error")
                        .description(format!("❌ {}", clean_error))
                        .color(0xff0000);
                    msg.channel_id
                        .send_message(ctx, serenity::builder::CreateMessage::default().embed(error_embed))
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        "accept" => {
            let swap_id = if args.len() > 1 {
                Some(args[1].parse::<i64>()
                    .map_err(|_| "Invalid swap ID".to_string())?)
            } else {
                None
            };
            
            match swap_service::accept_swap(ctx, msg, swap_id).await {
                Ok((result, _original_msg_id)) => {
                    let embed = swap_service::create_accept_deny_embed(&result);
                    msg.channel_id
                        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Err(e) => {
                    let clean_error = extract_clean_error(&e);
                    let error_embed = serenity::builder::CreateEmbed::default()
                        .title("Error")
                        .description(format!("❌ {}", clean_error))
                        .color(0xff0000);
                    msg.channel_id
                        .send_message(ctx, serenity::builder::CreateMessage::default().embed(error_embed))
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        "deny" => {
            let swap_id = if args.len() > 1 {
                Some(args[1].parse::<i64>()
                    .map_err(|_| "Invalid swap ID".to_string())?)
            } else {
                None
            };
            
            match swap_service::deny_swap(ctx, msg, swap_id).await {
                Ok((result, _original_msg_id)) => {
                    let embed = swap_service::create_accept_deny_embed(&result);
                    msg.channel_id
                        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Err(e) => {
                    let clean_error = extract_clean_error(&e);
                    let error_embed = serenity::builder::CreateEmbed::default()
                        .title("Error")
                        .description(format!("❌ {}", clean_error))
                        .color(0xff0000);
                    msg.channel_id
                        .send_message(ctx, serenity::builder::CreateMessage::default().embed(error_embed))
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        _ => {
            // Handle swap creation
            if args.len() < 2 {
                let help_embed = serenity::builder::CreateEmbed::default()
                    .title("🔄 Swap Command - Create")
                    .description("Create a new swap/trade")
                    .field("Usage", "`$swap <amount> <currency> [<@user or id> <amount> <currency>]`\nOR\n`$swap <currency> <amount> [<@user or id> <currency> <amount>]`", false)
                    .field("Examples",
                        "**Targeted swap (2-way trade):**\n\
                         `$swap 100 BTC @Alice 50 USD` or `$swap BTC 100 @Alice USD 50`\n\n\
                         **Open swap (anyone can accept):**\n\
                         `$swap 100 EUR` or `$swap EUR 100`",
                        false)
                    .color(0xffa500);

                msg.channel_id
                    .send_message(ctx, serenity::builder::CreateMessage::default().embed(help_embed))
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }

            // Determine format: amount-currency or currency-amount
            // Try to parse args[0] as number first
            let (maker_amount, maker_ticker) = if let Ok(amount) = args[0].parse::<f64>() {
                // Format: amount currency ...
                (amount, args[1].to_uppercase())
            } else {
                // Format: currency amount ...
                let amount = args[1].parse::<f64>()
                    .map_err(|_| "Invalid amount - expected number".to_string())?;
                (amount, args[0].to_uppercase())
            };

            if maker_amount <= 0.0 {
                msg.reply(ctx, "Amount must be positive").await
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }

            // Parse maker (use message author)
            let maker_id = msg.author.id.get() as i64;

            // Check if this is an open trade or targeted trade
            let (taker_id, taker_amount, taker_ticker) = if args.len() >= 4 {
                // Check if args[2] is a user mention/ID or a currency/amount
                let parse_result = parse_user_id(args[2]);
                
                if parse_result.is_ok() {
                    // It's a user ID - this is a TARGETED swap
                    // Format: $swap 10 ABC @user 10 XYZ OR $swap ABC 10 @user XYZ 10
                    let taker_id = parse_result.unwrap();
                    
                    // Determine format for taker: amount-currency or currency-amount
                    let (taker_amount, taker_ticker) = if let Ok(amount) = args[3].parse::<f64>() {
                        // Format: @user amount currency
                        (amount, if args.len() > 4 { args[4].to_uppercase() } else { return Err("Taker currency required".to_string()); })
                    } else {
                        // Format: @user currency amount
                        let amount = if args.len() > 4 {
                            args[4].parse::<f64>()
                                .map_err(|_| "Invalid taker amount".to_string())?
                        } else {
                            return Err("Taker amount required".to_string());
                        };
                        (amount, args[3].to_uppercase())
                    };

                    if taker_amount <= 0.0 {
                        msg.reply(ctx, "Amount must be positive").await
                            .map_err(|e| e.to_string())?;
                        return Ok(());
                    }

                    (Some(taker_id), Some(taker_amount), Some(taker_ticker))
                } else {
                    // It's not a user ID, treat as OPEN swap with 2 currencies
                    // Format: $swap 10 ABC 10 XYZ OR $swap ABC 10 XYZ 10
                    let (taker_amount, taker_ticker) = if let Ok(amount) = args[2].parse::<f64>() {
                        // Format: amount currency
                        (amount, args[3].to_uppercase())
                    } else {
                        // Format: currency amount
                        let amount = args[3].parse::<f64>()
                            .map_err(|_| "Invalid taker amount".to_string())?;
                        (amount, args[2].to_uppercase())
                    };

                    if taker_amount <= 0.0 {
                        msg.reply(ctx, "Amount must be positive").await
                            .map_err(|e| e.to_string())?;
                        return Ok(());
                    }

                    // For open swaps, taker_id is None
                    (None, Some(taker_amount), Some(taker_ticker))
                }
            } else {
                (None, None, None)
            };

            match swap_service::execute_swap(
                ctx,
                msg,
                maker_id,
                maker_amount,
                &maker_ticker,
                taker_id,
                taker_amount,
                taker_ticker.as_deref(),
            ).await {
                Ok(result) => {
                    let embed = swap_service::create_swap_embed(&result);
                    msg.channel_id
                        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Err(e) => {
                    msg.reply(ctx, format!("❌ Swap failed: {}", e)).await
                        .map_err(|e| e.to_string())?;
                }
            }
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
    
    let user_id = cleaned.parse::<i64>()
        .map_err(|_| "Invalid user ID or mention".to_string())?;
    
    // Discord IDs are typically 17-20 digits (at least 100000000000000)
    // This filters out small numbers that are likely amounts, not user IDs
    if user_id < 100000000000000 {
        return Err("Not a valid Discord user ID".to_string());
    }
    
    Ok(user_id)
}
