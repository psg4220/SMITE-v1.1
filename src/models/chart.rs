//! Chart generation models

use chrono::{DateTime, Utc};

/// A single data point on a price chart
#[derive(Debug, Clone)]
pub struct PricePoint {
    pub timestamp: DateTime<Utc>,
    pub price: f64,
}
