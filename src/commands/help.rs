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
            "`$ping` - Check bot latency and shard info\n`$help` - Show this help message",
            false,
        )
        .field(
            "💱 Currency",
            "`$create_currency <NAME> <TICKER>` - Create guild currency (Admin)\n`$info <TICKER>` - View currency details\n`$board` - List all currencies",
            false,
        )
        .field(
            "💰 Balance & Accounts",
            "`$balance [TICKER]` - Check your balance\n`$mint <@user> <amount> <TICKER>` - Mint currency (Minter/Admin)\n`$mint -s <amount> <TICKER>` - Set exact balance (Admin only)",
            false,
        )
        .field(
            "💸 Transactions",
            "`$send @user <amount> <TICKER>` - Transfer funds\n`$transaction [list|UUID]` - View transaction history",
            false,
        ).field(
            "💸 Multiple Transactions",
            "`$send (@user1 @user2 ... @userN) <amount> <TICKER>` - Transfer funds\n`$transaction [list|UUID]` - View transaction history",
            false,
        )
        .field(
            "💱 Swaps & Trading",
            "`$swap set <amount> <TICKER> [@user] [<amount> <TICKER>]` - Create swap offer\n`$swap list [status]` - View swaps (pending/accepted/all)\n`$swap accept <ID>` - Accept swap\n`$swap deny <ID>` - Reject swap",
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
            "🌐 Bridge to UnbelievaBoat",
            "`$wire in <amount> <TICKER>` - Transfer from UnbelievaBoat bank to SMITE\n`$wire out <guild_id> <amount>` - Transfer from SMITE to UnbelievaBoat bank\n`$wire set token <TOKEN>` - Configure UnbelievaBoat API token (DM-only, Admin)\n\n⚠️ **SECURITY WARNING**: Always run `$wire set token` in a **private/admin channel** to avoid exposing your token in public chat!",
            false,
        )
        .field(
            "⚡ Performance & Rate Limiting",
            "🔹 **Autosharding**: Bot automatically scales across multiple shards\n🔹 **Per-user cooldown**: 5 seconds per command per user\n🔹 **Global rate limit**: 50 requests/second\n🔹 **UnbelievaBoat API**: Rate limited to 20 requests/second",
            false,
        )
        .field(
            "📚 More Information",
            "Use `$ping` for latency and shard details\nVisit [documentation](https://github.com/psg4220/SMITE-v1.1/wiki/Commands) for detailed command info",
            false,
        );

    msg.channel_id
        .send_message(ctx, serenity::builder::CreateMessage::default().embed(embed))
        .await
        .map_err(|e| format!("Failed to send help message: {}", e))?;

    Ok(())
}


