use serenity::model::channel::Message;
use serenity::prelude::Context;
use serenity::model::prelude::{GuildId, UserId};
use tracing::debug;

pub struct PermissionContext {
    pub user_id: u64,
    pub guild_id: u64,
    pub user_roles: Vec<String>,
}

/// Get user's role names from a guild
pub async fn get_user_role_names(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
) -> Result<Vec<String>, String> {
    debug!("Getting roles for user {} in guild {}", user_id, guild_id);
    
    // Fetch Member via HTTP API
    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(e) => {
            // If user is not a member of this guild, return empty roles
            debug!("User {} is not a member of guild {}: {}", user_id, guild_id, e);
            return Ok(Vec::new());
        }
    };

    // Try to get guild roles from cache
    if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
        let roles: Vec<String> = member
            .roles
            .iter()
            .filter_map(|rid| guild.roles.get(rid))
            .map(|role| role.name.clone())
            .collect();

        debug!("User {} roles in guild {} (from cache): {:?}", user_id, guild_id, roles);
        return Ok(roles);
    }

    // Fallback: fetch roles via API if not cached
    let guild = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map_err(|e| format!("Failed to get guild: {}", e))?;
    let roles: Vec<String> = member
        .roles
        .iter()
        .filter_map(|rid| guild.roles.get(rid))
        .map(|role| role.name.clone())
        .collect();

    debug!("User {} roles in guild {} (from API): {:?}", user_id, guild_id, roles);

    Ok(roles)
}

/// Check if a user has the required roles in a guild.
/// 
/// Parameters:
/// - `ctx`: Serenity context
/// - `msg`: The message that triggered the command
/// - `required_roles`: Array of role names (e.g., ["Admin", "Minter"])
///   * If "Admin" role is present, user automatically passes all checks
///   * Otherwise, user must have at least one role from this list
/// 
/// Returns PermissionContext with user info and their roles, or an error if:
/// - Command used outside a guild
/// - User doesn't have required roles (unless they have "Admin")
pub async fn check_permission(
    ctx: &Context,
    msg: &Message,
    required_roles: &[&str],
) -> Result<PermissionContext, String> {
    // Guild is required
    let guild_id = msg
        .guild_id
        .ok_or("This command can only be used in a guild".to_string())?;

    let user_id = msg.author.id;

    // Get user's role names using the helper function
    let mut user_roles = get_user_role_names(ctx, guild_id, user_id).await?;

    // Check if user is the guild owner - they always have implicit admin
    let guild = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map_err(|e| format!("Failed to get guild: {}", e))?;
    
    if user_id.get() == guild.owner_id.get() {
        user_roles.push("Admin".to_string());
    }

    // Debug: log user roles
    debug!("User roles: {:?}", user_roles);
    
    // Check if user has "Admin" role - if so, they automatically pass
    if user_roles.contains(&"Admin".to_string()) {
        return Ok(PermissionContext {
            user_id: user_id.get(),
            guild_id: guild_id.get(),
            user_roles,
        });
    }

    // Check if user has any of the required roles
    let has_required_role = required_roles
        .iter()
        .any(|req_role| user_roles.iter().any(|ur| ur == req_role));

    if !has_required_role {
        return Err(format!(
            "You need one of these roles to use this command: {}",
            required_roles.join(", ")
        ));
    }

    Ok(PermissionContext {
        user_id: user_id.get(),
        guild_id: guild_id.get(),
        user_roles,
    })
}
