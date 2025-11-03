use serenity::model::channel::Message;
use serenity::prelude::Context;
use serenity::builder::CreateEmbed;

const ITEMS_PER_PAGE: usize = 10;

pub async fn list_currencies(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    // Extract pool from context
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>()
            .cloned()
            .ok_or("Database pool not found")?
    };

    // Parse sort order and page number
    let mut sort_by = "oldest"; // default
    let mut page_num = 1;

    for arg in args {
        match arg.to_lowercase().as_str() {
            "recent" => sort_by = "recent",
            arg => {
                // Try to parse as page number
                if let Ok(num) = arg.parse::<usize>() {
                    page_num = num;
                }
            }
        }
    }

    // Fetch paginated currencies from database (all currencies)
    let (currencies, total_count) = crate::db::currency::get_currencies_paginated(&pool, sort_by, page_num, ITEMS_PER_PAGE)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    if currencies.is_empty() && page_num == 1 {
        return Err("❌ No currencies found. Create one with `$create_currency`".to_string());
    }

    // Calculate total pages
    let total_pages = (total_count as usize + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE;

    // Validate page number
    if page_num < 1 || page_num > total_pages.max(1) {
        return Err(format!(
            "❌ Invalid page number. This command has {} page(s)",
            total_pages
        ));
    }

    // Create embed for this page
    let embed = create_currency_page(&currencies, page_num, total_pages, sort_by);

    // Send the message
    msg.channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
        .await
        .map_err(|e| format!("Failed to send message: {}", e))?;

    Ok(())
}

fn create_currency_page(currencies: &[(i64, String, String)], page_num: usize, total_pages: usize, sort_by: &str) -> CreateEmbed {
    let mut description = String::new();
    for (idx, (_id, name, ticker)) in currencies.iter().enumerate() {
        let item_num = (page_num - 1) * ITEMS_PER_PAGE + idx + 1;
        description.push_str(&format!("{}. **{}** (`{}`)\n", item_num, name, ticker));
    }

    let sort_label = if sort_by.to_lowercase() == "recent" {
        "Recent"
    } else {
        "Oldest"
    };

    let footer_text = format!(
        "Page {}/{} • Sorted by: {}",
        page_num,
        total_pages,
        sort_label
    );

    CreateEmbed::default()
        .title("💱 Currency Board")
        .description(description)
        .footer(serenity::builder::CreateEmbedFooter::new(footer_text))
        .color(0x00b0f4)
}
