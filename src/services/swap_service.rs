use serenity::model::channel::Message;
use serenity::prelude::Context;
use serenity::model::prelude::UserId;
use crate::db;

pub struct SwapResult {
    pub swap_id: i64,
    pub maker_id: i64,
    pub taker_id: Option<i64>,
    pub maker_amount: String,
    pub maker_currency: String,
    pub taker_amount: String,
    pub taker_currency: String,
    pub status: String
}

pub struct AcceptDenyResult {
    pub success_count: usize,
    pub fail_count: usize,
    pub message: String,
    pub swap_id: i64,
    pub maker_id: i64,
    pub taker_id: i64,
    pub maker_offer: String,
    pub taker_offer: String,
    pub status: String,
}

pub async fn execute_swap(
    ctx: &Context,
    msg: &Message,
    maker_id: i64,
    maker_amount: f64,
    maker_ticker: &str,
    taker_id: Option<i64>,
    taker_amount: Option<f64>,
    taker_ticker: Option<&str>,
) -> Result<SwapResult, String> {
    // Get guild_id if available (works for both guild and DM)
    let guild_id = msg.guild_id.map(|id| id.get() as i64).unwrap_or(0);

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };
    
    // Get maker's currency by ticker
    let maker_currency = db::currency::get_currency_by_ticker(&pool, guild_id, maker_ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("Currency {} not found", maker_ticker))?;
    let maker_currency_id = maker_currency.0;
    let maker_currency_name = maker_currency.2;
    
    // Get maker's account ID (must exist)
    let maker_account_id = db::account::get_account_id(&pool, maker_id, maker_currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Maker has no account for this currency".to_string())?;
    
    // Verify maker has sufficient balance
    let maker_balance = db::account::get_account_balance(&pool, maker_id, maker_currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Maker has no account".to_string())?;
    
    if maker_balance < maker_amount {
        return Err(format!("Maker has insufficient {} balance", maker_ticker));
    }
    
    // If taker is specified, this is a targeted swap
    if let (Some(taker_id_val), Some(taker_amount_val), Some(taker_ticker_val)) = (taker_id, taker_amount, taker_ticker) {
        // Get taker's currency by ticker
        let taker_currency = db::currency::get_currency_by_ticker(&pool, guild_id, taker_ticker_val)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or(format!("Currency {} not found", taker_ticker_val))?;
        let taker_currency_id = taker_currency.0;
        let taker_currency_name = taker_currency.2;
        
        // Get or create taker account for their currency
        let taker_account_id = db::account::get_account_id(&pool, taker_id_val, taker_currency_id).await
            .map_err(|e| format!("Database error: {}", e))?;
        
        let taker_account_id_final = if let Some(id) = taker_account_id {
            id
        } else {
            db::account::create_account(&pool, taker_id_val, taker_currency_id)
                .await
                .map_err(|e| format!("Failed to create taker account: {}", e))?
        };
        
        // Verify taker has sufficient balance in their currency
        let taker_balance = db::account::get_account_balance(&pool, taker_id_val, taker_currency_id)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or("Taker has no account".to_string())?;
        
        if taker_balance < taker_amount_val {
            return Err(format!("Taker has insufficient {} balance", taker_ticker_val));
        }
        
        // DEDUCT MAKER'S BALANCE IMMEDIATELY (taker will be deducted on accept)
        // Maker loses their currency
        db::account::update_balance(&pool, maker_account_id, -maker_amount)
            .await
            .map_err(|e| format!("Failed to deduct maker balance: {}", e))?;
        
        // Create the swap record (direct insert, no stored procedures)
        let swap_id = db::swap::insert_swap(
            &pool,
            maker_account_id,
            Some(taker_account_id_final),
            maker_currency_id,
            taker_currency_id,
            maker_amount,
            taker_amount_val,
        ).await
        .map_err(|e| format!("Failed to create swap: {}", e))?;
        
        // Send DM to taker if in mutual guild
        let taker_user_id = UserId::new(taker_id_val as u64);
        if let Ok(_) = taker_user_id.to_user(ctx).await {
            let msg_guild_id = msg.guild_id;
            if let Some(guild_id_obj) = msg_guild_id {
                if let Ok(_) = guild_id_obj.member(ctx, taker_user_id).await {
                    let embed = serenity::builder::CreateEmbed::default()
                        .title("🔄 Swap Request")
                        .description(format!("<@{}> has initiated a swap with you", maker_id))
                        .field("Swap ID", format!("`{}`", swap_id), false)
                        .field("Maker Offers", format!("`{:.2} {}`", maker_amount, maker_currency_name), true)
                        .field("Maker Wants", format!("`{:.2} {}`", taker_amount_val, taker_currency_name), true)
                        .field("Status", "⏳ **Awaiting Acceptance**", false)
                        .field("To Accept", format!("`$swap accept {}`", swap_id), true)
                        .field("To Deny", format!("`$swap deny {}`", swap_id), true)
                        .footer(serenity::builder::CreateEmbedFooter::new("ℹ️ Balances have been deducted. They will be credited when you accept."))
                        .color(0xffa500);
                    
                    let _ = taker_user_id.dm(ctx, serenity::builder::CreateMessage::default().embed(embed)).await;
                }
            }
        }
        
        // Store the message ID for later editing
        let _ = db::swap::store_swap_message(&pool, swap_id, msg.channel_id.get() as i64, msg.id.get() as i64).await;
        
        Ok(SwapResult {
            swap_id,
            maker_id,
            taker_id: Some(taker_id_val),
            maker_amount: format!("{:.2}", maker_amount),
            maker_currency: maker_currency_name,
            taker_amount: format!("{:.2}", taker_amount_val),
            taker_currency: taker_currency_name,
            status: "pending".to_string()
        })
    } else {
        // Open swap - taker_id is NULL, anyone can accept
        // Deduct maker's currency immediately
        db::account::update_balance(&pool, maker_account_id, -maker_amount)
            .await
            .map_err(|e| format!("Failed to deduct maker balance: {}", e))?;
        
        let swap_id = db::swap::insert_swap(
            &pool,
            maker_account_id,
            None,  // taker_id is NULL for open swaps
            maker_currency_id,
            maker_currency_id,
            maker_amount,
            0.0,
        ).await
        .map_err(|e| format!("Failed to create open swap: {}", e))?;
        
        Ok(SwapResult {
            swap_id,
            maker_id,
            taker_id: None,
            maker_amount: format!("{:.2}", maker_amount),
            maker_currency: maker_currency_name,
            taker_amount: "?".to_string(),
            taker_currency: "?".to_string(),
            status: "open".to_string(),
        })
    }
}

pub async fn accept_swap(
    ctx: &Context,
    msg: &Message,
    swap_id: Option<i64>,
) -> Result<(AcceptDenyResult, Option<u64>), String> {
    let user_id = msg.author.id.get() as i64;
    
    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };
    
    if let Some(id) = swap_id {
        // Accept a specific swap by ID
        // Get swap details: (id, maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status)
        let swap_details = db::swap::get_swap_by_id(&pool, id).await
            .map_err(|e| format!("Failed to fetch swap: {}", e))?
            .ok_or("Swap not found".to_string())?;
        
        let status = swap_details.7.as_str();
        if status != "pending" {
            if status == "accepted" {
                return Err("❌ This swap has already been accepted!".to_string());
            } else if status == "cancelled" {
                return Err("❌ This swap has been cancelled!".to_string());
            } else if status == "expired" {
                return Err("❌ This swap has expired!".to_string());
            }
            return Err(format!("❌ Swap status is '{}', cannot accept.", status));
        }
        
        let _swap_id = swap_details.0;
        let maker_account_id = swap_details.1;
        let taker_id_existing = swap_details.2;
        let maker_currency_id = swap_details.3;
        let taker_currency_id = swap_details.4;
        let maker_amount = swap_details.5;
        let taker_amount = swap_details.6;
        
        // Get the actual Discord user IDs from account IDs
        let maker_discord_id = db::account::get_discord_id_by_account_id(&pool, maker_account_id)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or("Maker account not found".to_string())?;
        
        let taker_discord_id = if let Some(taker_account_id) = taker_id_existing {
            db::account::get_discord_id_by_account_id(&pool, taker_account_id)
                .await
                .map_err(|e| format!("Database error: {}", e))?
                .ok_or("Taker account not found".to_string())?
        } else {
            0 // Open swap has no taker yet
        };
        
        // SECURITY: Verify user is authorized to accept this swap
        if taker_discord_id != 0 {
            // Targeted swap: Only the taker can accept
            if user_id != taker_discord_id {
                return Err("❌ You are not authorized to accept this swap. Only the designated taker can accept targeted swaps.".to_string());
            }
        } else {
            // Open swap: The maker CANNOT accept their own swap
            if user_id == maker_discord_id {
                return Err("❌ You cannot accept your own open swap. Another user must accept it.".to_string());
            }
        }
        
        // Get user's account for taker currency
        let user_taker_account_id = db::account::get_account_id(&pool, user_id, taker_currency_id).await
            .map_err(|e| format!("Database error: {}", e))?;
        
        let user_taker_account_id_final = if let Some(id) = user_taker_account_id {
            id
        } else {
            db::account::create_account(&pool, user_id, taker_currency_id)
                .await
                .map_err(|e| format!("Failed to create user account: {}", e))?
        };
        
        // Get maker's account for their currency
        let maker_currency_account_id = db::account::get_account_id(&pool, user_id, maker_currency_id).await
            .map_err(|e| format!("Database error: {}", e))?;
        
        let _maker_currency_account_id_final = if let Some(id) = maker_currency_account_id {
            id
        } else {
            db::account::create_account(&pool, user_id, maker_currency_id)
                .await
                .map_err(|e| format!("Failed to create user account: {}", e))?
        };
        
        // ATOMIC CHECK: Verify taker has sufficient balance BEFORE making any changes
        if taker_id_existing.is_some() {
            // Only check balance for targeted swaps (open swaps have taker_amount = 0)
            let taker_balance = db::account::get_account_balance(&pool, user_id, taker_currency_id)
                .await
                .map_err(|e| format!("Database error: {}", e))?
                .ok_or("Taker has no account".to_string())?;
            
            // Verify taker has sufficient balance
            if taker_balance < taker_amount {
                return Err(format!("❌ Insufficient balance. You need {:.2} but only have {:.2}", taker_amount, taker_balance));
            }
        }
        
        // If this is an OPEN swap (taker_id_existing is None), we need to set the taker
        if taker_id_existing.is_none() {
            // For open swaps, deduct from the acceptor (new taker)
            db::account::update_balance(&pool, user_taker_account_id_final, -taker_amount)
                .await
                .map_err(|e| format!("Failed to deduct taker balance: {}", e))?;
            
            db::swap::update_swap_with_taker(&pool, id, user_taker_account_id_final, "accepted")
                .await
                .map_err(|e| format!("Failed to update swap: {}", e))?;
        } else {
            // Targeted swap - deduct from taker's balance NOW (maker was already deducted in execute_swap)
            db::account::update_balance(&pool, user_taker_account_id_final, -taker_amount)
                .await
                .map_err(|e| format!("Failed to deduct taker balance: {}", e))?;
            
            // Just update status
            db::swap::update_swap_status(&pool, id, "accepted")
                .await
                .map_err(|e| format!("Failed to update swap status: {}", e))?;
        }
        
        // Credit balances:
        // User (taker/acceptor) receives maker's currency
        // Need to get the user's account for maker_currency_id
        let user_maker_account_id = db::account::get_account_id(&pool, user_id, maker_currency_id).await
            .map_err(|e| format!("Database error: {}", e))?;
        
        let user_maker_account_id_final = if let Some(id) = user_maker_account_id {
            id
        } else {
            db::account::create_account(&pool, user_id, maker_currency_id)
                .await
                .map_err(|e| format!("Failed to create user account for maker currency: {}", e))?
        };
        
        db::account::update_balance(&pool, user_maker_account_id_final, maker_amount)
            .await
            .map_err(|e| format!("Failed to credit user with maker currency: {}", e))?;
        
        // Maker receives taker's currency
        // Get the maker's account for taker_currency_id
        let maker_taker_account_id = db::account::get_account_id(&pool, maker_discord_id, taker_currency_id).await
            .map_err(|e| format!("Database error: {}", e))?;
        
        let maker_taker_account_id_final = if let Some(id) = maker_taker_account_id {
            id
        } else {
            db::account::create_account(&pool, maker_discord_id, taker_currency_id)
                .await
                .map_err(|e| format!("Failed to create maker account for taker currency: {}", e))?
        };
        
        db::account::update_balance(&pool, maker_taker_account_id_final, taker_amount)
            .await
            .map_err(|e| format!("Failed to credit maker with taker currency: {}", e))?;
        
        // Log transactions for the swap (2 transactions total)
        // Transaction 1: Taker sends taker_currency to Maker
        let transaction_uuid_1 = uuid::Uuid::new_v4().to_string();
        db::transaction::create_transaction(
            &pool,
            &transaction_uuid_1,
            user_taker_account_id_final,
            maker_taker_account_id_final,
            taker_amount,
        ).await
        .map_err(|e| format!("Failed to log transaction 1: {}", e))?;
        
        // Transaction 2: Maker sends maker_currency to Taker
        let transaction_uuid_2 = uuid::Uuid::new_v4().to_string();
        db::transaction::create_transaction(
            &pool,
            &transaction_uuid_2,
            maker_account_id,
            user_maker_account_id_final,
            maker_amount,
        ).await
        .map_err(|e| format!("Failed to log transaction 2: {}", e))?;
        
        // Get currency tickers
        let maker_currency_ticker = db::currency::get_currency_by_id(&pool, maker_currency_id)
            .await
            .unwrap_or(None)
            .map(|c| c.3)
            .unwrap_or_else(|| "???".to_string());
        let taker_currency_ticker = db::currency::get_currency_by_id(&pool, taker_currency_id)
            .await
            .unwrap_or(None)
            .map(|c| c.3)
            .unwrap_or_else(|| "???".to_string());
        
        Ok((AcceptDenyResult {
            success_count: 1,
            fail_count: 0,
            message: format!("✅ Swap `{}` accepted! Balances have been credited.", id),
            swap_id: id,
            maker_id: maker_discord_id,
            taker_id: user_id,
            maker_offer: format!("{:.2} {}", maker_amount, maker_currency_ticker),
            taker_offer: format!("{:.2} {}", taker_amount, taker_currency_ticker),
            status: "accepted".to_string(),
        }, Some(msg.id.get())))
    } else {
        // Accept all pending swaps - not typically used, but keep for compatibility
        Err("Please specify a swap ID with `$swap accept <id>`".to_string())
    }
}

