use serenity::model::channel::Message;
use serenity::prelude::Context;
use crate::db;
use crate::services::permission_service;

pub struct MintResult {
    pub user_id: i64,
    pub amount: f64,
    pub new_balance: f64,
    pub currency_ticker: String,
}

pub async fn execute_mint(
    ctx: &Context,
    msg: &Message,
    user_id: i64,
    amount: f64,
    currency_ticker: &str,
) -> Result<MintResult, String> {
    // Check permission (Admin or Minter role required for minting)
    let perm_ctx = permission_service::check_permission(
        ctx,
        msg,
        &["Admin", "Minter"],
    )
    .await?;

    let guild_id = perm_ctx.guild_id;

    // Get pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .ok_or("Database not initialized".to_string())?
            .clone()
    };

    // Look up currency by ticker - must belong to current guild for security
    let currency_id = db::currency::get_currency_by_ticker(&pool, guild_id as i64, currency_ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .map(|(id, _, _)| id)
        .ok_or(format!(
            "Currency {} not found in this guild. Please create one with $cc \"Name\" {}",
            currency_ticker, currency_ticker
        ))?;
    
    // SECURITY: Verify the currency and check permissions
    let currency_details = db::currency::get_currency_by_id(&pool, currency_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Currency not found".to_string())?;
    
    let currency_guild_id = currency_details.1;
    
    // If minting a currency from another guild, verify "Minter" role in that guild
    if currency_guild_id != guild_id as i64 {
        // This is a cross-guild mint attempt - check Minter role in target guild
        let target_guild_id = serenity::model::prelude::GuildId::new(currency_guild_id as u64);
        let target_user_id = serenity::model::prelude::UserId::new(msg.author.id.get());
        
        // Get user's roles in the target guild using permission_service
        let target_roles = permission_service::get_user_role_names(ctx, target_guild_id, target_user_id)
            .await?;
        
        // Check if user has Minter role in the target guild (Admin also counts)
        let has_permission = target_roles.contains(&"Minter".to_string()) 
            || target_roles.contains(&"Admin".to_string());
        
        if !has_permission {
            return Err("❌ You must have the 'Minter' or 'Admin' role in the target guild to mint that currency!".to_string());
        }
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
            "❌ Operation blocked: Cannot reduce balance below 0. Current: {}, Requested change: {}",
            current_balance, amount
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
