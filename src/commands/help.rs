use serenity::builder::CreateEmbed;
use serenity::model::channel::Message;
use serenity::prelude::Context;

pub async fn execute(ctx: &Context, msg: &Message) -> Result<(), String> {
    let embed = CreateEmbed::default()
        .title("📖 SMITE Commands Help")
        .description("**SMITE** - Society for Micronational Interbank Transactions and Exchanges\nA Discord bot for currency exchange and financial transactions between micro-nations.")
        .color(0x00b0f4)
        .field(
            "🎯 General",
            "`$ping` - Check bot latency\n`$help` - Show this help message",
            false,
        )
        .field(
            "💱 Currency",
            "`$create_currency <NAME> <TICKER>` - Create guild currency (Admin)\n`$info <TICKER>` - View currency details\n`$board` - List all currencies",
            false,
        )
        .field(
            "💰 Balance & Accounts",
            "`$balance [TICKER]` - Check your balance\n`$mint <amount> <TICKER>` - Mint currency (default: yourself, Minter/Admin)",
            false,
        )
        .field(
            "💸 Transactions",
            "`$send @user(s) <amount> <TICKER>` - Transfer funds\n`$transaction [list|UUID]` - View transaction history",
            false,
        )
        .field(
            "💱 Swaps & Trading",
            "`$swap set <amount> <TICKER> [@user] [<amount> <TICKER>]` - Create swap offer\n`$swap list [status]` - View swaps\n`$swap accept <ID>` - Accept swap\n`$swap deny <ID>` - Reject swap",
            false,
        )
        .field(
            "💵 Tax Management",
            "`$tax set <TICKER> <percentage>` - Set tax rate (Admin/Tax Collector)\n`$tax collect <TICKER> [amount|all]` - Collect taxes\n`$tax info <TICKER>` - View tax details",
            false,
        )
        .field(
            "📊 Prices & Charts",
            "`$price <BASE>/<QUOTE> [timeframe]` - Get price\n`$price chart <BASE>/<QUOTE> [timeframe]` - Generate chart\n`$price list [filter]` - View price list",
            false,
        )
        .field(
            "⚡ Rate Limiting",
            "5-second cooldown per command per user\nGlobal 50 requests/second limit\nFor more info, visit the [documentation](https://github.com/psg4220/SMITE-v1.1/wiki/Commands).",
            false,
        );

    msg.channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
        .await
        .map_err(|e| format!("Failed to send help message: {}", e))?;

    Ok(())
}


