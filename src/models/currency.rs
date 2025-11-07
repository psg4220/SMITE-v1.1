//! Currency information models

/// Detailed currency information
#[derive(Debug)]
pub struct CurrencyInfo {
    pub name: String,
    pub ticker: String,
    pub total_in_circulation: f64,
    pub account_balance_total: f64,
    pub tax_balance_total: f64,
    pub swap_maker_total: f64,
    pub date_created: String,
}
