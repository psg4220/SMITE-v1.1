/// Blacklisted tickers to prevent scams and impersonation of real-world currencies
pub fn get_blacklisted_tickers() -> Vec<String> {
    vec![
        // Fiat currencies
        "USD", "EUR", "GBP", "JPY", "AUD", "CAD", "CHF", "CNY", "SEK", "NZD",
        "MXN", "SGD", "HKD", "NOK", "KRW", "TRY", "RUB", "INR", "BRL", "ZAR",
        "MYR", "PHP", "IDR", "THB", "VND", "PKR", "BGN", "HRK", "CZK", "DKK",
        "HUF", "PLN", "RON", "COP", "AED", "SAR", "QAR", "KWD", "BHD", "OMR",
        "JOD", "LBP", "EGP", "ILS", "NGN", "GHS", "KES", "UGX", "ETB", "MAD",
        "TND", "ARS", "CLP", "PEN", "UYU", "BWP", "LSL", "SZL", "MUR", "SCR",
        // Cryptocurrencies
        "BTC", "ETH", "XRP", "BCH", "LTC", "EOS", "XLM", "ADA", "TRX", "NEO",
        "IOTA", "XMR", "DASH", "ZEC", "BSV", "DOT", "DOGE", "VET", "THETA",
        "LINK", "UNI", "AAVE", "SNX", "YFI", "SUSHI", "CREAM", "BAND", "REN",
        "MATIC", "CELO", "FIL", "ALGO", "ATOM", "AVAX", "SOLA", "SOL", "FTT",
        "OKB", "BNB", "HT", "LEO", "SHIB", "DYDX", "ARB", "OP", "BLUR",
        "GMX", "JOE", "MAGI", "ILV", "ENJ", "SAND", "MANA", "FLOW", "GALA",
        "APE", "BAYC", "X2Y2", "MINT", "LOOT",
        // Precious metals
        "XAU", "XAG", "XPT", "XPD",
        // Commodities
        "WTI", "BREN", "GOLD", "SILV"
    ].iter().map(|s| s.to_string()).collect()
}
