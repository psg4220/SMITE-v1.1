use async_trait::async_trait;
use sqlx::Error;
use chrono::{DateTime, Utc};

#[async_trait]
pub trait DatabaseBackend: Send + Sync {
    // Account
    async fn create_account(&self, discord_id: i64, currency_id: i64) -> Result<i64, Error>;
    async fn get_account_balance(&self, discord_id: i64, currency_id: i64) -> Result<Option<f64>, Error>;
    async fn get_account_id(&self, discord_id: i64, currency_id: i64) -> Result<Option<i64>, Error>;
    async fn update_balance(&self, account_id: i64, amount: f64) -> Result<(), Error>;
    async fn set_balance(&self, account_id: i64, balance: f64) -> Result<(), Error>;
    async fn get_total_balance(&self, currency_id: i64) -> Result<Option<f64>, Error>;
    async fn get_discord_id_by_account_id(&self, account_id: i64) -> Result<Option<i64>, Error>;
    async fn add_balance(&self, discord_id: i64, currency_id: i64, amount: f64) -> Result<(), Error>;

    // Currency
    async fn create_currency(&self, guild_id: i64, name: &str, ticker: &str) -> Result<i64, Error>;
    async fn get_currency_by_id(&self, currency_id: i64) -> Result<Option<(i64, i64, String, String)>, Error>;
    async fn get_currency_by_ticker(&self, ticker: &str) -> Result<Option<(i64, String, String)>, Error>;
    async fn get_currency_by_ticker_with_guild(&self, ticker: &str) -> Result<Option<(i64, i64, String, String)>, Error>;
    async fn get_currency_by_guild(&self, guild_id: i64) -> Result<Option<(i64, String, String)>, Error>;
    async fn get_all_currencies(&self) -> Result<Vec<(i64, String, String)>, Error>;
    async fn get_currencies_paginated(&self, sort_by: &str, page: usize, page_size: usize) -> Result<(Vec<(i64, String, String)>, i64), Error>;
    async fn get_currency_date(&self, currency_id: i64) -> Result<Option<String>, Error>;

