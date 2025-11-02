use sqlx::mysql::MySqlPool;
use tracing::{info, warn, debug};

pub mod currency;
pub mod account;
pub mod swap;
pub mod transaction;
pub mod tradelog;
pub mod tax;

/// Initialize the MySQL connection pool and create tables
pub async fn init_db() -> Result<MySqlPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL not set in .env file");

    let pool = MySqlPool::connect(&database_url).await?;

    // Create all tables
    create_tables(&pool).await?;

    Ok(pool)
}

/// Read and execute SQL file for creating tables
async fn execute_sql_file(pool: &MySqlPool, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let sql_content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read {}: {}", file_path, e))?;

    // For create_tables.sql, split by semicolon
    for statement in sql_content.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("--") {
            match sqlx::raw_sql(trimmed).execute(pool).await {
                Ok(_) => {},
                Err(e) => {
                    // Log but don't fail - tables might already exist
                    debug!("Database schema note: {}", e);
                }
            }
        }
    }

    Ok(())
}

/// Create all database tables
async fn create_tables(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    info!("Initializing database schema...");
    
    // Create tables from SQL file
    if let Err(e) = execute_sql_file(pool, "migrations/create_tables.sql").await {
        warn!("Failed to create tables: {}", e);
    } else {
        info!("Tables initialized successfully");
    }

    Ok(())
}