pub async fn deny_swap(
    ctx: &Context,
    msg: &Message,
    swap_id: Option<i64>,
) -> Result<(AcceptDenyResult, Option<u64>), String> {
    let user_id = msg.author.id.get() as i64;
    
    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };
    
    if let Some(id) = swap_id {
        // Deny specific swap
        let swap_details = db::swap::get_swap_by_id(&pool, id).await
            .map_err(|e| format!("Failed to fetch swap: {}", e))?
            .ok_or("Swap not found".to_string())?;
        
        let status = swap_details.7.as_str();
        if status != "pending" {
            if status == "accepted" {
                return Err("❌ This swap has already been accepted!".to_string());
            } else if status == "cancelled" {
                return Err("❌ This swap has already been cancelled!".to_string());
            } else if status == "expired" {
                return Err("❌ This swap has expired!".to_string());
            }
            return Err(format!("❌ Swap status is '{}', cannot deny.", status));
        }
        
        let maker_account_id = swap_details.1;
        let taker_id_existing = swap_details.2;
        
        // Get the actual Discord user IDs from account IDs
        let maker_discord_id = db::account::get_discord_id_by_account_id(&pool, maker_account_id)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or("Maker account not found".to_string())?;
        
        let taker_discord_id = if let Some(taker_account_id) = taker_id_existing {
            db::account::get_discord_id_by_account_id(&pool, taker_account_id)
                .await
                .map_err(|e| format!("Database error: {}", e))?
                .ok_or("Taker account not found".to_string())?
        } else {
            0
        };
        
        // SECURITY: Only the maker or the taker can deny a swap
        let is_authorized = (user_id == maker_discord_id) || (taker_discord_id != 0 && user_id == taker_discord_id);
        if !is_authorized {
            return Err("❌ You are not authorized to deny this swap. Only the maker or taker can deny.".to_string());
        }
        let maker_amount = swap_details.5;
        let taker_amount = swap_details.6;
        
        // Mark swap as cancelled
        db::swap::update_swap_status(&pool, id, "cancelled")
            .await
            .map_err(|e| format!("Failed to cancel swap: {}", e))?;
        
        // REFUND BALANCES
        // Maker gets their currency back
        db::account::update_balance(&pool, maker_account_id, maker_amount)
            .await
            .map_err(|e| format!("Failed to refund maker balance: {}", e))?;
        
        // If targeted swap, refund taker too
        if let Some(taker_account_id) = taker_id_existing {
            db::account::update_balance(&pool, taker_account_id, taker_amount)
                .await
                .map_err(|e| format!("Failed to refund taker balance: {}", e))?;
        }
        
        // Get currency names
        let maker_currency_id = swap_details.3;
        let taker_currency_id = swap_details.4;
        let maker_currency_ticker = db::currency::get_currency_by_id(&pool, maker_currency_id)
            .await
            .unwrap_or(None)
            .map(|c| c.3)
            .unwrap_or_else(|| "???".to_string());
        let taker_currency_ticker = db::currency::get_currency_by_id(&pool, taker_currency_id)
            .await
            .unwrap_or(None)
            .map(|c| c.3)
            .unwrap_or_else(|| "???".to_string());
        
        let taker_discord_id_final = if let Some(_) = taker_id_existing {
            taker_discord_id
        } else {
            0 // Open swap, no specific taker
        };
        
        Ok((AcceptDenyResult {
            success_count: 1,
            fail_count: 0,
            message: format!("❌ Swap `{}` denied! Balances have been refunded.", id),
            swap_id: id,
            maker_id: maker_discord_id,
            taker_id: taker_discord_id_final,
            maker_offer: format!("{:.2} {}", maker_amount, maker_currency_ticker),
            taker_offer: format!("{:.2} {}", taker_amount, taker_currency_ticker),
            status: "cancelled".to_string(),
        }, Some(msg.id.get())))
    } else {
        // Deny all pending swaps - not typically used
        Err("Please specify a swap ID with `$swap deny <id>`".to_string())
    }
}

