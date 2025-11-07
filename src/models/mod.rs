//! Data models for SMITE commands and services
//!
//! This module organizes all result and data transfer structs used across commands.
//! Each model represents the output/response from a service operation.

pub mod balance;
pub mod wire;
pub mod chart;
pub mod send;
pub mod swap;
pub mod mint;
pub mod price;
pub mod transaction;
pub mod create_currency;
pub mod currency;
pub mod ping;

// Re-export commonly used types for convenience
pub use balance::BalanceResult;
pub use wire::{WireResult, WireDirection};
pub use chart::PricePoint;
pub use send::SendResult;
pub use swap::{SwapResult, AcceptDenyResult, SwapListResult};
pub use mint::MintResult;
pub use price::PriceResult;
pub use transaction::{TransactionListResult, TransactionDetailResult};
pub use create_currency::CreateCurrencyResult;
pub use currency::CurrencyInfo;
pub use ping::PingMetrics;
