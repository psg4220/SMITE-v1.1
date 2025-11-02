use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use lazy_static::lazy_static;
use serenity::model::id::UserId;
use tokio::sync::Mutex;

lazy_static! {
    static ref COMMAND_COOLDOWNS: Mutex<HashMap<(UserId, String), u64>> = 
        Mutex::new(HashMap::new());
    
    // Track when we last warned a user about cooldown (to avoid message spam)
    // Key: (UserId, command), Value: timestamp of last warning
    static ref COOLDOWN_WARNINGS: Mutex<HashMap<(UserId, String), u64>> = 
        Mutex::new(HashMap::new());
    
    // Global rate limiting: tracks request timestamps for sliding window (1 second window)
    static ref GLOBAL_REQUESTS: Mutex<Vec<u64>> = Mutex::new(Vec::new());
}

const COOLDOWN_SECONDS: u64 = 5;
const GLOBAL_RATE_LIMIT: u64 = 50;  // requests per second
const RATE_WINDOW_MS: u64 = 1000;    // 1 second in milliseconds

/// Check if a user can execute a command (cooldown not active)
/// Returns Ok(()) if cooldown has passed
/// Returns Err((remaining_seconds, should_send_warning_message)) if still on cooldown
/// The boolean indicates if we should send a warning (true on first violation, false on retries)
pub async fn check_cooldown(user_id: UserId, command: &str) -> Result<(), (u64, bool)> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let command_str = command.to_string();
    let key = (user_id, command_str);
    
    let result = {
        let mut cooldowns = COMMAND_COOLDOWNS.lock().await;
        if let Some(&last_time) = cooldowns.get(&key) {
            let elapsed = now.saturating_sub(last_time);
            if elapsed < COOLDOWN_SECONDS {
                // Still on cooldown - check if we should send a warning
                let remaining = COOLDOWN_SECONDS - elapsed;
                
                // Check if we've already warned about this cooldown
                let mut warnings = COOLDOWN_WARNINGS.lock().await;
                let should_warn = if let Some(&last_warning) = warnings.get(&key) {
                    // Only warn if the warning was from a previous cooldown period
                    last_warning < last_time
                } else {
                    // Never warned, so warn now
                    true
                };
                
                if should_warn {
                    // Record this warning
                    warnings.insert(key.clone(), now);
                }
                
                Err((remaining, should_warn))
            } else {
                cooldowns.insert(key.clone(), now);
                Ok(())
            }
        } else {
            cooldowns.insert(key.clone(), now);
            Ok(())
        }
    };

    result
}

/// Check global rate limit (50 requests per second across all users)
/// Returns Ok(()) if under limit, Err(remaining_ms) if rate limit exceeded
pub async fn check_global_rate_limit() -> Result<(), u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    
    let window_start = now.saturating_sub(RATE_WINDOW_MS);
    
    let mut requests = GLOBAL_REQUESTS.lock().await;
    
    // Remove requests outside the 1-second window
    requests.retain(|&timestamp| timestamp > window_start);
    
    if requests.len() >= GLOBAL_RATE_LIMIT as usize {
        // Rate limit exceeded
        // Calculate when the oldest request will leave the window
        let oldest_request = requests[0];
        let oldest_leaves_at = oldest_request + RATE_WINDOW_MS;
        let remaining_ms = oldest_leaves_at.saturating_sub(now);
        Err(remaining_ms)
    } else {
        // Under limit, record this request
        requests.push(now);
        Ok(())
    }
}

/// Get the cooldown seconds constant
pub fn get_cooldown_seconds() -> u64 {
    COOLDOWN_SECONDS
}
