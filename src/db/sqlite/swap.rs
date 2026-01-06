use sqlx::sqlite::SqlitePool;
use sqlx::Row;

/// Get a swap by ID (direct query)
/// Returns: (id, maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status)
pub async fn get_swap_by_id(
    pool: &SqlitePool,
    swap_id: i64,
) -> Result<Option<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "SELECT CAST(id AS INTEGER), CAST(maker_id AS INTEGER), CAST(taker_id AS INTEGER), CAST(maker_currency_id AS INTEGER), 
                CAST(taker_currency_id AS INTEGER), CAST(maker_amount AS REAL), CAST(taker_amount AS REAL), status 
         FROM currency_swap WHERE id = ?"
    )
    .bind(swap_id)
    .fetch_optional(pool)
    .await
}

/// Get all pending swaps for a maker (direct query)
pub async fn get_pending_swaps_for_maker(
    pool: &SqlitePool,
    maker_account_id: i64,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "SELECT CAST(id AS INTEGER), CAST(maker_id AS INTEGER), CAST(taker_id AS INTEGER), CAST(maker_currency_id AS INTEGER), 
                CAST(taker_currency_id AS INTEGER), CAST(maker_amount AS REAL), CAST(taker_amount AS REAL), status 
         FROM currency_swap WHERE maker_id = ? AND status = 'pending'"
    )
    .bind(maker_account_id)
    .fetch_all(pool)
    .await
}

/// Get all pending swaps for a taker (direct query)
pub async fn get_pending_swaps_for_taker(
    pool: &SqlitePool,
    taker_account_id: i64,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "SELECT CAST(id AS INTEGER), CAST(maker_id AS INTEGER), CAST(taker_id AS INTEGER), CAST(maker_currency_id AS INTEGER), 
                CAST(taker_currency_id AS INTEGER), CAST(maker_amount AS REAL), CAST(taker_amount AS REAL), status 
         FROM currency_swap WHERE taker_id = ? AND status = 'pending'"
    )
    .bind(taker_account_id)
    .fetch_all(pool)
    .await
}

/// Get all open swaps (swaps where taker_id is NULL) - direct query
pub async fn get_open_swaps(
    pool: &SqlitePool,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "SELECT CAST(id AS INTEGER), CAST(maker_id AS INTEGER), CAST(taker_id AS INTEGER), CAST(maker_currency_id AS INTEGER), 
                CAST(taker_currency_id AS INTEGER), CAST(maker_amount AS REAL), CAST(taker_amount AS REAL), status 
         FROM currency_swap WHERE taker_id IS NULL AND status = 'pending'"
    )
    .fetch_all(pool)
    .await
}

/// Create a new currency swap (targeted swap)
/// Implements sp_create_swap logic: checks balance, deducts maker's balance, creates swap record
pub async fn create_swap(
    pool: &SqlitePool,
    maker_id: i64,
    maker_currency_id: i64,
    taker_currency_id: i64,
    maker_amount: f64,
    taker_amount: f64,
    taker_id: i64,
) -> Result<i64, sqlx::Error> {
    // Start a transaction
    let mut tx = pool.begin().await?;

    // Check if maker has sufficient balance
    let balance_row = sqlx::query("SELECT CAST(balance AS REAL) as balance FROM account WHERE id = ?")
        .bind(maker_id)
        .fetch_optional(&mut *tx)
        .await?;

    let balance: f64 = balance_row
        .map(|r| r.get::<f64, _>("balance"))
        .unwrap_or(0.0);

    if balance < maker_amount {
        tx.rollback().await?;
        return Err(sqlx::Error::Protocol("Maker has insufficient balance".into()));
    }

    // Deduct maker's balance
    sqlx::query("UPDATE account SET balance = balance - ?, date_updated = datetime('now') WHERE id = ?")
        .bind(maker_amount)
        .bind(maker_id)
        .execute(&mut *tx)
        .await?;

    // Insert swap record with taker_id for targeted swaps
    let result = sqlx::query(
        "INSERT INTO currency_swap (maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status) 
         VALUES (?, ?, ?, ?, ?, ?, 'pending')"
    )
    .bind(maker_id)
    .bind(taker_id)
    .bind(maker_currency_id)
    .bind(taker_currency_id)
    .bind(maker_amount)
    .bind(taker_amount)
    .execute(&mut *tx)
    .await?;

    let swap_id = result.last_insert_rowid();

    tx.commit().await?;

    Ok(swap_id)
}

