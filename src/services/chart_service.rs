use sqlx::mysql::MySqlPool;
use plotters::prelude::*;
use chrono::{DateTime, Utc, NaiveDateTime, Duration};
use crate::db;

/// Chart data point with timestamp and price
#[derive(Debug, Clone)]
pub struct PricePoint {
    pub timestamp: DateTime<Utc>,
    pub price: f64,
}

/// Parse timeframe string to duration
/// Supported: 1d, 2d, 4d, 7d, 1w, 2w, 4w, 1M, 3M, 1y, all
pub fn parse_chart_timeframe(timeframe: &str) -> Result<Option<Duration>, String> {
    match timeframe.to_lowercase().as_str() {
        "1d" => Ok(Some(Duration::days(1))),
        "2d" => Ok(Some(Duration::days(2))),
        "4d" => Ok(Some(Duration::days(4))),
        "7d" => Ok(Some(Duration::days(7))),
        "1w" => Ok(Some(Duration::weeks(1))),
        "2w" => Ok(Some(Duration::weeks(2))),
        "4w" => Ok(Some(Duration::weeks(4))),
        "1m" | "1month" => Ok(Some(Duration::days(30))),
        "3m" | "3months" => Ok(Some(Duration::days(90))),
        "1y" | "1year" => Ok(Some(Duration::days(365))),
        "all" => Ok(None), // None means no time filter
        _ => Err(format!("❌ Unknown timeframe: '{}'. Supported: 1d, 2d, 4d, 7d, 1w, 2w, 4w, 1M, 3M, 1y, all", timeframe)),
    }
}

/// Get all price logs for a currency pair sorted by date
pub async fn get_price_history(
    pool: &MySqlPool,
    base_ticker: &str,
    quote_ticker: &str,
) -> Result<Vec<PricePoint>, String> {
    // Get currency IDs by tickers
    let base_currency = db::currency::get_currency_by_ticker(pool, base_ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("❌ Currency '{}' not found", base_ticker))?;

    let quote_currency = db::currency::get_currency_by_ticker(pool, quote_ticker)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or(format!("❌ Currency '{}' not found", quote_ticker))?;

    let base_currency_id = base_currency.0;
    let quote_currency_id = quote_currency.0;

    // Get the canonical order
    let (canonical_base_id, canonical_quote_id, is_reversed) = 
        db::tradelog::normalize_pair(pool, base_currency_id, quote_currency_id)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

    // Fetch price logs with timestamps for this pair
    let logs = db::tradelog::get_price_logs_with_timestamps(pool, canonical_base_id, canonical_quote_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    if logs.is_empty() {
        return Err("❌ No trading history found for this pair.".to_string());
    }

    // Convert logs to price points
    let mut points: Vec<PricePoint> = Vec::new();
    
    for (_id, price, date_str) in logs {
        // Calculate the displayed price (invert if reversed)
        let displayed_price = if is_reversed {
            1.0 / price
        } else {
            price
        };

        // Parse timestamp from database string (format: "YYYY-MM-DD HH:MM:SS")
        let naive_dt = NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_else(|_| Utc::now().naive_utc());
        let timestamp = DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc);
        
        points.push(PricePoint {
            timestamp,
            price: displayed_price,
        });
    }

    // Sort by timestamp (should already be sorted from DB, but ensure it)
    points.sort_by_key(|p| p.timestamp);

    Ok(points)
}

/// Get price history filtered by timeframe
pub async fn get_price_history_with_timeframe(
    pool: &MySqlPool,
    base_ticker: &str,
    quote_ticker: &str,
    timeframe: &str,
) -> Result<Vec<PricePoint>, String> {
    let mut points = get_price_history(pool, base_ticker, quote_ticker).await?;
    
    // Parse timeframe and filter if needed
    if let Some(duration) = parse_chart_timeframe(timeframe)? {
        let now = Utc::now();
        let cutoff_time = now - duration;
        
        points.retain(|p| p.timestamp >= cutoff_time);
        
        if points.is_empty() {
            return Err(format!("❌ No price data found in the last {}", timeframe));
        }
    }
    
    Ok(points)
}