    // Swap
    async fn create_swap(&self, maker_id: i64, maker_amount: f64, maker_currency_id: i64, taker_id: Option<i64>, taker_amount: Option<f64>, taker_currency_id: Option<i64>) -> Result<i64, Error>;
    async fn create_swap_targeted(&self, maker_id: i64, maker_currency_id: i64, taker_currency_id: i64, maker_amount: f64, taker_amount: f64, taker_id: i64) -> Result<i64, Error>;
    async fn create_swap_open(&self, maker_id: i64, maker_currency_id: i64, taker_currency_id: i64, maker_amount: f64, taker_amount: f64) -> Result<i64, Error>;
    async fn get_swap(&self, swap_id: i64) -> Result<Option<(i64, i64, Option<i64>, f64, i64, Option<f64>, Option<i64>, String, String)>, Error>;
    async fn get_swap_by_id(&self, swap_id: i64) -> Result<Option<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, Error>;
    async fn accept_swap(&self, swap_id: i64, taker_id: i64, uuid1: &str, uuid2: &str) -> Result<(), Error>;
    async fn deny_swap(&self, swap_id: i64) -> Result<(), Error>;
    async fn cancel_swap(&self, swap_id: i64) -> Result<(), Error>;
    async fn get_pending_swaps_by_maker(&self, maker_id: i64) -> Result<Vec<(i64, i64, Option<i64>, f64, i64, Option<f64>, Option<i64>, String, String)>, Error>;
    async fn get_swaps_by_maker(&self, maker_id: i64) -> Result<Vec<(i64, i64, Option<i64>, f64, i64, Option<f64>, Option<i64>, String, String)>, Error>;
    async fn get_swaps_by_taker(&self, taker_id: i64) -> Result<Vec<(i64, i64, Option<i64>, f64, i64, Option<f64>, Option<i64>, String, String)>, Error>;
    async fn get_all_pending_swaps(&self) -> Result<Vec<(i64, i64, Option<i64>, f64, i64, Option<f64>, Option<i64>, String, String)>, Error>;
    async fn get_all_open_swaps(&self) -> Result<Vec<(i64, i64, Option<i64>, f64, i64, Option<f64>, Option<i64>, String, String)>, Error>;
    async fn get_swap_message(&self, swap_id: i64) -> Result<Option<(i64, i64)>, Error>;
    async fn store_swap_message(&self, swap_id: i64, channel_id: i64, message_id: i64) -> Result<(), Error>;
    async fn get_total_swap_maker_amount(&self, currency_id: i64) -> Result<Option<f64>, Error>;
    async fn get_swaps_paginated(&self, page: usize, page_size: usize, sort_by: &str, status_filter: &str, base_currency: Option<&str>, quote_currency: Option<&str>) -> Result<(Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String, String, String)>, i64), Error>;

    // Transaction
    async fn create_transaction(&self, uuid: &str, sender_account_id: i64, receiver_account_id: i64, amount: f64) -> Result<(), Error>;
    async fn get_transaction(&self, uuid: &str) -> Result<Option<(i64, i64, f64, String, String)>, Error>;
    async fn get_transactions_by_sender(&self, sender_account_id: i64, limit: i32) -> Result<Vec<(i64, i64, f64, String, String)>, Error>;
    async fn get_transactions_by_receiver(&self, receiver_account_id: i64, limit: i32) -> Result<Vec<(i64, i64, f64, String, String)>, Error>;
    async fn get_user_transactions(&self, user_discord_id: i64, limit: i32) -> Result<Vec<(i64, i64, f64, String, String, String)>, Error>;
    async fn get_user_transactions_paginated(&self, user_discord_id: i64, page: usize, page_size: usize) -> Result<(Vec<(i64, i64, f64, String, String, String)>, i64), Error>;

    // Tradelog
    async fn log_price(&self, base_currency_id: i64, quote_currency_id: i64, price: f64) -> Result<(), Error>;
    async fn get_latest_price(&self, base_currency_id: i64, quote_currency_id: i64) -> Result<Option<f64>, Error>;
    async fn get_latest_price_bidirectional(&self, base_currency_id: i64, quote_currency_id: i64) -> Result<Option<(f64, bool)>, Error>;
    async fn get_price_logs_for_pair(&self, base_currency_id: i64, quote_currency_id: i64, limit: i32) -> Result<Vec<(i64, f64, String)>, Error>;
    async fn get_price_logs_with_timestamps(&self, base_currency_id: i64, quote_currency_id: i64) -> Result<Vec<(i64, f64, String)>, Error>;
    async fn get_price_logs_in_range(&self, base_currency_id: i64, quote_currency_id: i64, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<(i64, f64, String)>, Error>;
    async fn normalize_pair(&self, base_currency_id: i64, quote_currency_id: i64) -> Result<(i64, i64, bool), Error>;
    async fn calculate_vwap(&self, base_currency_id: i64, quote_currency_id: i64, timeframe: &str) -> Result<Option<f64>, Error>;
    async fn get_latest_prices_with_filter(&self, filter_base: Option<&str>, filter_quote: Option<&str>) -> Result<Vec<(String, String, f64)>, Error>;

    // Tradelog
    async fn add_price_log(&self, base_currency_id: i64, quote_currency_id: i64, price: f64) -> Result<i64, Error>;

    // Tax
    async fn get_tax_percentage(&self, currency_id: i64) -> Result<Option<f64>, Error>;
    async fn set_tax_percentage(&self, currency_id: i64, percentage: f64) -> Result<(), Error>;
    async fn get_tax_account(&self, currency_id: i64) -> Result<Option<i64>, Error>;
    async fn get_tax_account_with_guild(&self, currency_id: i64) -> Result<Option<(i64, i64)>, Error>;
    async fn get_total_tax_balance(&self, currency_id: i64) -> Result<Option<f64>, Error>;
    async fn add_tax(&self, currency_id: i64, amount: f64) -> Result<(), Error>;
    async fn collect_tax(&self, currency_id: i64, amount: f64) -> Result<f64, Error>;
    async fn create_tax_account(&self, currency_id: i64, tax_percentage: i32) -> Result<i64, Error>;

    // API
    async fn get_api_token(&self, currency_id: i64) -> Result<Option<String>, Error>;
    async fn set_api_token(&self, currency_id: i64, token: &str) -> Result<(), Error>;
    async fn get_api_token_by_type(&self, currency_id: i64, api_type_id: i32) -> Result<Option<String>, Error>;
    async fn store_api_token(&self, currency_id: i64, api_type_id: i32, encrypted_token: &str) -> Result<(), Error>;
}
