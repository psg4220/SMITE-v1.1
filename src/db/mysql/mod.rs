use sqlx::mysql::MySqlPool;
use crate::db::traits::DatabaseBackend;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

pub mod account;
pub mod currency;
pub mod swap;
pub mod transaction;
pub mod tradelog;
pub mod tax;
pub mod api;

pub struct MySqlBackend {
    pool: MySqlPool,
}

impl MySqlBackend {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DatabaseBackend for MySqlBackend {
    // Account
    async fn create_account(&self, discord_id: i64, currency_id: i64) -> Result<i64, sqlx::Error> {
        account::create_account(&self.pool, discord_id, currency_id).await
    }
    async fn get_account_balance(&self, discord_id: i64, currency_id: i64) -> Result<Option<f64>, sqlx::Error> {
        account::get_account_balance(&self.pool, discord_id, currency_id).await
    }
    async fn get_account_id(&self, discord_id: i64, currency_id: i64) -> Result<Option<i64>, sqlx::Error> {
        account::get_account_id(&self.pool, discord_id, currency_id).await
    }
    async fn update_balance(&self, account_id: i64, new_balance: f64) -> Result<(), sqlx::Error> {
        account::update_balance(&self.pool, account_id, new_balance).await
    }
    async fn get_total_balance(&self, currency_id: i64) -> Result<Option<f64>, sqlx::Error> {
        account::get_total_balance(&self.pool, currency_id).await
    }
    async fn get_discord_id_by_account_id(&self, account_id: i64) -> Result<Option<i64>, sqlx::Error> {
        account::get_discord_id_by_account_id(&self.pool, account_id).await
    }
    async fn set_balance(&self, account_id: i64, balance: f64) -> Result<(), sqlx::Error> {
        account::set_balance(&self.pool, account_id, balance).await
    }
    async fn add_balance(&self, discord_id: i64, currency_id: i64, amount: f64) -> Result<(), sqlx::Error> {
        account::add_balance(&self.pool, discord_id, currency_id, amount).await
    }

    // Currency
    async fn create_currency(&self, guild_id: i64, name: &str, ticker: &str) -> Result<i64, sqlx::Error> {
        currency::create_currency(&self.pool, guild_id, name, ticker).await
    }
    async fn get_currency_by_id(&self, currency_id: i64) -> Result<Option<(i64, i64, String, String)>, sqlx::Error> {
        currency::get_currency_by_id(&self.pool, currency_id).await
    }
    async fn get_currency_by_ticker(&self, ticker: &str) -> Result<Option<(i64, String, String)>, sqlx::Error> {
        currency::get_currency_by_ticker(&self.pool, ticker).await
    }
    async fn get_currency_by_ticker_with_guild(&self, ticker: &str) -> Result<Option<(i64, i64, String, String)>, sqlx::Error> {
        currency::get_currency_by_ticker_with_guild(&self.pool, ticker).await
    }
    async fn get_currency_by_guild(&self, guild_id: i64) -> Result<Option<(i64, String, String)>, sqlx::Error> {
        currency::get_currency_by_guild(&self.pool, guild_id).await
    }
    async fn get_all_currencies(&self) -> Result<Vec<(i64, String, String)>, sqlx::Error> {
        currency::get_all_currencies(&self.pool).await
    }
    async fn get_currencies_paginated(&self, sort_by: &str, page: usize, page_size: usize) -> Result<(Vec<(i64, String, String)>, i64), sqlx::Error> {
        currency::get_currencies_paginated(&self.pool, sort_by, page, page_size).await
    }
    async fn get_currency_date(&self, currency_id: i64) -> Result<Option<String>, sqlx::Error> {
        currency::get_currency_date(&self.pool, currency_id).await
    }

