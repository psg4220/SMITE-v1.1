//! Swap/trading models

/// Result of creating a swap
#[derive(Debug)]
pub struct SwapResult {
    pub swap_id: i64,
    pub maker_id: i64,
    pub taker_id: Option<i64>,
    pub maker_amount: String,
    pub maker_currency: String,
    pub taker_amount: String,
    pub taker_currency: String,
    pub status: String,
}

/// Result of accepting or denying a swap
#[derive(Debug)]
pub struct AcceptDenyResult {
    pub swap_id: i64,
    pub maker_id: i64,
    pub taker_id: i64,
    pub maker_offer: String,
    pub taker_offer: String,
    pub status: String,
}

/// Paginated list of swaps
#[derive(Debug)]
pub struct SwapListResult {
    pub swaps: Vec<(i64, i64, Option<i64>, String, String, f64, f64, String)>,  // (id, maker_id, taker_id, maker_ticker, taker_ticker, maker_amount, taker_amount, status)
    pub current_page: usize,
    pub total_pages: usize,
    pub total_swaps: i64,
}