/// Create an open currency swap (any user can accept)
/// Implements sp_create_swap_open logic: checks balance, deducts maker's balance, creates open swap
pub async fn create_swap_open(
    pool: &SqlitePool,
    maker_id: i64,
    maker_currency_id: i64,
    taker_currency_id: i64,
    maker_amount: f64,
    taker_amount: f64,
) -> Result<i64, sqlx::Error> {
    // Start a transaction
    let mut tx = pool.begin().await?;

    // Check if maker has sufficient balance
    let balance_row = sqlx::query("SELECT CAST(balance AS REAL) as balance FROM account WHERE id = ?")
        .bind(maker_id)
        .fetch_optional(&mut *tx)
        .await?;

    let balance: f64 = balance_row
        .map(|r| r.get::<f64, _>("balance"))
        .unwrap_or(0.0);

    if balance < maker_amount {
        tx.rollback().await?;
        return Err(sqlx::Error::Protocol("Maker has insufficient balance".into()));
    }

    // Deduct maker's balance
    sqlx::query("UPDATE account SET balance = balance - ?, date_updated = datetime('now') WHERE id = ?")
        .bind(maker_amount)
        .bind(maker_id)
        .execute(&mut *tx)
        .await?;

    // Insert open swap record (taker_id is NULL, anyone can accept)
    let result = sqlx::query(
        "INSERT INTO currency_swap (maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status) 
         VALUES (?, NULL, ?, ?, ?, ?, 'pending')"
    )
    .bind(maker_id)
    .bind(maker_currency_id)
    .bind(taker_currency_id)
    .bind(maker_amount)
    .bind(taker_amount)
    .execute(&mut *tx)
    .await?;

    let swap_id = result.last_insert_rowid();

    tx.commit().await?;

    Ok(swap_id)
}

