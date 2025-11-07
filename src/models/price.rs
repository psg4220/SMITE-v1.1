//! Price query models

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
