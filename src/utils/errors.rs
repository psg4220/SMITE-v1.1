use thiserror::Error;
use crate::utils::encryption::CryptoError;

/// Extract clean error message from database error strings
/// 
/// Removes technical error codes and prefixes like:
/// "error returned from database: 1644 (45000): Insufficient balance to accept swap"
/// 
/// Returns only the meaningful error message:
/// "Insufficient balance to accept swap"
pub fn extract_clean_error(error_msg: &str) -> String {
    if error_msg.contains("error returned from database:") {
        // Find the last colon, everything after it is the actual error message
        if let Some(last_colon) = error_msg.rfind(": ") {
            error_msg[last_colon + 2..].trim().to_string()
        } else {
            error_msg.to_string()
        }
    } else {
        error_msg.to_string()
    }
}

/// Wire command errors with Discord embed formatting
#[derive(Debug, Error)]
pub enum WireError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Encryption error: {0}")]
    Crypto(#[from] CryptoError),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Transaction error: {0}")]
    Transaction(String),
    
    #[error("Compensation failed: {0}")]
    CompensationFailed(String),
}

impl WireError {
    /// Convert WireError to a colored Discord embed for user-facing errors
    pub fn to_embed(&self) -> serenity::builder::CreateEmbed {
        match self {
            WireError::Database(msg) => {
                serenity::builder::CreateEmbed::default()
                    .title("❌ Database Error")
                    .description(format!("An internal database error occurred:\n```\n{}\n```", msg))
                    .color(0xff0000) // Red
            }
            WireError::Crypto(crypto_err) => {
                serenity::builder::CreateEmbed::default()
                    .title("🔐 Encryption Error")
                    .description(format!("Failed to process security layer:\n```\n{}\n```", crypto_err))
                    .color(0xff8800) // Orange
            }
            WireError::Api(msg) => {
                let (color, title) = if msg.contains("token") || msg.contains("auth") || msg.contains("401") || msg.contains("403") {
                    (0xff0000, "🔑 Invalid API Token") // Red for auth errors
                } else {
                    (0xff8800, "⚠️ API Error") // Orange for other API errors
                };
                
                serenity::builder::CreateEmbed::default()
                    .title(title)
                    .description(format!(
                        "UnbelievaBoat API communication failed:\n```\n{}\n```\n\n**Troubleshooting:**\n\
                        • Verify your API token is correct: `$wire set token <your_token>`\n\
                        • Check UnbelievaBoat server status\n\
                        • Try again in a few moments",
                        msg
                    ))
                    .color(color)
            }
            WireError::InsufficientBalance(msg) => {
                serenity::builder::CreateEmbed::default()
                    .title("💸 Insufficient Balance")
                    .description(msg)
                    .color(0xffaa00) // Yellow-orange
            }
            WireError::InvalidConfig(msg) => {
                serenity::builder::CreateEmbed::default()
                    .title("⚙️ Configuration Issue")
                    .description(msg)
                    .color(0xff8800) // Orange
            }
            WireError::Transaction(msg) => {
                serenity::builder::CreateEmbed::default()
                    .title("❌ Transaction Failed")
                    .description(format!("Database transaction error:\n```\n{}\n```", msg))
                    .color(0xff0000) // Red
            }
            WireError::CompensationFailed(msg) => {
                serenity::builder::CreateEmbed::default()
                    .title("⚠️ Compensation Failed")
                    .description(format!(
                        "API failed AND automatic balance restoration failed. **Critical error**:\n```\n{}\n```\n\n\
                        ⚠️ **Please contact server administrators immediately!**",
                        msg
                    ))
                    .color(0xff0000) // Red
            }
        }
    }
}
