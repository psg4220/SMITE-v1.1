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

    if file_path.contains("procedures.sql") {
        // For procedures, split by "DELIMITER ;" boundary and handle the DELIMITER // structure
        let parts: Vec<&str> = sql_content.split("DELIMITER ;").collect();
        let mut procedure_errors = Vec::new();
        
        for part in parts {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            
            // Remove "DELIMITER //" prefix if present
            let sql = if part.starts_with("DELIMITER //") {
                part[12..].trim() // Skip "DELIMITER //"
            } else {
                part
            };
            
            // Replace "//" with ";" for MySQL execution
            let executable = sql.replace("//", ";");
            
            if !executable.is_empty() && !executable.trim().starts_with("--") {
                match sqlx::raw_sql(&executable).execute(pool).await {
                    Ok(_) => {
                        // Extract procedure name for logging
                        if let Some(proc_name) = executable.lines()
                            .find(|line| line.contains("PROCEDURE"))
                            .and_then(|line| line.split_whitespace().nth(3)) {
                            println!("✓ Procedure {} loaded", proc_name);
                        }
                    },
                    Err(e) => {
                        let error_msg = format!("Failed to execute procedure: {}", e);
                        eprintln!("❌ {}", error_msg);
                        procedure_errors.push(error_msg);
                    }
                }
            }
        }
        
        // If any procedures failed, return error to stop bot startup
        if !procedure_errors.is_empty() {
            return Err(format!("❌ {} procedure(s) failed to load. Bot startup aborted.", procedure_errors.len()).into());
        }
    } else {
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

    // Create stored procedures from SQL file - THIS MUST SUCCEED
    println!("🔧 Loading stored procedures...");
    match execute_sql_file(pool, "migrations/procedures.sql").await {
        Ok(_) => {
            println!("✓ All stored procedures loaded successfully");
            Ok(())
        },
        Err(e) => {
            eprintln!("❌ CRITICAL: Failed to load stored procedures: {}", e);
            eprintln!("❌ The bot cannot start without valid procedures.");
            eprintln!("❌ Please check that procedures.sql is valid and exists.");
            Err(sqlx::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Procedure loading failed: {}", e)
            )))
        }
    }
}
