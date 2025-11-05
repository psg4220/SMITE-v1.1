use serenity::prelude::*;
use std::time::Instant;

pub struct PingMetrics {
    pub response_latency: u64,
    pub uptime: String,
    pub shard_id: String,
}

pub async fn get_ping_metrics(ctx: &Context, start_time: Instant) -> Result<PingMetrics, String> {
    let response_latency = start_time.elapsed().as_millis() as u64;
    
    // Get shard information
    let shard_id = ctx.shard_id.0.to_string();
    
    // Get bot uptime from client data
    let uptime = {
        let data = ctx.data.read().await;
        if let Some(&bot_start_time) = data.get::<crate::BotData>() {
            let elapsed = bot_start_time.elapsed();
            let hours = elapsed.as_secs() / 3600;
            let minutes = (elapsed.as_secs() % 3600) / 60;
            let seconds = elapsed.as_secs() % 60;
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else {
            "Unknown".to_string()
        }
    };

    Ok(PingMetrics {
        response_latency,
        uptime,
        shard_id,
    })
}

pub fn create_ping_embed(metrics: &PingMetrics) -> serenity::builder::CreateEmbed {
    serenity::builder::CreateEmbed::default()
        .title("Pong! 🏓")
        .field("Response Latency", format!("{}ms", metrics.response_latency), true)
        .field("Shard ID", &metrics.shard_id, true)
        .field("Uptime", &metrics.uptime, false)
        .color(0x00b0f4)
}
