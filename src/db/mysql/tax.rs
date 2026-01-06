use sqlx::mysql::MySqlPool;

/// Get tax account with currency guild_id
pub async fn get_tax_account_with_guild(
    pool: &MySqlPool,
    currency_id: i64,
) -> Result<Option<(i64, i64, f64, i32, i64)>, sqlx::Error> {
    let result: Option<(i64, i64, String, i32, i64)> = sqlx::query_as(
        "SELECT ta.id, ta.currency_id, CAST(ta.balance AS CHAR) as balance_str, ta.tax_percentage, c.guild_id 
         FROM tax_account ta 
         JOIN currency c ON ta.currency_id = c.id 
         WHERE ta.currency_id = ?"
    )
    .bind(currency_id)
    .fetch_optional(pool)
    .await?;

    // Convert the string back to f64
    match result {
        Some((id, curr_id, balance_str, tax_pct, guild_id)) => {
            let balance = balance_str.parse::<f64>()
                .map_err(|e| sqlx::Error::Decode(e.into()))?;
            Ok(Some((id, curr_id, balance, tax_pct, guild_id)))
        },
        None => Ok(None),
    }
}

/// Get or create tax account for a currency
pub async fn get_tax_account(
    pool: &MySqlPool,
    currency_id: i64,
) -> Result<Option<(i64, i64, f64, i32)>, sqlx::Error> {
    let result: Option<(i64, i64, String, i32)> = sqlx::query_as(
        "SELECT id, currency_id, CAST(balance AS CHAR) as balance_str, tax_percentage FROM tax_account WHERE currency_id = ?"
    )
    .bind(currency_id)
    .fetch_optional(pool)
    .await?;

    // Convert the string back to f64
    match result {
        Some((id, curr_id, balance_str, tax_pct)) => {
            let balance = balance_str.parse::<f64>()
                .map_err(|e| sqlx::Error::Decode(e.into()))?;
            Ok(Some((id, curr_id, balance, tax_pct)))
        },
        None => Ok(None),
    }
}

/// Create a new tax account for a currency
pub async fn create_tax_account(
    pool: &MySqlPool,
    currency_id: i64,
    tax_percentage: i32,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO tax_account (currency_id, balance, tax_percentage) VALUES (?, 0, ?)"
    )
    .bind(currency_id)
    .bind(tax_percentage)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

/// Update tax percentage for a currency
pub async fn set_tax_percentage(
    pool: &MySqlPool,
    currency_id: i64,
    tax_percentage: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE tax_account SET tax_percentage = ? WHERE currency_id = ?"
    )
    .bind(tax_percentage)
    .bind(currency_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Add tax to an account
pub async fn add_tax(
    pool: &MySqlPool,
    currency_id: i64,
    amount: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE tax_account SET balance = balance + ? WHERE currency_id = ?"
    )
    .bind(amount)
    .bind(currency_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Collect (withdraw) tax from an account
pub async fn collect_tax(
    pool: &MySqlPool,
    currency_id: i64,
    amount: f64,
) -> Result<f64, sqlx::Error> {
    // Get current balance - cast DECIMAL to CHAR for proper handling
    let tax_account: (i64, i64, String, i32) = sqlx::query_as(
        "SELECT id, currency_id, CAST(balance AS CHAR) as balance_str, tax_percentage FROM tax_account WHERE currency_id = ?"
    )
    .bind(currency_id)
    .fetch_one(pool)
    .await?;

    let current_balance = tax_account.2.parse::<f64>()
        .map_err(|e| sqlx::Error::Decode(e.into()))?;
    
    let collect_amount = if amount >= current_balance {
        current_balance
    } else {
        amount
    };

    // Deduct from tax account
    sqlx::query(
        "UPDATE tax_account SET balance = balance - ? WHERE currency_id = ?"
    )
    .bind(collect_amount)
    .bind(currency_id)
    .execute(pool)
    .await?;

    Ok(collect_amount)
}

/// Get tax percentage for a currency
pub async fn get_tax_percentage(
    pool: &MySqlPool,
    currency_id: i64,
) -> Result<Option<i32>, sqlx::Error> {
    let result = sqlx::query_scalar::<_, i32>(
        "SELECT tax_percentage FROM tax_account WHERE currency_id = ?"
    )
    .bind(currency_id)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

/// Get total tax balance for a currency
pub async fn get_total_tax_balance(
    pool: &MySqlPool,
    currency_id: i64,
) -> Result<Option<f64>, sqlx::Error> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT CAST(balance AS CHAR) as balance_str FROM tax_account WHERE currency_id = ?"
    )
    .bind(currency_id)
    .fetch_optional(pool)
    .await?;

    match result {
        Some((balance_str,)) => {
            let balance = balance_str.parse::<f64>()
                .map_err(|e| sqlx::Error::Decode(e.into()))?;
            Ok(Some(balance))
        },
        None => Ok(Some(0.0)),
    }
}
