//! Ping/status command models

/// Bot ping metrics and uptime information
#[derive(Debug)]
pub struct PingMetrics {
    pub response_latency: u64,
    pub uptime: String,
    pub shard_id: String,
}
