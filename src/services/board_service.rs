use serenity::model::channel::Message;
use serenity::prelude::Context;
use serenity::builder::CreateEmbed;

use crate::utils::Page;

const ITEMS_PER_PAGE: usize = 10;

pub async fn list_currencies(ctx: &Context, msg: &Message, args: &[&str]) -> Result<(), String> {
    let guild_id = msg.guild_id.map(|id| id.get() as i64).ok_or("This command can only be used in a guild")?;
    
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

    // Fetch currencies from database
    let currencies = crate::db::currency::get_currencies_by_guild_sorted(&pool, guild_id, sort_by)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    if currencies.is_empty() {
        return Err("❌ No currencies found in this guild. Create one with `$create_currency`".to_string());
    }

    // Create paginated embeds
    let pages = create_currency_pages(&currencies, sort_by);

    // Validate page number
    if page_num < 1 || page_num > pages.len() {
        return Err(format!(
            "❌ Invalid page number. This command has {} page(s)",
            pages.len()
        ));
    }

    // Create page object and navigate to requested page
    let mut page = Page::new(pages);
    for _ in 1..page_num {
        page.next();
    }

    // Send the message
    msg.channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default().embed(page.current_embed().clone()))
        .await
        .map_err(|e| format!("Failed to send message: {}", e))?;

    Ok(())
}

fn create_currency_pages(currencies: &[(i64, String, String)], sort_by: &str) -> Vec<CreateEmbed> {
    let mut pages = Vec::new();
    let total_pages = (currencies.len() + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE;

    for page_idx in 0..total_pages {
        let start = page_idx * ITEMS_PER_PAGE;
        let end = std::cmp::min(start + ITEMS_PER_PAGE, currencies.len());
        let page_currencies = &currencies[start..end];

        let mut description = String::new();
        for (idx, (_id, name, ticker)) in page_currencies.iter().enumerate() {
            let item_num = start + idx + 1;
            description.push_str(&format!("{}. **{}** (`{}`)\n", item_num, name, ticker));
        }

        let sort_label = if sort_by.to_lowercase() == "recent" {
            "Recent"
        } else {
            "Oldest"
        };

        let footer_text = format!(
            "Page {}/{} • Sorted by: {}",
            page_idx + 1,
            total_pages,
            sort_label
        );

        let embed = CreateEmbed::default()
            .title("💱 Currency Board")
            .description(description)
            .footer(serenity::builder::CreateEmbedFooter::new(footer_text))
            .color(0x00b0f4);

        pages.push(embed);
    }

    pages
}
