-- Create all database tables

DELIMITER //

CREATE TABLE IF NOT EXISTS currency (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    guild_id BIGINT UNIQUE NOT NULL,
    name VARCHAR(64) UNIQUE NOT NULL,
    ticker VARCHAR(16) UNIQUE NOT NULL,
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP
) //

CREATE TABLE IF NOT EXISTS account (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    discord_id BIGINT NOT NULL,
    currency_id BIGINT NOT NULL,
    balance DECIMAL(18,8) NOT NULL DEFAULT 0.0,
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP,
    date_updated DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    UNIQUE KEY uk_account_discord_currency (discord_id, currency_id),
    
    CONSTRAINT fk_account_currency
        FOREIGN KEY (currency_id)
        REFERENCES currency(id)
        ON DELETE RESTRICT
        ON UPDATE CASCADE
) //

CREATE INDEX IF NOT EXISTS idx_account_currency ON account(currency_id) //
CREATE INDEX IF NOT EXISTS idx_account_discord ON account(discord_id) //

CREATE TABLE IF NOT EXISTS currency_swap (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    maker_id BIGINT NOT NULL,
    taker_id BIGINT NULL,
    maker_currency_id BIGINT NOT NULL,
    taker_currency_id BIGINT NOT NULL,
    maker_amount DECIMAL(18,8) NOT NULL,
    taker_amount DECIMAL(18,8) NOT NULL,
    status ENUM('pending','accepted','completed','cancelled','expired') DEFAULT 'pending',
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP,
    date_updated DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    CONSTRAINT fk_swap_maker
        FOREIGN KEY (maker_id)
        REFERENCES account(id)
        ON DELETE RESTRICT ON UPDATE CASCADE,
    CONSTRAINT fk_swap_taker
        FOREIGN KEY (taker_id)
        REFERENCES account(id)
        ON DELETE RESTRICT ON UPDATE CASCADE,
    CONSTRAINT fk_swap_maker_currency
        FOREIGN KEY (maker_currency_id)
        REFERENCES currency(id)
        ON DELETE RESTRICT ON UPDATE CASCADE,
    CONSTRAINT fk_swap_taker_currency
        FOREIGN KEY (taker_currency_id)
        REFERENCES currency(id)
        ON DELETE RESTRICT ON UPDATE CASCADE
) //

CREATE INDEX IF NOT EXISTS idx_swap_status ON currency_swap(status) //

CREATE INDEX IF NOT EXISTS idx_swap_maker ON currency_swap(maker_id) //

CREATE INDEX IF NOT EXISTS idx_swap_taker ON currency_swap(taker_id) //

CREATE TABLE IF NOT EXISTS transaction (
    uuid CHAR(36) PRIMARY KEY,
    sender_id BIGINT NOT NULL,
    receiver_id BIGINT NOT NULL,
    amount DECIMAL(18,8) NOT NULL,
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    CONSTRAINT fk_transaction_sender
        FOREIGN KEY (sender_id)
        REFERENCES account(id)
        ON DELETE RESTRICT ON UPDATE CASCADE,
    CONSTRAINT fk_transaction_receiver
        FOREIGN KEY (receiver_id)
        REFERENCES account(id)
        ON DELETE RESTRICT ON UPDATE CASCADE
) //

CREATE INDEX IF NOT EXISTS idx_transaction_sender ON transaction(sender_id) //

CREATE INDEX IF NOT EXISTS idx_transaction_receiver ON transaction(receiver_id) //

CREATE TABLE IF NOT EXISTS tradelog (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    currency_id BIGINT NOT NULL,
    price DECIMAL(18,8) NOT NULL,
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    CONSTRAINT fk_tradelog_currency
        FOREIGN KEY (currency_id)
        REFERENCES currency(id)
        ON DELETE RESTRICT ON UPDATE CASCADE
) //

CREATE INDEX IF NOT EXISTS idx_currency_date ON tradelog(currency_id, date_created) //

DELIMITER ;
