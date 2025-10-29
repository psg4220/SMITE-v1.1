use sqlx::mysql::MySqlPool;

pub mod currency;
pub mod account;
pub mod swap;
pub mod transaction;
pub mod tradelog;

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
                    eprintln!("⚠️  Note: {}", e);
                }
            }
        }
    }

    Ok(())
}

/// Create all database tables
async fn create_tables(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    println!("📋 Initializing database schema...");
    
    // Create tables from SQL file
    if let Err(e) = execute_sql_file(pool, "migrations/create_tables.sql").await {
        eprintln!("⚠️  Warning: Failed to create tables: {}", e);
    } else {
        println!("✓ Tables initialized successfully");
    }

    // Load stored procedures using shell script
    println!("🔧 Loading stored procedures...");
    match std::process::Command::new("bash")
        .arg("load_procedures.sh")
        .current_dir(std::env::current_dir().unwrap_or_default())
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                println!("✓ All stored procedures loaded successfully");
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                eprintln!("❌ CRITICAL: Failed to load stored procedures");
                eprintln!("stdout: {}", stdout);
                eprintln!("stderr: {}", stderr);
                eprintln!("❌ The bot cannot start without valid procedures.");
                eprintln!("❌ Please check that load_procedures.sh can execute and procedures.sql is valid.");
                Err(sqlx::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Procedure loading script failed"
                )))
            }
        }
        Err(e) => {
            eprintln!("❌ CRITICAL: Failed to execute load_procedures.sh: {}", e);
            eprintln!("❌ The bot cannot start without valid procedures.");
            eprintln!("❌ Please ensure load_procedures.sh exists and is executable.");
            Err(sqlx::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Failed to run procedure loader: {}", e)
            )))
        }
    }
}
