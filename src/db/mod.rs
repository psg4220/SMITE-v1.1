use std::sync::Arc;
use std::str::FromStr;
use std::time::Duration;
use sqlx::mysql::MySqlPool;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use tracing::{info, warn, debug};

pub mod traits;
pub mod mysql;
pub mod sqlite;


pub use traits::DatabaseBackend;
pub use mysql::MySqlBackend;
pub use sqlite::SqliteBackend;

/// Initialize the database connection pool and create tables
pub async fn init_db() -> Result<Arc<dyn DatabaseBackend>, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL not set in .env file");

    if database_url.starts_with("mysql://") {
        info!("Initializing MySQL backend...");
        let pool = MySqlPool::connect(&database_url).await?;
        
        // Create tables for MySQL
        // Note: We might want to move this logic to the backend implementation later
        create_tables_mysql(&pool).await?;
        
        // Initialize API types
        if let Err(e) = initialize_api_types_mysql(&pool).await {
            warn!("Failed to initialize API types: {}", e);
        }

        Ok(Arc::new(MySqlBackend::new(pool)) as Arc<dyn DatabaseBackend>)
        
    } else if database_url.starts_with("sqlite://") {
        info!("Initializing SQLite backend with WAL mode...");
        
        // Configure SQLite with WAL mode for better concurrency
        let connect_options = SqliteConnectOptions::from_str(&database_url)?
            .journal_mode(SqliteJournalMode::Wal)           // Enable WAL mode for concurrent reads + 1 writer
            .synchronous(SqliteSynchronous::Normal)         // Faster than FULL, still safe with WAL
            .busy_timeout(Duration::from_secs(30))          // Wait up to 30s for locks instead of failing immediately
            .pragma("cache_size", "-64000")                 // 64MB cache (negative = KB)
            .pragma("temp_store", "memory")                 // Store temp tables in RAM
            .create_if_missing(true);                       // Create database file if it doesn't exist
        
        let pool = SqlitePoolOptions::new()
            .max_connections(2)                            // Allow up to 2 concurrent connections
            .min_connections(1)                             // Keep at least 1 connection ready
            .acquire_timeout(Duration::from_secs(30))       // Timeout for acquiring a connection from pool
            .connect_with(connect_options)
            .await?;
        
        info!("SQLite WAL mode enabled with 10 max connections");
        
        // Create tables for SQLite
        create_tables_sqlite(&pool).await?;
        
        // Initialize API types
        if let Err(e) = initialize_api_types_sqlite(&pool).await {
            warn!("Failed to initialize API types: {}", e);
        }

        Ok(Arc::new(SqliteBackend::new(pool)) as Arc<dyn DatabaseBackend>)
    } else if database_url.starts_with("libsql://") {
        info!("Initializing LibSQL backend...");
        // TODO: Implement LibSQL backend
        Err("LibSQL backend not yet implemented. Use sqlite:// for now.".into())
    } else {
        Err(format!("Unsupported database URL scheme: {}", database_url).into())
    }
}

/// Read and execute SQL file for creating tables (MySQL)
async fn create_tables_mysql(pool: &MySqlPool) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = "migrations/create_tables.sql";
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

async fn initialize_api_types_mysql(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    // Check if api_token column exists in currency table
    let row: Option<(i64,)> = sqlx::query_as("SELECT count(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'currency' AND COLUMN_NAME = 'api_token'")
        .fetch_optional(pool)
        .await?;

    if let Some((count,)) = row {
        if count == 0 {
            info!("Adding api_token column to currency table");
            sqlx::query("ALTER TABLE currency ADD COLUMN api_token VARCHAR(255) NULL")
                .execute(pool)
                .await?;
        }
    }

    Ok(())
}

/// Read and execute SQL file for creating tables (SQLite)
async fn create_tables_sqlite(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = "migrations/create_tables_sqlite.sql";
    let sql_content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read {}: {}", file_path, e))?;

    // Remove SQL comments (lines starting with --)
    let lines: Vec<&str> = sql_content
        .lines()
        .filter(|line| !line.trim().starts_with("--"))
        .collect();
    let cleaned_sql = lines.join("\n");

    // Split by semicolon and execute each statement
    for statement in cleaned_sql.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() {
            debug!("Executing SQL: {}", &trimmed[..trimmed.len().min(80)]);
            match sqlx::raw_sql(trimmed).execute(pool).await {
                Ok(_) => {
                    debug!("SQL executed successfully");
                },
                Err(e) => {
                    // Only log as debug if it's a "table already exists" type error
                    let err_msg = e.to_string();
                    if err_msg.contains("already exists") {
                        debug!("Database schema note: {}", e);
                    } else {
                        // Log actual errors as warnings so we can see them
                        warn!("Database schema error: {}", e);
                    }
                }
            }
        }
    }

    info!("SQLite tables initialized");
    Ok(())
}

/// Initialize API types for SQLite (insert default types if they don't exist)
async fn initialize_api_types_sqlite(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Insert default API types if they don't exist
    sqlx::query("INSERT OR IGNORE INTO api_type (id, name) VALUES (1, 'unbelievaboat')")
        .execute(pool)
        .await?;

    Ok(())
}

