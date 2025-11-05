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
mod blacklist;
mod api;

struct Handler;

struct BotData;

impl TypeMapKey for BotData {
    type Value = Instant;
}

struct DatabasePool;

impl TypeMapKey for DatabasePool {
    type Value = MySqlPool;
}

struct CommandPrefix;

impl TypeMapKey for CommandPrefix {
    type Value = String;
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
    info!("  SMITE v1.1.2 - Society for Micronational Interbank Transactions and Exchanges");
    info!("");
    
    // Ensure encryption key exists
    if let Err(e) = utils::ensure_encryption_key() {
        warn!("Failed to ensure encryption key: {}", e);
    }
    
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
    let prefix = std::env::var("PREFIX").unwrap_or_else(|_| "$".to_string());
    
    info!("Using command prefix: '{}'", prefix);
    
    let intents = GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGES;

    // Enable autosharding - Discord will automatically shard the bot based on guild count
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Failed to create client");

    // Store the start time, database pool, and prefix in client data
    {
        let mut data = client.data.write().await;
        data.insert::<BotData>(Instant::now());
        data.insert::<DatabasePool>(pool);
        data.insert::<CommandPrefix>(prefix);
    }

    // Start the client with autosharding enabled
    if let Err(e) = client.start_autosharded().await {
        error!("Client error: {}", e);
    }
}

