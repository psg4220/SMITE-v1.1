pub mod table;
pub mod page;
pub mod errors;
pub mod ratelimit;
pub mod encryption;

pub use errors::extract_clean_error;
pub use ratelimit::{check_cooldown, check_global_rate_limit};
pub use encryption::{encrypt_token, decrypt_token};

/// Check if a user has required roles in a guild (case-insensitive)
/// Special behavior for "admin" role: checks Discord ADMINISTRATOR permission instead of role name
/// Other roles (minter, tax collector, etc.) are checked by name only
pub async fn check_user_roles(
    ctx: &serenity::prelude::Context,
    guild_id: serenity::model::prelude::GuildId,
    user_id: serenity::model::prelude::UserId,
    required_roles: &[&str],
) -> Result<(), String> {
    use tracing::debug;
    use serenity::model::permissions::Permissions;

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
    
    let is_guild_owner = user_id.get() == guild.owner_id.get();
    if is_guild_owner {
        debug!("User is guild owner, granting admin permission");
        user_roles.push("__ADMIN_PERMISSION__".to_string()); // Special marker
    }

    // Check if "admin" is in required roles
    let admin_required = required_roles.iter().any(|r| r.to_lowercase() == "admin");
    
    if admin_required {
        // For admin requirement, check Discord ADMINISTRATOR permission
        // Get all guild roles to check permissions
        let guild_roles = guild_id
            .roles(&ctx.http)
            .await
            .map_err(|e| format!("Failed to get guild roles: {}", e))?;
        
        // Check if any of the member's roles have ADMINISTRATOR permission
        let has_admin_permission = member.roles.iter().any(|role_id| {
            if let Some(role) = guild_roles.get(role_id) {
                role.permissions.contains(Permissions::ADMINISTRATOR)
            } else {
                false
            }
        });

        debug!("User has admin permission via role: {}", has_admin_permission);

        if has_admin_permission {
            debug!("User has ADMINISTRATOR permission");
            return Ok(());
        }

        // If admin not required by permission but is required by role name, return error
        if user_roles.iter().any(|r| r == "__ADMIN_PERMISSION__") {
            // User is guild owner (implicit admin)
            return Ok(());
        }

        // Admin permission not found, check if other required roles are met
        let other_required_roles: Vec<&str> = required_roles
            .iter()
            .filter(|r| r.to_lowercase() != "admin")
            .copied()
            .collect();

        if !other_required_roles.is_empty() {
            // Check other roles
            let has_required_role = other_required_roles
                .iter()
                .any(|req_role| user_roles.iter().any(|ur| ur.to_lowercase() == req_role.to_lowercase()));

            debug!("User has required alternative role: {}", has_required_role);

            if has_required_role {
                return Ok(());
            }
        }

        // No admin permission and no alternative roles found
        return Err(format!(
            "You need ADMINISTRATOR permission or one of these roles to use this command: {}",
            required_roles.join(", ")
        ));
    } else {
        // "admin" not required, just check by role name
        debug!("Final user roles: {:?}", user_roles);
        debug!("Required roles: {:?}", required_roles);

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
    }

    Ok(())
}

/// Ensure encryption key exists in .env, generate if missing
pub fn ensure_encryption_key() -> Result<String, String> {
    use tracing::info;
    use std::fs;
    use std::path::Path;

    // Try to get from environment
    if let Ok(key) = std::env::var("TOKEN_ENCRYPTION_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    // Key not found or empty, generate a new one
    info!("TOKEN_ENCRYPTION_KEY not found in environment, generating new 256-bit key...");
    
    // Generate 32 random bytes (256 bits) for AES256
    use rand::RngCore;
    let mut key_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key_bytes);
    
    let key_hex = hex::encode(key_bytes);
    
    // Try to append to .env file
    let env_path = ".env";
    let env_exists = Path::new(env_path).exists();
    
    let env_content = if env_exists {
        fs::read_to_string(env_path)
            .map_err(|e| format!("Failed to read .env: {}", e))?
    } else {
        String::new()
    };
    
    // Check if key already exists in file (might have been added by another process)
    if env_content.contains("TOKEN_ENCRYPTION_KEY=") {
        // Try to load it again
        return std::env::var("TOKEN_ENCRYPTION_KEY")
            .map_err(|_| "TOKEN_ENCRYPTION_KEY exists in .env but could not be loaded".to_string());
    }
    
    // Append the key to .env
    let new_content = if env_content.is_empty() || env_content.ends_with('\n') {
        format!("{}TOKEN_ENCRYPTION_KEY={}\n", env_content, key_hex)
    } else {
        format!("{}\nTOKEN_ENCRYPTION_KEY={}\n", env_content, key_hex)
    };
    
    fs::write(env_path, new_content)
        .map_err(|e| format!("Failed to write TOKEN_ENCRYPTION_KEY to .env: {}", e))?;
    
    info!("Generated and saved new TOKEN_ENCRYPTION_KEY to .env");
    
    Ok(key_hex)
}