/// Accept a swap as the taker
/// Implements sp_accept_swap logic: validates swap, checks balances, transfers funds, logs transactions
pub async fn accept_swap(
    pool: &SqlitePool,
    swap_id: i64,
    user_discord_id: i64,
    uuid1: &str,
    uuid2: &str,
) -> Result<(), sqlx::Error> {
    // Start a transaction
    let mut tx = pool.begin().await?;

    // Get swap details
    let swap_row = sqlx::query(
        "SELECT maker_id, taker_id, maker_currency_id, taker_currency_id, 
                CAST(maker_amount AS REAL) as maker_amount, CAST(taker_amount AS REAL) as taker_amount, status 
         FROM currency_swap WHERE id = ?"
    )
    .bind(swap_id)
    .fetch_optional(&mut *tx)
    .await?;

    let swap = match swap_row {
        Some(row) => row,
        None => {
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol("Swap not found".into()));
        }
    };

    let status: String = swap.get("status");
    if status != "pending" {
        tx.rollback().await?;
        return Err(sqlx::Error::Protocol("Swap is not pending".into()));
    }

    let maker_account_id: i64 = swap.get("maker_id");
    let taker_account_id: Option<i64> = swap.get("taker_id");
    let maker_currency_id: i64 = swap.get("maker_currency_id");
    let taker_currency_id: i64 = swap.get("taker_currency_id");
    let maker_amount: f64 = swap.get("maker_amount");
    let taker_amount: f64 = swap.get("taker_amount");

    // Get maker's Discord ID
    let maker_discord_row = sqlx::query("SELECT discord_id FROM account WHERE id = ?")
        .bind(maker_account_id)
        .fetch_optional(&mut *tx)
        .await?;

    let maker_discord_id: i64 = match maker_discord_row {
        Some(row) => row.get("discord_id"),
        None => {
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol("Maker account not found".into()));
        }
    };

    // Get taker's Discord ID (if targeted swap)
    let taker_discord_id: Option<i64> = if let Some(tid) = taker_account_id {
        let taker_row = sqlx::query("SELECT discord_id FROM account WHERE id = ?")
            .bind(tid)
            .fetch_optional(&mut *tx)
            .await?;
        match taker_row {
            Some(row) => Some(row.get("discord_id")),
            None => {
                tx.rollback().await?;
                return Err(sqlx::Error::Protocol("Taker Discord ID not found".into()));
            }
        }
    } else {
        None // Open swap, no designated taker
    };

    // Authorization check
    if let Some(designated_taker) = taker_discord_id {
        // Targeted swap: only designated taker can accept
        if user_discord_id != designated_taker {
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol("Not authorized to accept this swap".into()));
        }
    } else {
        // Open swap: maker cannot accept their own swap
        if user_discord_id == maker_discord_id {
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol("Cannot accept your own open swap".into()));
        }
    }

    // Get or create accepting user's account for taker currency (the currency they will give)
    let user_taker_account_row = sqlx::query("SELECT id FROM account WHERE discord_id = ? AND currency_id = ?")
        .bind(user_discord_id)
        .bind(taker_currency_id)
        .fetch_optional(&mut *tx)
        .await?;

    let user_taker_account_id: i64 = match user_taker_account_row {
        Some(row) => row.get("id"),
        None => {
            // Create account
            let result = sqlx::query("INSERT INTO account (discord_id, currency_id, balance) VALUES (?, ?, 0)")
                .bind(user_discord_id)
                .bind(taker_currency_id)
                .execute(&mut *tx)
                .await?;
            result.last_insert_rowid()
        }
    };

    // Get or create accepting user's account for maker currency (the currency they will receive)
    let user_maker_account_row = sqlx::query("SELECT id FROM account WHERE discord_id = ? AND currency_id = ?")
        .bind(user_discord_id)
        .bind(maker_currency_id)
        .fetch_optional(&mut *tx)
        .await?;

    let user_maker_account_id: i64 = match user_maker_account_row {
        Some(row) => row.get("id"),
        None => {
            // Create account
            let result = sqlx::query("INSERT INTO account (discord_id, currency_id, balance) VALUES (?, ?, 0)")
                .bind(user_discord_id)
                .bind(maker_currency_id)
                .execute(&mut *tx)
                .await?;
            result.last_insert_rowid()
        }
    };

    // Check accepting user has sufficient balance for their currency
    let balance_row = sqlx::query("SELECT CAST(balance AS REAL) as balance FROM account WHERE id = ?")
        .bind(user_taker_account_id)
        .fetch_optional(&mut *tx)
        .await?;

    let taker_balance: f64 = balance_row
        .map(|r| r.get::<f64, _>("balance"))
        .unwrap_or(0.0);

    if taker_balance < taker_amount {
        tx.rollback().await?;
        return Err(sqlx::Error::Protocol("Insufficient balance to accept swap".into()));
    }

    // Deduct accepting user's balance (they give their currency)
    sqlx::query("UPDATE account SET balance = balance - ?, date_updated = datetime('now') WHERE id = ?")
        .bind(taker_amount)
        .bind(user_taker_account_id)
        .execute(&mut *tx)
        .await?;

    // Credit accepting user with maker's currency
    sqlx::query("UPDATE account SET balance = balance + ?, date_updated = datetime('now') WHERE id = ?")
        .bind(maker_amount)
        .bind(user_maker_account_id)
        .execute(&mut *tx)
        .await?;

    // Get or create maker's account for taker currency (the currency they will receive)
    let maker_taker_account_row = sqlx::query("SELECT id FROM account WHERE discord_id = ? AND currency_id = ?")
        .bind(maker_discord_id)
        .bind(taker_currency_id)
        .fetch_optional(&mut *tx)
        .await?;

    let maker_taker_account_id: i64 = match maker_taker_account_row {
        Some(row) => row.get("id"),
        None => {
            // Create account
            let result = sqlx::query("INSERT INTO account (discord_id, currency_id, balance) VALUES (?, ?, 0)")
                .bind(maker_discord_id)
                .bind(taker_currency_id)
                .execute(&mut *tx)
                .await?;
            result.last_insert_rowid()
        }
    };

    // Credit maker with accepting user's currency
    sqlx::query("UPDATE account SET balance = balance + ?, date_updated = datetime('now') WHERE id = ?")
        .bind(taker_amount)
        .bind(maker_taker_account_id)
        .execute(&mut *tx)
        .await?;

    // Log transactions (2 total) using provided UUIDs to ensure uniqueness
    // Transaction 1: Accepting user sends their currency to maker
    sqlx::query("INSERT INTO \"transaction\" (uuid, sender_id, receiver_id, amount) VALUES (?, ?, ?, ?)")
        .bind(uuid1)
        .bind(user_taker_account_id)
        .bind(maker_taker_account_id)
        .bind(taker_amount)
        .execute(&mut *tx)
        .await?;

    // Transaction 2: Maker sends their currency to accepting user
    sqlx::query("INSERT INTO \"transaction\" (uuid, sender_id, receiver_id, amount) VALUES (?, ?, ?, ?)")
        .bind(uuid2)
        .bind(maker_account_id)
        .bind(user_maker_account_id)
        .bind(maker_amount)
        .execute(&mut *tx)
        .await?;

    // Update swap status to accepted
    sqlx::query("UPDATE currency_swap SET status = 'accepted', date_updated = datetime('now') WHERE id = ?")
        .bind(swap_id)
        .execute(&mut *tx)
        .await?;

    // For open swaps, set the taker_id to the accepting user's account
    if taker_account_id.is_none() {
        sqlx::query("UPDATE currency_swap SET taker_id = ?, date_updated = datetime('now') WHERE id = ?")
            .bind(user_taker_account_id)
            .bind(swap_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(())
}

/// Complete a swap (mark as completed)
pub async fn complete_swap(pool: &SqlitePool, swap_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE currency_swap SET status = 'completed', date_updated = datetime('now') WHERE id = ?")
        .bind(swap_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Cancel a swap
/// Implements sp_cancel_swap logic: validates swap is pending, refunds maker's balance
pub async fn cancel_swap(pool: &SqlitePool, swap_id: i64) -> Result<(), sqlx::Error> {
    // Start a transaction
    let mut tx = pool.begin().await?;

    // Get swap details
    let swap_row = sqlx::query(
        "SELECT maker_id, CAST(maker_amount AS REAL) as maker_amount, status FROM currency_swap WHERE id = ?"
    )
    .bind(swap_id)
    .fetch_optional(&mut *tx)
    .await?;

    let swap = match swap_row {
        Some(row) => row,
        None => {
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol("Swap not found".into()));
        }
    };

    let status: String = swap.get("status");
    if status != "pending" {
        tx.rollback().await?;
        return Err(sqlx::Error::Protocol("Swap is not pending".into()));
    }

    let maker_account_id: i64 = swap.get("maker_id");
    let maker_amount: f64 = swap.get("maker_amount");

    // Refund maker's balance (only the maker had their balance deducted during swap creation)
    sqlx::query("UPDATE account SET balance = balance + ?, date_updated = datetime('now') WHERE id = ?")
        .bind(maker_amount)
        .bind(maker_account_id)
        .execute(&mut *tx)
        .await?;

    // Update swap status to cancelled
    sqlx::query("UPDATE currency_swap SET status = 'cancelled', date_updated = datetime('now') WHERE id = ?")
        .bind(swap_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

/// Get a swap by ID (uses direct query, same as get_swap_by_id)
pub async fn get_swap(
    pool: &SqlitePool,
    swap_id: i64,
) -> Result<Option<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    get_swap_by_id(pool, swap_id).await
}

/// Get pending swaps by maker ID (direct query)
pub async fn get_pending_swaps_by_maker(
    pool: &SqlitePool,
    maker_id: i64,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    get_pending_swaps_for_maker(pool, maker_id).await
}

/// Get all swaps by maker ID (any status)
pub async fn get_swaps_by_maker(
    pool: &SqlitePool,
    maker_id: i64,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "SELECT CAST(id AS INTEGER), CAST(maker_id AS INTEGER), CAST(taker_id AS INTEGER), CAST(maker_currency_id AS INTEGER), 
                CAST(taker_currency_id AS INTEGER), CAST(maker_amount AS REAL), CAST(taker_amount AS REAL), status 
         FROM currency_swap WHERE maker_id = ? ORDER BY date_created DESC"
    )
    .bind(maker_id)
    .fetch_all(pool)
    .await
}

/// Get all swaps by taker ID (any status)
pub async fn get_swaps_by_taker(
    pool: &SqlitePool,
    taker_id: i64,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "SELECT CAST(id AS INTEGER), CAST(maker_id AS INTEGER), CAST(taker_id AS INTEGER), CAST(maker_currency_id AS INTEGER), 
                CAST(taker_currency_id AS INTEGER), CAST(maker_amount AS REAL), CAST(taker_amount AS REAL), status 
         FROM currency_swap WHERE taker_id = ? ORDER BY date_created DESC"
    )
    .bind(taker_id)
    .fetch_all(pool)
    .await
}

/// Get all pending swaps (admin view)
pub async fn get_all_pending_swaps(
    pool: &SqlitePool,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "SELECT CAST(id AS INTEGER), CAST(maker_id AS INTEGER), CAST(taker_id AS INTEGER), CAST(maker_currency_id AS INTEGER), 
                CAST(taker_currency_id AS INTEGER), CAST(maker_amount AS REAL), CAST(taker_amount AS REAL), status 
         FROM currency_swap WHERE status = 'pending' ORDER BY date_created DESC"
    )
    .fetch_all(pool)
    .await
}

