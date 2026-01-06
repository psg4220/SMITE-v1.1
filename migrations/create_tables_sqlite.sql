-- Create all database tables for SQLite/LibSQL

-- Currency table
CREATE TABLE IF NOT EXISTS currency (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id INTEGER UNIQUE NOT NULL,
    name TEXT UNIQUE NOT NULL,
    ticker TEXT UNIQUE NOT NULL,
    date_created TEXT DEFAULT (datetime('now'))
);

-- Account table
CREATE TABLE IF NOT EXISTS account (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    discord_id INTEGER NOT NULL,
    currency_id INTEGER NOT NULL,
    balance REAL NOT NULL DEFAULT 0.0,
    date_created TEXT DEFAULT (datetime('now')),
    date_updated TEXT DEFAULT (datetime('now')),
    
    UNIQUE (discord_id, currency_id),
    FOREIGN KEY (currency_id) REFERENCES currency(id) ON DELETE RESTRICT ON UPDATE CASCADE
);

-- Create indexes for account table
CREATE INDEX IF NOT EXISTS idx_account_currency ON account(currency_id);
CREATE INDEX IF NOT EXISTS idx_account_discord ON account(discord_id);

-- Currency swap table
CREATE TABLE IF NOT EXISTS currency_swap (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    maker_id INTEGER NOT NULL,
    taker_id INTEGER,
    maker_currency_id INTEGER NOT NULL,
    taker_currency_id INTEGER NOT NULL,
    maker_amount REAL NOT NULL,
    taker_amount REAL NOT NULL,
    status TEXT DEFAULT 'pending' CHECK(status IN ('pending', 'accepted', 'completed', 'cancelled', 'expired')),
    date_created TEXT DEFAULT (datetime('now')),
    date_updated TEXT DEFAULT (datetime('now')),
    
    FOREIGN KEY (maker_id) REFERENCES account(id) ON DELETE RESTRICT ON UPDATE CASCADE,
    FOREIGN KEY (taker_id) REFERENCES account(id) ON DELETE RESTRICT ON UPDATE CASCADE,
    FOREIGN KEY (maker_currency_id) REFERENCES currency(id) ON DELETE RESTRICT ON UPDATE CASCADE,
    FOREIGN KEY (taker_currency_id) REFERENCES currency(id) ON DELETE RESTRICT ON UPDATE CASCADE
);

-- Create indexes for swap table
CREATE INDEX IF NOT EXISTS idx_swap_status ON currency_swap(status);
CREATE INDEX IF NOT EXISTS idx_swap_maker ON currency_swap(maker_id);
CREATE INDEX IF NOT EXISTS idx_swap_taker ON currency_swap(taker_id);

-- Transaction table
CREATE TABLE IF NOT EXISTS "transaction" (
    uuid TEXT PRIMARY KEY,
    sender_id INTEGER NOT NULL,
    receiver_id INTEGER NOT NULL,
    amount REAL NOT NULL,
    date_created TEXT DEFAULT (datetime('now')),
    
    FOREIGN KEY (sender_id) REFERENCES account(id) ON DELETE RESTRICT ON UPDATE CASCADE,
    FOREIGN KEY (receiver_id) REFERENCES account(id) ON DELETE RESTRICT ON UPDATE CASCADE
);

-- Create indexes for transaction table
CREATE INDEX IF NOT EXISTS idx_transaction_sender ON "transaction"(sender_id);
CREATE INDEX IF NOT EXISTS idx_transaction_receiver ON "transaction"(receiver_id);

-- Tradelog table
CREATE TABLE IF NOT EXISTS tradelog (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    base_currency_id INTEGER NOT NULL,
    quote_currency_id INTEGER NOT NULL,
    price REAL NOT NULL,
    date_created TEXT DEFAULT (datetime('now')),
    
    FOREIGN KEY (base_currency_id) REFERENCES currency(id) ON DELETE RESTRICT ON UPDATE CASCADE,
    FOREIGN KEY (quote_currency_id) REFERENCES currency(id) ON DELETE RESTRICT ON UPDATE CASCADE
);

-- Create indexes for tradelog table
CREATE INDEX IF NOT EXISTS idx_tradelog_pair_date ON tradelog(base_currency_id, quote_currency_id, date_created DESC);

-- Tax account table
CREATE TABLE IF NOT EXISTS tax_account (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    currency_id INTEGER UNIQUE NOT NULL,
    balance REAL NOT NULL DEFAULT 0.0,
    tax_percentage INTEGER NOT NULL DEFAULT 0,
    date_created TEXT DEFAULT (datetime('now')),
    date_updated TEXT DEFAULT (datetime('now')),
    
    FOREIGN KEY (currency_id) REFERENCES currency(id) ON DELETE CASCADE ON UPDATE CASCADE
);

-- API type table
CREATE TABLE IF NOT EXISTS api_type (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    date_created TEXT DEFAULT (datetime('now'))
);

-- API token table
CREATE TABLE IF NOT EXISTS api_token (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    currency_id INTEGER NOT NULL,
    api_type_id INTEGER NOT NULL,
    encrypted_token TEXT NOT NULL,
    date_created TEXT DEFAULT (datetime('now')),
    date_updated TEXT DEFAULT (datetime('now')),
    
    FOREIGN KEY (currency_id) REFERENCES currency(id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (api_type_id) REFERENCES api_type(id) ON DELETE RESTRICT ON UPDATE CASCADE
);

-- Create indexes for api_token table
CREATE INDEX IF NOT EXISTS idx_api_token_currency ON api_token(currency_id);
CREATE INDEX IF NOT EXISTS idx_api_token_type ON api_token(api_type_id);

-- Swap message table (for tracking DM messages for swap notifications)
CREATE TABLE IF NOT EXISTS swap_message (
    swap_id INTEGER PRIMARY KEY,
    channel_id INTEGER NOT NULL,
    message_id INTEGER NOT NULL,
    
    FOREIGN KEY (swap_id) REFERENCES currency_swap(id) ON DELETE CASCADE ON UPDATE CASCADE
);

-- Insert default API type for UnbelievaBoat
INSERT OR IGNORE INTO api_type (id, name) VALUES (1, 'unbelievaboat');
