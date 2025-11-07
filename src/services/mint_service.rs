use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;
use crate::models::MintResult;

// Maximum value for DECIMAL(24,8): 999,999,999,999,999.99999999
const MAX_BALANCE: f64 = 999_999_999_999_999.99999999;

pub async fn execute_mint(
    ctx: &Context,
    msg: &Message,
    user_id: i64,
    amount: f64,
    currency_ticker: &str,
) -> Result<MintResult, String> {
    // Get guild ID (required)
    let guild_id = msg
        .guild_id
        .ok_or("This command can only be used in a guild".to_string())?;

    // Check permission - user must be admin or minter
    crate::utils::check_user_roles(ctx, guild_id, msg.author.id, &["admin", "minter"])
        .await?;

    let guild_id_i64 = guild_id.get() as i64;

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    // Look up currency by ticker
    let currency_id = db::currency::get_currency_by_ticker(&pool, currency_ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .map(|(id, _, _)| id)
        .ok_or_else(|| format!("Currency '{}' not found", currency_ticker))?;
    
    // SECURITY: Verify the currency and check permissions
    let currency_details = db::currency::get_currency_by_id(&pool, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Currency not found".to_string())?;
    
    let currency_guild_id = currency_details.1;
    
    // If minting a currency from another guild, verify permission in that guild
    if currency_guild_id != guild_id_i64 {
        // This is a cross-guild mint attempt - check permission in target guild
        let target_guild_id = serenity::model::prelude::GuildId::new(currency_guild_id as u64);
        
        crate::utils::check_user_roles(ctx, target_guild_id, msg.author.id, &["admin", "minter"])
            .await?;
    }

    // Get or create account
    let account_id = match db::account::get_account_id(&pool, user_id, currency_id).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            // Account doesn't exist, create it
            db::account::create_account(&pool, user_id, currency_id)
                .await
                .map_err(|e| format!("Failed to create account: {}", e))?
        }
        Err(e) => return Err(format!("Database error: {}", e)),
    };

    // Get current balance
    let current_balance = db::account::get_account_balance(&pool, user_id, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .unwrap_or(0.0);

    // Calculate new balance
    let new_balance = current_balance + amount;

    // Prevent negative balance
    if new_balance < 0.0 {
        return Err(format!(
            "❌ Operation blocked: Cannot reduce balance below 0.\n\
             Current balance: {:.8} {}\n\
             Requested change: {:.8} {}\n\
             New balance would be: {:.8} {}",
            current_balance, currency_ticker,
            amount, currency_ticker,
            new_balance, currency_ticker
        ));
    }

    // Check for overflow
    if new_balance > MAX_BALANCE {
        return Err(format!(
            "❌ Operation blocked: Balance would exceed maximum limit.\n\
             Current balance: {:.8} {}\n\
             Requested mint: {:.8} {}\n\
             New balance would be: {:.8} {}\n\
             Maximum allowed: {:.8} {}",
            current_balance, currency_ticker,
            amount, currency_ticker,
            new_balance, currency_ticker,
            MAX_BALANCE, currency_ticker
        ));
    }

    // Update balance
    db::account::update_balance(&pool, account_id, amount).await
        .map_err(|e| format!("Failed to update balance: {}", e))?;

    Ok(MintResult {
        user_id,
        amount,
        new_balance,
        currency_ticker: currency_ticker.to_string(),
    })
}

pub fn create_mint_embed(result: &MintResult) -> serenity::builder::CreateEmbed {
    serenity::builder::CreateEmbed::default()
        .title("💰 Mint Operation")
        .field("User", format!("<@{}>", result.user_id), false)
        .field(
            "Amount Changed",
            format!("{:+.2} {}", result.amount, result.currency_ticker),
            true,
        )
        .field(
            "New Balance",
            format!("{:.2} {}", result.new_balance, result.currency_ticker),
            true,
        )
        .color(0x9900ff)
}