/// Get all open swaps (swaps without a taker)
pub async fn get_all_open_swaps(
    pool: &SqlitePool,
) -> Result<Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String)>(
        "SELECT CAST(id AS INTEGER), CAST(maker_id AS INTEGER), CAST(taker_id AS INTEGER), CAST(maker_currency_id AS INTEGER), 
                CAST(taker_currency_id AS INTEGER), CAST(maker_amount AS REAL), CAST(taker_amount AS REAL), status 
         FROM currency_swap WHERE taker_id IS NULL AND status = 'pending' ORDER BY date_created DESC"
    )
    .fetch_all(pool)
    .await
}

/// Store swap message ID for later editing
pub async fn store_swap_message(
    pool: &SqlitePool,
    swap_id: i64,
    channel_id: i64,
    message_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO swap_message (swap_id, channel_id, message_id) VALUES (?, ?, ?)"
    )
    .bind(swap_id)
    .bind(channel_id)
    .bind(message_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get swap message ID by swap ID
pub async fn get_swap_message(
    pool: &SqlitePool,
    swap_id: i64,
) -> Result<Option<(i64, i64, i64)>, sqlx::Error> {
    sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT swap_id, channel_id, message_id FROM swap_message WHERE swap_id = ?"
    )
    .bind(swap_id)
    .fetch_optional(pool)
    .await
}