pub async fn get_swap_status(
    ctx: &Context,
    _msg: &Message,
    swap_id: i64,
) -> Result<serenity::builder::CreateEmbed, String> {
    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };
    
    // Fetch swap details
    let swap_details = db::swap::get_swap_by_id(&pool, swap_id).await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Swap not found".to_string())?;
    
    let maker_account_id = swap_details.1;
    let taker_account_id = swap_details.2;
    let maker_currency_id = swap_details.3;
    let taker_currency_id = swap_details.4;
    let maker_amount = swap_details.5;
    let taker_amount = swap_details.6;
    let status = swap_details.7.as_str();
    
    // Get Discord IDs from account IDs
    let maker_discord_id = db::account::get_discord_id_by_account_id(&pool, maker_account_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Maker account not found".to_string())?;
    
    let taker_discord_id = if let Some(taker_acc_id) = taker_account_id {
        db::account::get_discord_id_by_account_id(&pool, taker_acc_id)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or("Taker account not found".to_string())?
    } else {
        0
    };
    
    // Get currency tickers
    let maker_ticker = db::currency::get_currency_by_id(&pool, maker_currency_id)
        .await
        .unwrap_or(None)
        .map(|c| c.3)
        .unwrap_or_else(|| "???".to_string());
    
    let taker_ticker = db::currency::get_currency_by_id(&pool, taker_currency_id)
        .await
        .unwrap_or(None)
        .map(|c| c.3)
        .unwrap_or_else(|| "???".to_string());
    
    // Build the embed
    let title = match status {
        "pending" => "⏳ Swap Pending",
        "accepted" => "✅ Swap Accepted",
        "cancelled" => "❌ Swap Cancelled",
        "expired" => "⏱️ Swap Expired",
        _ => "🔄 Swap Status",
    };
    
    let color = match status {
        "pending" => 0xffa500,    // Orange
        "accepted" => 0x00ff00,   // Green
        "cancelled" => 0xff0000,  // Red
        "expired" => 0x808080,    // Gray
        _ => 0x9900ff,            // Purple
    };
    
    let mut embed = serenity::builder::CreateEmbed::default()
        .title(title)
        .field("Swap ID", format!("`{}`", swap_id), true)
        .field("Status", format!("**{}**", status), true)
        .field("Maker", format!("<@{}>", maker_discord_id), true)
        .field("Maker Offers", format!("`{:.2} {}`", maker_amount, maker_ticker), true);
    
    if taker_discord_id != 0 {
        embed = embed
            .field("Taker", format!("<@{}>", taker_discord_id), true)
            .field("Taker Wants", format!("`{:.2} {}`", taker_amount, taker_ticker), true);
    } else {
        embed = embed
            .field("Taker", "**Open Swap** (anyone can accept)".to_string(), true)
            .field("Taker Wants", format!("`{:.2} {}`", taker_amount, taker_ticker), true);
    }
    
    embed = embed.color(color);
    Ok(embed)
}