    // Swap
    async fn create_swap(&self, maker_id: i64, maker_amount: f64, maker_currency_id: i64, taker_id: Option<i64>, taker_amount: Option<f64>, taker_currency_id: Option<i64>) -> Result<i64, sqlx::Error> {
        if let Some(tid) = taker_id {
            let t_amount = taker_amount.unwrap_or(0.0);
            let t_currency_id = taker_currency_id.unwrap_or(0);
            swap::create_swap(&self.pool, maker_id, maker_currency_id, t_currency_id, maker_amount, t_amount, tid).await
        } else {
            let t_amount = taker_amount.unwrap_or(0.0);
            let t_currency_id = taker_currency_id.unwrap_or(0);
            swap::create_swap_open(&self.pool, maker_id, maker_currency_id, t_currency_id, maker_amount, t_amount).await
        }
    }
    async fn get_swap(&self, swap_id: i64) -> Result<Option<(i64, i64, Option<i64>, f64, i64, Option<f64>, Option<i64>, String, String)>, sqlx::Error> {
        let result = swap::get_swap(&self.pool, swap_id).await?;
        Ok(result.map(|(id, maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status)| {
            (id, maker_id, taker_id, maker_amount, maker_currency_id, Some(taker_amount), Some(taker_currency_id), status, String::new())
        }))
    }
    async fn accept_swap(&self, swap_id: i64, taker_id: i64, uuid1: &str, uuid2: &str) -> Result<(), sqlx::Error> {
        swap::accept_swap(&self.pool, swap_id, taker_id, uuid1, uuid2).await
    }
    async fn deny_swap(&self, swap_id: i64) -> Result<(), sqlx::Error> {
        swap::cancel_swap(&self.pool, swap_id).await
    }
    async fn cancel_swap(&self, swap_id: i64) -> Result<(), sqlx::Error> {
        swap::cancel_swap(&self.pool, swap_id).await
    }
    async fn get_pending_swaps_by_maker(&self, maker_id: i64) -> Result<Vec<(i64, i64, Option<i64>, f64, i64, Option<f64>, Option<i64>, String, String)>, sqlx::Error> {
        let result = swap::get_pending_swaps_by_maker(&self.pool, maker_id).await?;
        Ok(result.into_iter().map(|(id, maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status)| {
            (id, maker_id, taker_id, maker_amount, maker_currency_id, Some(taker_amount), Some(taker_currency_id), status, String::new())
        }).collect())
    }
    async fn get_swaps_by_maker(&self, maker_id: i64) -> Result<Vec<(i64, i64, Option<i64>, f64, i64, Option<f64>, Option<i64>, String, String)>, sqlx::Error> {
        let result = swap::get_swaps_by_maker(&self.pool, maker_id).await?;
        Ok(result.into_iter().map(|(id, maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status)| {
            (id, maker_id, taker_id, maker_amount, maker_currency_id, Some(taker_amount), Some(taker_currency_id), status, String::new())
        }).collect())
    }
    async fn get_swaps_by_taker(&self, taker_id: i64) -> Result<Vec<(i64, i64, Option<i64>, f64, i64, Option<f64>, Option<i64>, String, String)>, sqlx::Error> {
        let result = swap::get_swaps_by_taker(&self.pool, taker_id).await?;
        Ok(result.into_iter().map(|(id, maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status)| {
            (id, maker_id, taker_id, maker_amount, maker_currency_id, Some(taker_amount), Some(taker_currency_id), status, String::new())
        }).collect())
    }
    async fn get_all_pending_swaps(&self) -> Result<Vec<(i64, i64, Option<i64>, f64, i64, Option<f64>, Option<i64>, String, String)>, sqlx::Error> {
        let result = swap::get_all_pending_swaps(&self.pool).await?;
        Ok(result.into_iter().map(|(id, maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status)| {
            (id, maker_id, taker_id, maker_amount, maker_currency_id, Some(taker_amount), Some(taker_currency_id), status, String::new())
        }).collect())
    }
    async fn get_all_open_swaps(&self) -> Result<Vec<(i64, i64, Option<i64>, f64, i64, Option<f64>, Option<i64>, String, String)>, sqlx::Error> {
        let result = swap::get_all_open_swaps(&self.pool).await?;
        Ok(result.into_iter().map(|(id, maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status)| {
            (id, maker_id, taker_id, maker_amount, maker_currency_id, Some(taker_amount), Some(taker_currency_id), status, String::new())
        }).collect())
    }
    async fn get_swap_message(&self, swap_id: i64) -> Result<Option<(i64, i64)>, sqlx::Error> {
        let result = swap::get_swap_message(&self.pool, swap_id).await?;
        Ok(result.map(|(channel_id, message_id, _)| (channel_id, message_id)))
    }
    async fn store_swap_message(&self, swap_id: i64, channel_id: i64, message_id: i64) -> Result<(), sqlx::Error> {
        swap::store_swap_message(&self.pool, swap_id, channel_id, message_id).await
    }
    async fn get_total_swap_maker_amount(&self, currency_id: i64) -> Result<Option<f64>, sqlx::Error> {
        swap::get_total_swap_maker_amount(&self.pool, currency_id).await
    }
    async fn create_swap_targeted(&self, maker_id: i64, maker_currency_id: i64, taker_currency_id: i64, maker_amount: f64, taker_amount: f64, taker_id: i64) -> Result<i64, sqlx::Error> {
        swap::create_swap(&self.pool, maker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, taker_id).await
    }
    async fn create_swap_open(&self, maker_id: i64, maker_currency_id: i64, taker_currency_id: i64, maker_amount: f64, taker_amount: f64) -> Result<i64, sqlx::Error> {
        swap::create_swap_open(&self.pool, maker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount).await
    }
    async fn get_swap_by_id(&self, swap_id: i64) -> Result<Option<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
        swap::get_swap_by_id(&self.pool, swap_id).await
    }
    async fn get_swaps_paginated(&self, page: usize, page_size: usize, sort_by: &str, status_filter: &str, base_currency: Option<&str>, quote_currency: Option<&str>) -> Result<(Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String, String, String)>, i64), sqlx::Error> {
        swap::get_swaps_paginated(&self.pool, page, page_size, sort_by, status_filter, base_currency, quote_currency).await
    }

    // Transaction
    async fn create_transaction(&self, uuid: &str, sender_account_id: i64, receiver_account_id: i64, amount: f64) -> Result<(), sqlx::Error> {
        transaction::create_transaction(&self.pool, uuid, sender_account_id, receiver_account_id, amount).await
    }
    async fn get_transaction(&self, uuid: &str) -> Result<Option<(i64, i64, f64, String, String)>, sqlx::Error> {
        let result = transaction::get_transaction(&self.pool, uuid).await?;
        Ok(result.map(|(uuid, sender_id, receiver_id, amount)| (sender_id, receiver_id, amount, String::new(), uuid)))
    }
    async fn get_transactions_by_sender(&self, sender_account_id: i64, limit: i32) -> Result<Vec<(i64, i64, f64, String, String)>, sqlx::Error> {
        let result = transaction::get_transactions_by_sender(&self.pool, sender_account_id, limit).await?;
        Ok(result.into_iter().map(|(uuid, sender_id, receiver_id, amount)| (sender_id, receiver_id, amount, String::new(), uuid)).collect())
    }
    async fn get_transactions_by_receiver(&self, receiver_account_id: i64, limit: i32) -> Result<Vec<(i64, i64, f64, String, String)>, sqlx::Error> {
        let result = transaction::get_transactions_by_receiver(&self.pool, receiver_account_id, limit).await?;
        Ok(result.into_iter().map(|(uuid, sender_id, receiver_id, amount)| (sender_id, receiver_id, amount, String::new(), uuid)).collect())
    }
    async fn get_user_transactions(&self, user_discord_id: i64, limit: i32) -> Result<Vec<(i64, i64, f64, String, String, String)>, sqlx::Error> {
        transaction::get_user_transactions(&self.pool, user_discord_id, limit as u32).await
    }
    async fn get_user_transactions_paginated(&self, user_discord_id: i64, page: usize, page_size: usize) -> Result<(Vec<(i64, i64, f64, String, String, String)>, i64), sqlx::Error> {
        transaction::get_user_transactions_paginated(&self.pool, user_discord_id, page, page_size).await
    }

    // Tradelog
    async fn log_price(&self, base_currency_id: i64, quote_currency_id: i64, price: f64) -> Result<(), sqlx::Error> {
        tradelog::add_price_log(&self.pool, base_currency_id, quote_currency_id, price).await?;
        Ok(())
    }
    async fn add_price_log(&self, base_currency_id: i64, quote_currency_id: i64, price: f64) -> Result<i64, sqlx::Error> {
        tradelog::add_price_log(&self.pool, base_currency_id, quote_currency_id, price).await
    }
    async fn get_latest_price(&self, base_currency_id: i64, quote_currency_id: i64) -> Result<Option<f64>, sqlx::Error> {
        let result = tradelog::get_latest_price_for_pair(&self.pool, base_currency_id, quote_currency_id).await?;
        Ok(result.map(|(price, _)| price))
    }
    async fn get_latest_price_bidirectional(&self, base_currency_id: i64, quote_currency_id: i64) -> Result<Option<(f64, bool)>, sqlx::Error> {
        tradelog::get_latest_price_bidirectional(&self.pool, base_currency_id, quote_currency_id).await
    }
    async fn get_price_logs_for_pair(&self, base_currency_id: i64, quote_currency_id: i64, limit: i32) -> Result<Vec<(i64, f64, String)>, sqlx::Error> {
        let logs = tradelog::get_price_logs_for_pair(&self.pool, base_currency_id, quote_currency_id).await?;
        // Convert (i64, i64, i64, f64) to (i64, f64, String)
        // We don't have the date string here, so we return empty string for now
        Ok(logs.into_iter().map(|(id, _, _, price)| (id, price, String::new())).collect())
    }
    async fn get_price_logs_with_timestamps(&self, base_currency_id: i64, quote_currency_id: i64) -> Result<Vec<(i64, f64, String)>, sqlx::Error> {
        tradelog::get_price_logs_with_timestamps(&self.pool, base_currency_id, quote_currency_id).await
    }
    async fn get_price_logs_in_range(&self, base_currency_id: i64, quote_currency_id: i64, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<(i64, f64, String)>, sqlx::Error> {
        let start_str = start_date.format("%Y-%m-%d %H:%M:%S").to_string();
        let end_str = end_date.format("%Y-%m-%d %H:%M:%S").to_string();
        tradelog::get_price_logs_in_range(&self.pool, base_currency_id, quote_currency_id, &start_str, &end_str).await
    }
    async fn normalize_pair(&self, base_currency_id: i64, quote_currency_id: i64) -> Result<(i64, i64, bool), sqlx::Error> {
        tradelog::normalize_pair(&self.pool, base_currency_id, quote_currency_id).await
    }
    async fn calculate_vwap(&self, base_currency_id: i64, quote_currency_id: i64, timeframe: &str) -> Result<Option<f64>, sqlx::Error> {
        tradelog::calculate_vwap(&self.pool, base_currency_id, quote_currency_id, timeframe).await
    }
    async fn get_latest_prices_with_filter(&self, filter_base: Option<&str>, filter_quote: Option<&str>) -> Result<Vec<(String, String, f64)>, sqlx::Error> {
        tradelog::get_latest_prices_with_filter(&self.pool, filter_base, filter_quote).await
    }

    // Tax
    async fn get_tax_percentage(&self, currency_id: i64) -> Result<Option<f64>, sqlx::Error> {
        let result = tax::get_tax_percentage(&self.pool, currency_id).await?;
        Ok(result.map(|p| p as f64))
    }
    async fn set_tax_percentage(&self, currency_id: i64, percentage: f64) -> Result<(), sqlx::Error> {
        tax::set_tax_percentage(&self.pool, currency_id, percentage as i32).await
    }
    async fn get_tax_account(&self, currency_id: i64) -> Result<Option<i64>, sqlx::Error> {
        let result = tax::get_tax_account(&self.pool, currency_id).await?;
        Ok(result.map(|(id, _, _, _)| id))
    }
    async fn get_tax_account_with_guild(&self, currency_id: i64) -> Result<Option<(i64, i64)>, sqlx::Error> {
        let result = tax::get_tax_account_with_guild(&self.pool, currency_id).await?;
        Ok(result.map(|(id, guild_id, _, _, _)| (id, guild_id)))
    }
    async fn get_total_tax_balance(&self, currency_id: i64) -> Result<Option<f64>, sqlx::Error> {
        tax::get_total_tax_balance(&self.pool, currency_id).await
    }
    async fn add_tax(&self, currency_id: i64, amount: f64) -> Result<(), sqlx::Error> {
        tax::add_tax(&self.pool, currency_id, amount).await
    }
    async fn collect_tax(&self, currency_id: i64, amount: f64) -> Result<f64, sqlx::Error> {
        tax::collect_tax(&self.pool, currency_id, amount).await
    }
    async fn create_tax_account(&self, currency_id: i64, tax_percentage: i32) -> Result<i64, sqlx::Error> {
        tax::create_tax_account(&self.pool, currency_id, tax_percentage).await
    }

    // API
    async fn get_api_token(&self, currency_id: i64) -> Result<Option<String>, sqlx::Error> {
        api::get_api_token(&self.pool, currency_id, 1).await
    }
    async fn set_api_token(&self, currency_id: i64, token: &str) -> Result<(), sqlx::Error> {
        api::store_api_token(&self.pool, currency_id, 1, token).await
    }
    async fn get_api_token_by_type(&self, currency_id: i64, api_type_id: i32) -> Result<Option<String>, sqlx::Error> {
        api::get_api_token(&self.pool, currency_id, api_type_id).await
    }
    async fn store_api_token(&self, currency_id: i64, api_type_id: i32, encrypted_token: &str) -> Result<(), sqlx::Error> {
        api::store_api_token(&self.pool, currency_id, api_type_id, encrypted_token).await
    }
}
