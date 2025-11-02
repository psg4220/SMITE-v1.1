use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::time::Instant;
use sqlx::mysql::MySqlPool;
use tracing::{info, warn, error, debug};
use tracing_subscriber::EnvFilter;

mod db;
mod commands;
mod services;
mod utils;                                                   

struct Handler;

struct BotData;

impl TypeMapKey for BotData {
    type Value = Instant;
}

struct DatabasePool;

impl TypeMapKey for DatabasePool {
    type Value = MySqlPool;
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        commands::handle_message(&ctx, &msg).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        
        // Check for rate limits now that bot is connected
        debug!("Checking Discord rate limit status...");
        match ctx.http.get_current_user().await {
            Ok(_) => {
                info!("No rate limit detected - Bot is fully ready!");
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("429") || error_msg.contains("rate limit") || error_msg.contains("Ratelimited") {
                    warn!("Bot is being rate limited by Discord! Error: {}", error_msg);
                } else {
                    warn!("Failed to check rate limit status: {}", error_msg);
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("smite_v1p1=debug".parse().unwrap())
            .add_directive("serenity=warn".parse().unwrap()))
        .with_target(true)
        .with_thread_ids(true)
        .init();
    
    info!("🤖 Starting SMITE bot...");
    info!("   ______   __       __  ______  ________  ________ ");
    info!("  /      \\ |  \\     /  \\|      \\|        \\|        \\");
    info!(" |  $$$$$$\\| $$\\   /  $$ \\$$$$$$ \\$$$$$$$$| $$$$$$$$");
    info!(" | $$___\\$$| $$$\\ /  $$$  | $$     | $$   | $$__    ");
    info!("  \\$$    \\ | $$$$\\  $$$$  | $$     | $$   | $$  \\   ");
    info!("  _\\$$$$$$\\| $$\\$$ $$ $$  | $$     | $$   | $$$$$   ");
    info!(" |  \\__| $$| $$ \\$$$| $$ _| $$_    | $$   | $$_____ ");
    info!("  \\$$    $$| $$  \\$ | $$|   $$ \\   | $$   | $$     \\");
    info!("   \\$$$$$$  \\$$      \\$$ \\$$$$$$    \\$$    \\$$$$$$$$");
    info!("  SMITE v1.1.0 - Society for Micronational Interbank Transactions and Exchanges");
    info!("");
    
    // Initialize database
    info!("Initializing database...");
    let pool = match db::init_db().await {
        Ok(p) => {
            info!("Database initialized successfully");
            p
        }
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return;
        }
    };
    
    let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");
    let intents = GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGES;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Failed to create client");

    // Store the start time and database pool in client data
    {
        let mut data = client.data.write().await;
        data.insert::<BotData>(Instant::now());
        data.insert::<DatabasePool>(pool);
    }

    if let Err(e) = client.start().await {
        error!("Client error: {}", e);
    }
}