pub fn create_swap_embed(result: &SwapResult) -> serenity::builder::CreateEmbed {
    let mut embed = serenity::builder::CreateEmbed::default()
        .title("🔄 Swap Created")
        .field("Swap ID", format!("`{}`", result.swap_id), true)
        .field("Maker", format!("<@{}>", result.maker_id), true)
        .field("Maker Offers", format!("`{} {}`", result.maker_amount, result.maker_currency), true);
    
    if let Some(taker_id) = result.taker_id {
        embed = embed
            .field("Taker", format!("<@{}>", taker_id), true)
            .field("Taker Wants", format!("`{} {}`", result.taker_amount, result.taker_currency), true)
            .field("Status", format!("**{}**", result.status), false);
    } else {
        embed = embed
            .description("This is an **open swap** - anyone can accept it!")
            .field("Status", format!("**{}**", result.status), false);
    }
    
    embed.color(0xffa500)
}

pub fn create_accept_deny_embed(result: &AcceptDenyResult) -> serenity::builder::CreateEmbed {
    let title = if result.status == "accepted" {
        "✅ Swap Accepted"
    } else {
        "❌ Swap Denied"
    };
    
    let color = if result.status == "accepted" {
        0x00ff00  // Green
    } else {
        0xff0000  // Red
    };
    
    serenity::builder::CreateEmbed::default()
        .title(title)
        .field("Swap ID", format!("`{}`", result.swap_id), true)
        .field("Status", format!("**{}**", result.status), true)
        .field("Maker", format!("<@{}>", result.maker_id), true)
        .field("Maker Offers", result.maker_offer.clone(), true)
        .field("Taker", format!("<@{}>", result.taker_id), true)
        .field("Taker Wants", result.taker_offer.clone(), true)
        .color(color)
}