/// Generate a price chart image as PNG bytes
pub async fn generate_chart(
    pool: &MySqlPool,
    base_ticker: &str,
    quote_ticker: &str,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let price_points = get_price_history(pool, base_ticker, quote_ticker).await?;

    if price_points.len() < 2 {
        return Err("❌ Not enough price data to generate chart (minimum 2 points required).".to_string());
    }

    // Use a temporary file path for BitMapBackend
    let temp_file = format!("/tmp/smite_chart_{}.png", chrono::Utc::now().timestamp_millis());
    
    {
        let backend = BitMapBackend::new(&temp_file, (width, height));
        let root = backend.into_drawing_area();
        root.fill(&WHITE)
            .map_err(|e| format!("Failed to fill canvas: {}", e))?;

        // Find price range
        let min_price = price_points.iter()
            .map(|p| p.price)
            .fold(f64::INFINITY, f64::min);
        let max_price = price_points.iter()
            .map(|p| p.price)
            .fold(f64::NEG_INFINITY, f64::max);

        // Add some padding to the price range
        let price_range = (max_price - min_price).max(1e-8); // Avoid division by zero
        let padding = price_range * 0.1;
        let y_min = (min_price - padding).max(0.0);
        let y_max = max_price + padding;

        // Get time range
        let x_min = price_points[0].timestamp;
        let x_max = price_points[price_points.len() - 1].timestamp;

        // Build chart with f64 Y axis
        let mut chart = ChartBuilder::on(&root)
            .caption(
                &format!("{}/{} Price Chart", base_ticker, quote_ticker),
                ("sans-serif", 40.0).into_font(),
            )
            .margin(15)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(x_min..x_max, y_min..y_max)
            .map_err(|e| format!("Failed to build chart: {}", e))?;

        // Configure mesh
        chart
            .configure_mesh()
            .y_desc(&format!("{} ({} per 1 {})", quote_ticker, quote_ticker, base_ticker))
            .x_desc("Time")
            .draw()
            .map_err(|e| format!("Failed to draw mesh: {}", e))?;

        // Draw price points as circles connected by lines
        for i in 0..price_points.len() {
            if i > 0 {
                // Draw line to previous point
                chart
                    .draw_series(std::iter::once(PathElement::new(
                        vec![
                            (price_points[i - 1].timestamp, price_points[i - 1].price),
                            (price_points[i].timestamp, price_points[i].price),
                        ],
                        &BLUE,
                    )))
                    .map_err(|e| format!("Failed to draw line: {}", e))?;
            }
            // Draw circle at point
            chart
                .draw_series(std::iter::once(Circle::new(
                    (price_points[i].timestamp, price_points[i].price),
                    3,
                    BLUE.filled(),
                )))
                .map_err(|e| format!("Failed to draw point: {}", e))?;
        }

        root.present()
            .map_err(|e| format!("Failed to render chart: {}", e))?;
    }

    // Read the temporary file into memory
    use std::fs;
    let image_data = fs::read(&temp_file)
        .map_err(|e| format!("Failed to read chart file: {}", e))?;

    // Clean up temporary file
    let _ = fs::remove_file(&temp_file);

    Ok(image_data)
}

