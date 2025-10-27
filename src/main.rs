use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::time::Instant;
use sqlx::mysql::MySqlPool;

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
        println!("{} is connected!", ready.user.name);
        
        // Check for rate limits now that bot is connected
        println!("\n🔍 Checking Discord rate limit status...");
        match ctx.http.get_current_user().await {
            Ok(_) => {
                println!("✓ No rate limit detected - Bot is fully ready!");
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("429") || error_msg.contains("rate limit") || error_msg.contains("Ratelimited") {
                    eprintln!("⚠️  WARNING: Bot is being rate limited by Discord!");
                    eprintln!("⚠️  Error: {}", error_msg);
                } else {
                    eprintln!("⚠️  Failed to check rate limit status: {}", error_msg);
                }
            }
        }
        println!();
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    
    // Initialize database
    println!("Initializing database...");
    let pool = match db::init_db().await {
        Ok(p) => {
            println!("✓ Database initialized successfully");
            p
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize database: {}", e);
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
        eprintln!("Client error: {}", e);
    }
}

