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
