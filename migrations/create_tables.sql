-- Create all database tables

SET FOREIGN_KEY_CHECKS=0;

CREATE TABLE IF NOT EXISTS currency (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    guild_id BIGINT UNIQUE NOT NULL,
    name VARCHAR(64) UNIQUE NOT NULL,
    ticker VARCHAR(16) UNIQUE NOT NULL,
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS account (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    discord_id BIGINT NOT NULL,
    currency_id BIGINT NOT NULL,
    balance DECIMAL(24,8) NOT NULL DEFAULT 0.0,
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP,
    date_updated DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    UNIQUE KEY uk_account_discord_currency (discord_id, currency_id),
    INDEX idx_account_currency (currency_id),
    INDEX idx_account_discord (discord_id),
    
    CONSTRAINT fk_account_currency
        FOREIGN KEY (currency_id)
        REFERENCES currency(id)
        ON DELETE RESTRICT
        ON UPDATE CASCADE
);

CREATE TABLE IF NOT EXISTS currency_swap (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    maker_id BIGINT NOT NULL,
    taker_id BIGINT NULL,
    maker_currency_id BIGINT NOT NULL,
    taker_currency_id BIGINT NOT NULL,
    maker_amount DECIMAL(24,8) NOT NULL,
    taker_amount DECIMAL(24,8) NOT NULL,
    status ENUM('pending','accepted','completed','cancelled','expired') DEFAULT 'pending',
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP,
    date_updated DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_swap_status (status),
    INDEX idx_swap_maker (maker_id),
    INDEX idx_swap_taker (taker_id),
    
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
);

CREATE TABLE IF NOT EXISTS transaction (
    uuid CHAR(36) PRIMARY KEY,
    sender_id BIGINT NOT NULL,
    receiver_id BIGINT NOT NULL,
    amount DECIMAL(24,8) NOT NULL,
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    INDEX idx_transaction_sender (sender_id),
    INDEX idx_transaction_receiver (receiver_id),
    
    CONSTRAINT fk_transaction_sender
        FOREIGN KEY (sender_id)
        REFERENCES account(id)
        ON DELETE RESTRICT ON UPDATE CASCADE,
    CONSTRAINT fk_transaction_receiver
        FOREIGN KEY (receiver_id)
        REFERENCES account(id)
        ON DELETE RESTRICT ON UPDATE CASCADE
);

CREATE TABLE IF NOT EXISTS tradelog (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    base_currency_id BIGINT NOT NULL,
    quote_currency_id BIGINT NOT NULL,
    price DECIMAL(24,8) NOT NULL,
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    INDEX idx_tradelog_pair_date (base_currency_id, quote_currency_id, date_created DESC),
    
    CONSTRAINT fk_tradelog_base_currency
        FOREIGN KEY (base_currency_id)
        REFERENCES currency(id)
        ON DELETE RESTRICT ON UPDATE CASCADE,
    CONSTRAINT fk_tradelog_quote_currency
        FOREIGN KEY (quote_currency_id)
        REFERENCES currency(id)
        ON DELETE RESTRICT ON UPDATE CASCADE
);

CREATE TABLE IF NOT EXISTS tax_account (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    currency_id BIGINT UNIQUE NOT NULL,
    balance DECIMAL(24,8) NOT NULL DEFAULT 0.0,
    tax_percentage INT NOT NULL DEFAULT 0,
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP,
    date_updated DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    CONSTRAINT fk_tax_account_currency
        FOREIGN KEY (currency_id)
        REFERENCES currency(id)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE TABLE IF NOT EXISTS api_type (
    id TINYINT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS api_token (
    id INT AUTO_INCREMENT PRIMARY KEY,
    currency_id BIGINT NOT NULL,
    api_type_id TINYINT NOT NULL,
    encrypted_token LONGTEXT NOT NULL,
    date_created DATETIME DEFAULT CURRENT_TIMESTAMP,
    date_updated DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_api_token_currency (currency_id),
    INDEX idx_api_token_type (api_type_id),
    
    CONSTRAINT fk_api_token_currency
        FOREIGN KEY (currency_id)
        REFERENCES currency(id)
        ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT fk_api_token_type
        FOREIGN KEY (api_type_id)
        REFERENCES api_type(id)
        ON DELETE RESTRICT ON UPDATE CASCADE
);

SET FOREIGN_KEY_CHECKS=1;