/// Get paginated swaps with optional filters
/// Returns: Vec<(swap_id, maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status, maker_ticker, taker_ticker)>
/// Supports filters:
/// - oldest/latest: sort order (default: latest)
/// - pending/accepted/cancelled: status filter (default: pending)
/// - highmaker/lowmaker/hightaker/lowtaker: sort by amount
/// - base:ABC/quote:XYZ: filter by currency ticker
pub async fn get_swaps_paginated(
    pool: &SqlitePool,
    page: usize,
    page_size: usize,
    sort_by: &str,           // "oldest", "latest", "highmaker", "lowmaker", "hightaker", "lowtaker"
    status_filter: &str,     // "pending", "accepted", "cancelled", or "all"
    base_currency: Option<&str>,  // filter by base currency ticker (maker currency)
    quote_currency: Option<&str>, // filter by quote currency ticker (taker currency)
) -> Result<(Vec<(i64, i64, Option<i64>, i64, i64, f64, f64, String, String, String)>, i64), sqlx::Error> {
    let offset = (page - 1) * page_size;
    
    // Build the query
    let mut query_str = String::from(
        "SELECT 
            CAST(cs.id AS INTEGER),
            CAST(a_maker.discord_id AS INTEGER),
            CAST(a_taker.discord_id AS INTEGER),
            CAST(cs.maker_currency_id AS INTEGER),
            CAST(cs.taker_currency_id AS INTEGER),
            CAST(cs.maker_amount AS REAL),
            CAST(cs.taker_amount AS REAL),
            cs.status,
            c_maker.ticker,
            c_taker.ticker
         FROM currency_swap cs
         JOIN account a_maker ON cs.maker_id = a_maker.id
         LEFT JOIN account a_taker ON cs.taker_id = a_taker.id
         JOIN currency c_maker ON cs.maker_currency_id = c_maker.id
         JOIN currency c_taker ON cs.taker_currency_id = c_taker.id
         WHERE 1=1"
    );
    
    // Add status filter
    if status_filter != "all" {
        query_str.push_str(&format!(" AND cs.status = '{}'", status_filter));
    }
    
    // Add base currency filter (maker_currency)
    if let Some(base_ticker) = base_currency {
        query_str.push_str(&format!(" AND UPPER(c_maker.ticker) = UPPER('{}')", base_ticker));
    }
    
    // Add quote currency filter (taker_currency)
    if let Some(quote_ticker) = quote_currency {
        query_str.push_str(&format!(" AND UPPER(c_taker.ticker) = UPPER('{}')", quote_ticker));
    }
    
    // Add ORDER BY clause
    match sort_by {
        "oldest" => query_str.push_str(" ORDER BY cs.date_created ASC"),
        "latest" => query_str.push_str(" ORDER BY cs.date_created DESC"),
        "highmaker" => query_str.push_str(" ORDER BY cs.maker_amount DESC"),
        "lowmaker" => query_str.push_str(" ORDER BY cs.maker_amount ASC"),
        "hightaker" => query_str.push_str(" ORDER BY cs.taker_amount DESC"),
        "lowtaker" => query_str.push_str(" ORDER BY cs.taker_amount ASC"),
        _ => query_str.push_str(" ORDER BY cs.date_created DESC"),
    }
    
    // Get total count
    let count_query = format!(
        "SELECT COUNT(*) as count FROM currency_swap cs
         JOIN account a_maker ON cs.maker_id = a_maker.id
         LEFT JOIN account a_taker ON cs.taker_id = a_taker.id
         JOIN currency c_maker ON cs.maker_currency_id = c_maker.id
         JOIN currency c_taker ON cs.taker_currency_id = c_taker.id
         WHERE 1=1{}{}{}",
        if status_filter != "all" { format!(" AND cs.status = '{}'", status_filter) } else { String::new() },
        if let Some(base_ticker) = base_currency { format!(" AND UPPER(c_maker.ticker) = UPPER('{}')", base_ticker) } else { String::new() },
        if let Some(quote_ticker) = quote_currency { format!(" AND UPPER(c_taker.ticker) = UPPER('{}')", quote_ticker) } else { String::new() }
    );
    
    let total_count: (i64,) = sqlx::query_as(&count_query)
        .fetch_one(pool)
        .await?;
    
    // Add LIMIT and OFFSET
    query_str.push_str(&format!(" LIMIT {} OFFSET {}", page_size, offset));
    
    // Execute query
    let swaps = sqlx::query_as::<_, (i64, i64, Option<i64>, i64, i64, f64, f64, String, String, String)>(
        &query_str
    )
    .fetch_all(pool)
    .await?;
    
    Ok((swaps, total_count.0))
}

