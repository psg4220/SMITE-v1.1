pub mod table;
pub mod page;
pub mod errors;
pub mod ratelimit;

pub use table::Table;
pub use page::Page;
pub use errors::extract_clean_error;
pub use ratelimit::{check_cooldown, check_global_rate_limit, get_cooldown_seconds};

/// Check if a user has required roles in a guild (case-insensitive)
/// Automatically grants "admin" to guild owner
pub async fn check_user_roles(
    ctx: &serenity::prelude::Context,
    guild_id: serenity::model::prelude::GuildId,
    user_id: serenity::model::prelude::UserId,
    required_roles: &[&str],
) -> Result<(), String> {
    use tracing::debug;

    // Fetch Member via HTTP API
    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(e) => {
            debug!("User {} is not a member of guild {}: {}", user_id, guild_id, e);
            return Err("User is not a member of this guild".to_string());
        }
    };

    debug!("Member has {} role IDs", member.roles.len());

    // Get user's role names
    let mut user_roles = Vec::new();

    // Try to get guild roles from cache first
    {
        if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
            for rid in &member.roles {
                if let Some(role) = guild.roles.get(rid) {
                    user_roles.push(role.name.clone());
                }
            }
            debug!("User {} roles in guild {} (from cache): {:?}", user_id, guild_id, user_roles);
        }
    } // Cache lock is dropped here

    // If no roles found in cache, fetch from API
    if user_roles.is_empty() {
        debug!("Guild not in cache, fetching roles from API");
        let all_roles = guild_id.roles(&ctx.http)
            .await
            .map_err(|e| format!("Failed to get guild roles: {}", e))?;
        
        debug!("Fetched {} total roles from guild", all_roles.len());
        
        for rid in &member.roles {
            if let Some(role) = all_roles.get(rid) {
                user_roles.push(role.name.clone());
            }
        }
        debug!("User {} roles in guild {} (from API): {:?}", user_id, guild_id, user_roles);
    }

    // Check if user is the guild owner - they always have implicit admin
    let guild = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map_err(|e| format!("Failed to get guild: {}", e))?;
    
    debug!("Guild owner ID: {}, User ID: {}, Is owner: {}", guild.owner_id.get(), user_id.get(), user_id.get() == guild.owner_id.get());
    
    if user_id.get() == guild.owner_id.get() {
        debug!("User is guild owner, granting admin role");
        user_roles.push("Admin".to_string());
    }

    debug!("Final user roles: {:?}", user_roles);
    debug!("Required roles: {:?}", required_roles);

    // Check if user has "Admin" role (case-insensitive) - if so, they pass all checks
    if user_roles.iter().any(|r| r.to_lowercase() == "admin") {
        debug!("User has admin role, granting access");
        return Ok(());
    }

    // Check if user has any of the required roles (case-insensitive)
    let has_required_role = required_roles
        .iter()
        .any(|req_role| user_roles.iter().any(|ur| ur.to_lowercase() == req_role.to_lowercase()));

    debug!("User has required role: {}", has_required_role);

    if !has_required_role {
        return Err(format!(
            "You need one of these roles to use this command: {}",
            required_roles.join(", ")
        ));
    }

    Ok(())
}
