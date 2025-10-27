use serenity::model::channel::Message;
use serenity::prelude::Context;
use std::time::Instant;
use crate::services::ping_service;

pub async fn execute(ctx: &Context, msg: &Message) -> Result<(), String> {
    // Measure response latency from message send to response
    let start_time = Instant::now();
    
    // Send initial message to measure roundtrip
    let response = msg
        .channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default()
            .content("📊 Calculating metrics..."))
        .await
        .map_err(|e| e.to_string())?;
    
    // Get ping metrics from service
    let metrics = ping_service::get_ping_metrics(ctx, start_time).await?;
    
    // Create embed from service
    let embed = ping_service::create_ping_embed(&metrics);
    
    // Delete the initial message
    response.delete(ctx).await
        .map_err(|e| e.to_string())?;
    
    // Send the final embed
    msg.channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