/// Get total maker amount in pending/open swaps for a currency
pub async fn get_total_swap_maker_amount(
    pool: &SqlitePool,
    currency_id: i64,
) -> Result<Option<f64>, sqlx::Error> {
    let row = sqlx::query("SELECT CAST(SUM(CAST(maker_amount AS REAL)) AS DOUBLE) as total FROM currency_swap WHERE maker_currency_id = ? AND status = 'pending'")
        .bind(currency_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.and_then(|r| r.get::<Option<f64>, _>("total")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // Create tables
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS currency (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                guild_id INTEGER UNIQUE NOT NULL,
                name TEXT UNIQUE NOT NULL,
                ticker TEXT UNIQUE NOT NULL,
                date_created TEXT DEFAULT (datetime('now'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS account (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                discord_id INTEGER NOT NULL,
                currency_id INTEGER NOT NULL,
                balance REAL NOT NULL DEFAULT 0.0,
                date_created TEXT DEFAULT (datetime('now')),
                date_updated TEXT DEFAULT (datetime('now')),
                UNIQUE (discord_id, currency_id)
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS currency_swap (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                maker_id INTEGER NOT NULL,
                taker_id INTEGER,
                maker_currency_id INTEGER NOT NULL,
                taker_currency_id INTEGER NOT NULL,
                maker_amount REAL NOT NULL,
                taker_amount REAL NOT NULL,
                status TEXT DEFAULT 'pending',
                date_created TEXT DEFAULT (datetime('now')),
                date_updated TEXT DEFAULT (datetime('now'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS \"transaction\" (
                uuid TEXT PRIMARY KEY,
                sender_id INTEGER NOT NULL,
                receiver_id INTEGER NOT NULL,
                amount REAL NOT NULL,
                date_created TEXT DEFAULT (datetime('now'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS swap_message (
                swap_id INTEGER PRIMARY KEY,
                channel_id INTEGER NOT NULL,
                message_id INTEGER NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    async fn create_test_currency(pool: &SqlitePool, guild_id: i64, name: &str, ticker: &str) -> i64 {
        let result = sqlx::query("INSERT INTO currency (guild_id, name, ticker) VALUES (?, ?, ?)")
            .bind(guild_id)
            .bind(name)
            .bind(ticker)
            .execute(pool)
            .await
            .unwrap();
        result.last_insert_rowid()
    }

    async fn create_test_account(pool: &SqlitePool, discord_id: i64, currency_id: i64, balance: f64) -> i64 {
        let result = sqlx::query("INSERT INTO account (discord_id, currency_id, balance) VALUES (?, ?, ?)")
            .bind(discord_id)
            .bind(currency_id)
            .bind(balance)
            .execute(pool)
            .await
            .unwrap();
        result.last_insert_rowid()
    }

    #[tokio::test]
    async fn test_create_swap_targeted() {
        let pool = setup_test_db().await;

        // Create currencies and accounts
        let currency1 = create_test_currency(&pool, 1, "Currency A", "CRA").await;
        let currency2 = create_test_currency(&pool, 2, "Currency B", "CRB").await;
        let maker_account = create_test_account(&pool, 100, currency1, 1000.0).await;
        let taker_account = create_test_account(&pool, 200, currency2, 500.0).await;

        let swap_id = create_swap(&pool, maker_account, currency1, currency2, 100.0, 50.0, taker_account)
            .await
            .unwrap();

        assert!(swap_id > 0);

        // Check maker balance was deducted
        let balance: f64 = sqlx::query_scalar("SELECT balance FROM account WHERE id = ?")
            .bind(maker_account)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(balance, 900.0);
    }

    #[tokio::test]
    async fn test_create_swap_insufficient_balance() {
        let pool = setup_test_db().await;

        let currency1 = create_test_currency(&pool, 1, "Currency A", "CRA").await;
        let currency2 = create_test_currency(&pool, 2, "Currency B", "CRB").await;
        let maker_account = create_test_account(&pool, 100, currency1, 50.0).await;
        let taker_account = create_test_account(&pool, 200, currency2, 500.0).await;

        let result = create_swap(&pool, maker_account, currency1, currency2, 100.0, 50.0, taker_account).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_swap_open() {
        let pool = setup_test_db().await;

        let currency1 = create_test_currency(&pool, 1, "Currency A", "CRA").await;
        let currency2 = create_test_currency(&pool, 2, "Currency B", "CRB").await;
        let maker_account = create_test_account(&pool, 100, currency1, 1000.0).await;

        let swap_id = create_swap_open(&pool, maker_account, currency1, currency2, 100.0, 50.0)
            .await
            .unwrap();

        assert!(swap_id > 0);

        // Verify swap has no taker
        let swap = get_swap_by_id(&pool, swap_id).await.unwrap().unwrap();
        assert!(swap.2.is_none()); // taker_id should be None
    }

    #[tokio::test]
    async fn test_cancel_swap() {
        let pool = setup_test_db().await;

        let currency1 = create_test_currency(&pool, 1, "Currency A", "CRA").await;
        let currency2 = create_test_currency(&pool, 2, "Currency B", "CRB").await;
        let maker_account = create_test_account(&pool, 100, currency1, 1000.0).await;

        let swap_id = create_swap_open(&pool, maker_account, currency1, currency2, 100.0, 50.0)
            .await
            .unwrap();

        // Cancel the swap
        cancel_swap(&pool, swap_id).await.unwrap();

        // Check maker balance was refunded
        let balance: f64 = sqlx::query_scalar("SELECT balance FROM account WHERE id = ?")
            .bind(maker_account)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(balance, 1000.0);

        // Check swap status
        let swap = get_swap_by_id(&pool, swap_id).await.unwrap().unwrap();
        assert_eq!(swap.7, "cancelled");
    }

    #[tokio::test]
    async fn test_get_swap_by_id() {
        let pool = setup_test_db().await;

        let currency1 = create_test_currency(&pool, 1, "Currency A", "CRA").await;
        let currency2 = create_test_currency(&pool, 2, "Currency B", "CRB").await;
        let maker_account = create_test_account(&pool, 100, currency1, 1000.0).await;

        let swap_id = create_swap_open(&pool, maker_account, currency1, currency2, 100.0, 50.0)
            .await
            .unwrap();

        let swap = get_swap_by_id(&pool, swap_id).await.unwrap();
        assert!(swap.is_some());
        let swap = swap.unwrap();
        assert_eq!(swap.0, swap_id);
        assert_eq!(swap.5, 100.0); // maker_amount
        assert_eq!(swap.6, 50.0);  // taker_amount
        assert_eq!(swap.7, "pending");
    }

    #[tokio::test]
    async fn test_store_and_get_swap_message() {
        let pool = setup_test_db().await;

        store_swap_message(&pool, 1, 123456, 789012).await.unwrap();

        let result = get_swap_message(&pool, 1).await.unwrap();
        assert!(result.is_some());
        let (swap_id, channel_id, message_id) = result.unwrap();
        assert_eq!(swap_id, 1);
        assert_eq!(channel_id, 123456);
        assert_eq!(message_id, 789012);
    }
}