/// Generate a price chart with timeframe filtering
pub async fn generate_chart_with_timeframe(
    pool: &MySqlPool,
    base_ticker: &str,
    quote_ticker: &str,
    timeframe: &str,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let price_points = get_price_history_with_timeframe(pool, base_ticker, quote_ticker, timeframe).await?;

    if price_points.len() < 2 {
        return Err("❌ Not enough price data to generate chart (minimum 2 points required).".to_string());
    }

    // Use a temporary file path for BitMapBackend
    let temp_file = format!("/tmp/smite_chart_{}.png", chrono::Utc::now().timestamp_millis());
    
    {
        let backend = BitMapBackend::new(&temp_file, (width, height));
        let root = backend.into_drawing_area();
        root.fill(&WHITE)
            .map_err(|e| format!("Failed to fill canvas: {}", e))?;

        // Find price range
        let min_price = price_points.iter()
            .map(|p| p.price)
            .fold(f64::INFINITY, f64::min);
        let max_price = price_points.iter()
            .map(|p| p.price)
            .fold(f64::NEG_INFINITY, f64::max);

        // Add some padding to the price range
        let price_range = (max_price - min_price).max(1e-8); // Avoid division by zero
        let padding = price_range * 0.1;
        let y_min = (min_price - padding).max(0.0);
        let y_max = max_price + padding;

        // Get time range
        let x_min = price_points[0].timestamp;
        let x_max = price_points[price_points.len() - 1].timestamp;

        // Build chart with f64 Y axis
        let mut chart = ChartBuilder::on(&root)
            .caption(
                &format!("{}/{} Price Chart ({})", base_ticker, quote_ticker, timeframe),
                ("sans-serif", 40.0).into_font(),
            )
            .margin(15)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(x_min..x_max, y_min..y_max)
            .map_err(|e| format!("Failed to build chart: {}", e))?;

        // Configure mesh
        chart
            .configure_mesh()
            .y_desc(&format!("{} ({} per 1 {})", quote_ticker, quote_ticker, base_ticker))
            .x_desc("Time")
            .draw()
            .map_err(|e| format!("Failed to draw mesh: {}", e))?;

        // Draw price points as circles connected by lines
        for i in 0..price_points.len() {
            if i > 0 {
                // Draw line to previous point
                chart
                    .draw_series(std::iter::once(PathElement::new(
                        vec![
                            (price_points[i - 1].timestamp, price_points[i - 1].price),
                            (price_points[i].timestamp, price_points[i].price),
                        ],
                        &BLUE,
                    )))
                    .map_err(|e| format!("Failed to draw line: {}", e))?;
            }
            // Draw circle at point
            chart
                .draw_series(std::iter::once(Circle::new(
                    (price_points[i].timestamp, price_points[i].price),
                    3,
                    BLUE.filled(),
                )))
                .map_err(|e| format!("Failed to draw point: {}", e))?;
        }

        root.present()
            .map_err(|e| format!("Failed to render chart: {}", e))?;
    }

    // Read the temporary file into memory
    use std::fs;
    let image_data = fs::read(&temp_file)
        .map_err(|e| format!("Failed to read chart file: {}", e))?;

    // Clean up temporary file
    let _ = fs::remove_file(&temp_file);

    Ok(image_data)
}

/// Parse timeframe string to minutes
/// Examples: "1m" -> 1, "1h" -> 60, "1d" -> 1440, "7d" -> 10080, "1mnt" -> 43200, "1y" -> 525600
pub fn parse_timeframe_to_minutes(timeframe: &str) -> Result<i64, String> {
    let timeframe = timeframe.to_lowercase();
    
    // Find where the letters start
    let split_idx = timeframe.chars().take_while(|c| c.is_numeric()).count();
    
    if split_idx == 0 || split_idx == timeframe.len() {
        return Err("❌ Invalid timeframe format. Examples: 1m, 5m, 1h, 4h, 1d, 7d, 1mnt, 1y".to_string());
    }
    
    let amount: i64 = timeframe[..split_idx]
        .parse()
        .map_err(|_| "❌ Invalid timeframe number".to_string())?;
    let unit = &timeframe[split_idx..];
    
    let minutes = match unit {
        "m" => amount,
        "h" => amount * 60,
        "d" => amount * 1440,
        "mnt" => amount * 43200,
        "y" => amount * 525600,
        _ => return Err(format!("❌ Unknown timeframe unit: '{}'. Use: m, h, d, mnt, y", unit)),
    };
    
    Ok(minutes)
}
