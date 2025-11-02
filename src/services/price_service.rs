use sqlx::mysql::MySqlPool;
use crate::db;

/// Result struct for price query
#[derive(Debug)]
pub struct PriceResult {
    pub base_ticker: String,
    pub quote_ticker: String,
    pub timeframe: String,
    pub last_price: f64,
    pub vwap: Option<f64>,
    pub is_reversed: bool,
}

/// Convert user-friendly timeframe string to MySQL INTERVAL format
/// Examples: "1m" -> "1 MINUTE", "1h" -> "1 HOUR", "1d" -> "1 DAY", etc.
pub fn parse_timeframe(timeframe: &str) -> Result<String, String> {
    let timeframe = timeframe.to_lowercase();
    
    // Find where the letters start
    let split_idx = timeframe.chars().take_while(|c| c.is_numeric()).count();
    
    if split_idx == 0 || split_idx == timeframe.len() {
        return Err("❌ Invalid timeframe format. Examples: 1m, 5m, 1h, 4h, 1d, 7d, 1mnt, 1y".to_string());
    }
    
    let amount = &timeframe[..split_idx];
    let unit = &timeframe[split_idx..];
    
    let interval_unit = match unit {
        "m" => "MINUTE",
        "h" => "HOUR",
        "d" => "DAY",
        "mnt" => "MONTH",
        "y" => "YEAR",
        _ => return Err(format!("❌ Unknown timeframe unit: '{}'. Use: m, h, d, mnt, y", unit)),
    };
    
    Ok(format!("{} {}", amount, interval_unit))
}

/// Get price and VWAP for a currency pair
pub async fn get_price(
    pool: &MySqlPool,
    base_ticker: &str,
    quote_ticker: &str,
    timeframe_arg: &str,
) -> Result<PriceResult, String> {
    // Validate inputs
    if base_ticker.is_empty() || quote_ticker.is_empty() {
        return Err("❌ Base and quote currencies cannot be empty".to_string());
    }

    if base_ticker == quote_ticker {
        return Err("❌ Base and quote currencies must be different".to_string());
    }

    // Get currency IDs by tickers
    let base_currency = db::currency::get_currency_by_ticker(pool, base_ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("❌ Currency '{}' not found", base_ticker))?;

    let quote_currency = db::currency::get_currency_by_ticker(pool, quote_ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("❌ Currency '{}' not found", quote_ticker))?;

    let base_currency_id = base_currency.0;
    let quote_currency_id = quote_currency.0;

    // Get the canonical order
    let (canonical_base_id, canonical_quote_id, is_reversed) = 
        db::tradelog::normalize_pair(pool, base_currency_id, quote_currency_id)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

    // Parse timeframe argument (default to 24h if not provided)
    let mysql_timeframe = parse_timeframe(timeframe_arg)?;

    // Get the latest price
    let price_result = db::tradelog::get_latest_price_for_pair(pool, canonical_base_id, canonical_quote_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let (canonical_price, _) = price_result
        .ok_or("❌ No trading history found for this pair. Please execute a swap first.")?;

    // Calculate the price for the requested order
    let displayed_price = if is_reversed {
        1.0 / canonical_price
    } else {
        canonical_price
    };

    // Calculate VWAP with the specified timeframe
    let vwap_result = db::tradelog::calculate_vwap(pool, canonical_base_id, canonical_quote_id, &mysql_timeframe)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let vwap_displayed = vwap_result.map(|vwap| {
        if is_reversed {
            1.0 / vwap
        } else {
            vwap
        }
    });

    Ok(PriceResult {
        base_ticker: base_ticker.to_string(),
        quote_ticker: quote_ticker.to_string(),
        timeframe: timeframe_arg.to_string(),
        last_price: displayed_price,
        vwap: vwap_displayed,
        is_reversed,
    })
}
