-- Stored procedures for swap operations

-- PROCEDURE: sp_create_swap
-- Creates a targeted swap and deducts maker's balance
-- This is for TARGETED swaps where both maker and taker are known upfront
-- Parameters: maker_account_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount
-- Returns: swap_id via USER_VARIABLE @swap_id
DELIMITER //

DROP PROCEDURE IF EXISTS sp_create_swap //

CREATE PROCEDURE sp_create_swap(
    IN p_maker_account_id BIGINT,
    IN p_maker_currency_id BIGINT,
    IN p_taker_currency_id BIGINT,
    IN p_maker_amount DECIMAL(18, 8),
    IN p_taker_amount DECIMAL(18, 8),
    IN p_taker_account_id BIGINT
)
BEGIN
    START TRANSACTION;
    
    -- Check if maker has sufficient balance
    IF (SELECT balance FROM account WHERE id = p_maker_account_id) < p_maker_amount THEN
        ROLLBACK;
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Maker has insufficient balance';
    END IF;
    
    -- Deduct maker's balance
    UPDATE account SET balance = balance - p_maker_amount WHERE id = p_maker_account_id;
    
    -- Insert swap record with taker_id for targeted swaps (NULL for open swaps)
    INSERT INTO currency_swap (maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status)
    VALUES (p_maker_account_id, p_taker_account_id, p_maker_currency_id, p_taker_currency_id, p_maker_amount, p_taker_amount, 'pending');
    
    SET @swap_id = LAST_INSERT_ID();
    
    COMMIT;
END //

DELIMITER ;

-- PROCEDURE: sp_create_swap_open
-- Creates an open swap (any user can accept)
-- For open swaps, taker_id is NULL and anyone except maker can accept
-- Parameters: maker_account_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount
-- Returns: swap_id via USER_VARIABLE @swap_id
DELIMITER //

DROP PROCEDURE IF EXISTS sp_create_swap_open //

CREATE PROCEDURE sp_create_swap_open(
    IN p_maker_account_id BIGINT,
    IN p_maker_currency_id BIGINT,
    IN p_taker_currency_id BIGINT,
    IN p_maker_amount DECIMAL(18, 8),
    IN p_taker_amount DECIMAL(18, 8)
)
BEGIN
    START TRANSACTION;
    
    -- Check if maker has sufficient balance
    IF (SELECT balance FROM account WHERE id = p_maker_account_id) < p_maker_amount THEN
        ROLLBACK;
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Maker has insufficient balance';
    END IF;
    
    -- Deduct maker's balance
    UPDATE account SET balance = balance - p_maker_amount WHERE id = p_maker_account_id;
    
    -- Insert open swap record (taker_id is NULL, anyone can accept)
    INSERT INTO currency_swap (maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status)
    VALUES (p_maker_account_id, NULL, p_maker_currency_id, p_taker_currency_id, p_maker_amount, p_taker_amount, 'pending');
    
    SET @swap_id = LAST_INSERT_ID();
    
    COMMIT;
END //

DELIMITER ;

-- PROCEDURE: sp_accept_swap
-- Accepts a pending swap (targeted or open)
-- Deducts taker's balance, credits both parties, logs transactions
-- Parameters: swap_id, user_discord_id (of the accepting user), uuid1 (transaction 1 ID), uuid2 (transaction 2 ID)
-- Returns: nothing via queries, but updates balances atomically
DELIMITER //

DROP PROCEDURE IF EXISTS sp_accept_swap //

CREATE PROCEDURE sp_accept_swap(
    IN p_swap_id BIGINT,
    IN p_user_discord_id BIGINT,
    IN p_uuid1 VARCHAR(36),
    IN p_uuid2 VARCHAR(36)
)
BEGIN
    DECLARE v_maker_account_id BIGINT;
    DECLARE v_taker_account_id BIGINT;
    DECLARE v_maker_currency_id BIGINT;
    DECLARE v_taker_currency_id BIGINT;
    DECLARE v_maker_amount DECIMAL(18, 8);
    DECLARE v_taker_amount DECIMAL(18, 8);
    DECLARE v_status VARCHAR(20);
    DECLARE v_user_taker_account_id BIGINT;
    DECLARE v_user_maker_account_id BIGINT;
    DECLARE v_maker_discord_id BIGINT;
    DECLARE v_taker_discord_id BIGINT;
    DECLARE v_taker_balance DECIMAL(18, 8);
    DECLARE v_maker_taker_account_id BIGINT;
    
    START TRANSACTION;
    
    -- Get swap details
    SELECT maker_id, taker_id, maker_currency_id, taker_currency_id, maker_amount, taker_amount, status
    INTO v_maker_account_id, v_taker_account_id, v_maker_currency_id, v_taker_currency_id, v_maker_amount, v_taker_amount, v_status
    FROM currency_swap WHERE id = p_swap_id;
    
    -- Check swap exists and is pending
    IF v_status IS NULL THEN
        ROLLBACK;
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Swap not found';
    END IF;
    
    IF v_status != 'pending' THEN
        ROLLBACK;
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Swap is not pending';
    END IF;
    
    -- Get maker's Discord ID - check if account exists
    IF v_maker_account_id IS NULL THEN
        ROLLBACK;
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Maker account not found';
    END IF;
    
    SELECT discord_id INTO v_maker_discord_id FROM account WHERE id = v_maker_account_id;
    
    IF v_maker_discord_id IS NULL THEN
        ROLLBACK;
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Maker Discord ID not found';
    END IF;
    
    -- Get taker's Discord ID (if targeted swap)
    IF v_taker_account_id IS NOT NULL THEN
        SELECT discord_id INTO v_taker_discord_id FROM account WHERE id = v_taker_account_id;
        IF v_taker_discord_id IS NULL THEN
            ROLLBACK;
            SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Taker Discord ID not found';
        END IF;
    ELSE
        SET v_taker_discord_id = 0; -- Open swap, no designated taker
    END IF;
    
    -- Authorization check
    IF v_taker_discord_id != 0 THEN
        -- Targeted swap: only designated taker can accept
        IF p_user_discord_id != v_taker_discord_id THEN
            ROLLBACK;
            SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Not authorized to accept this swap';
        END IF;
    ELSE
        -- Open swap: maker cannot accept their own swap
        IF p_user_discord_id = v_maker_discord_id THEN
            ROLLBACK;
            SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Cannot accept your own open swap';
        END IF;
    END IF;
    
    -- Get or create accepting user's accounts
    SELECT id INTO v_user_taker_account_id FROM account 
    WHERE discord_id = p_user_discord_id AND currency_id = v_taker_currency_id;
    
    IF v_user_taker_account_id IS NULL THEN
        INSERT INTO account (discord_id, currency_id, balance) 
        VALUES (p_user_discord_id, v_taker_currency_id, 0);
        SELECT LAST_INSERT_ID() INTO v_user_taker_account_id;
    END IF;
    
    SELECT id INTO v_user_maker_account_id FROM account 
    WHERE discord_id = p_user_discord_id AND currency_id = v_maker_currency_id;
    
    IF v_user_maker_account_id IS NULL THEN
        INSERT INTO account (discord_id, currency_id, balance) 
        VALUES (p_user_discord_id, v_maker_currency_id, 0);
        SELECT LAST_INSERT_ID() INTO v_user_maker_account_id;
    END IF;
    
    -- Check accepting user has sufficient balance for their currency
    SELECT balance INTO v_taker_balance FROM account WHERE id = v_user_taker_account_id;
    
    IF v_taker_balance < v_taker_amount THEN
        ROLLBACK;
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Insufficient balance to accept swap';
    END IF;
    
    -- Deduct accepting user's balance (they give their currency)
    UPDATE account SET balance = balance - v_taker_amount WHERE id = v_user_taker_account_id;
    
    -- Credit accepting user with maker's currency
    UPDATE account SET balance = balance + v_maker_amount WHERE id = v_user_maker_account_id;
    
    -- Get or create maker's account for taker currency
    SELECT id INTO v_maker_taker_account_id FROM account 
    WHERE discord_id = v_maker_discord_id AND currency_id = v_taker_currency_id;
    
    IF v_maker_taker_account_id IS NULL THEN
        INSERT INTO account (discord_id, currency_id, balance) 
        VALUES (v_maker_discord_id, v_taker_currency_id, 0);
        SELECT LAST_INSERT_ID() INTO v_maker_taker_account_id;
    END IF;
    
    -- Credit maker with accepting user's currency
    UPDATE account SET balance = balance + v_taker_amount WHERE id = v_maker_taker_account_id;
    
    -- Log transactions (2 total) using provided UUIDs to ensure uniqueness
    -- Transaction 1: Accepting user sends their currency to maker
    INSERT INTO transaction (uuid, sender_id, receiver_id, amount) 
    VALUES (p_uuid1, v_user_taker_account_id, v_maker_taker_account_id, v_taker_amount);
    
    -- Transaction 2: Maker sends their currency to accepting user
    INSERT INTO transaction (uuid, sender_id, receiver_id, amount) 
    VALUES (p_uuid2, v_maker_account_id, v_user_maker_account_id, v_maker_amount);
    
    -- Update swap status to accepted
    UPDATE currency_swap SET status = 'accepted' WHERE id = p_swap_id;
    
    -- For open swaps, set the taker_id to the accepting user's account
    IF v_taker_account_id IS NULL THEN
        UPDATE currency_swap SET taker_id = v_user_taker_account_id WHERE id = p_swap_id;
    END IF;
    
    COMMIT;
END //

DELIMITER ;

-- PROCEDURE: sp_cancel_swap
-- Cancels a pending swap and refunds both parties
-- Parameters: swap_id
-- Returns: nothing via queries, but refunds balances atomically
DELIMITER //

DROP PROCEDURE IF EXISTS sp_cancel_swap //

CREATE PROCEDURE sp_cancel_swap(
    IN p_swap_id BIGINT
)
BEGIN
    DECLARE v_maker_account_id BIGINT;
    DECLARE v_taker_account_id BIGINT;
    DECLARE v_maker_amount DECIMAL(18, 8);
    DECLARE v_taker_amount DECIMAL(18, 8);
    DECLARE v_status VARCHAR(20);
    
    START TRANSACTION;
    
    -- Get swap details
    SELECT maker_id, taker_id, maker_amount, taker_amount, status
    INTO v_maker_account_id, v_taker_account_id, v_maker_amount, v_taker_amount, v_status
    FROM currency_swap WHERE id = p_swap_id;
    
    -- Check swap exists and is pending
    IF v_status IS NULL THEN
        ROLLBACK;
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Swap not found';
    END IF;
    
    IF v_status != 'pending' THEN
        ROLLBACK;
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Swap is not pending';
    END IF;
    
    -- Check maker account exists
    IF v_maker_account_id IS NULL THEN
        ROLLBACK;
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Maker account not found';
    END IF;
    
    -- Refund maker's balance (only the maker had their balance deducted during swap creation)
    UPDATE account SET balance = balance + v_maker_amount WHERE id = v_maker_account_id;
    
    -- Update swap status to cancelled
    UPDATE currency_swap SET status = 'cancelled' WHERE id = p_swap_id;
    
    COMMIT;
END //

DELIMITER ;

-- PROCEDURE: sp_complete_swap
-- Marks a swap as completed (optional, currently unused)
DELIMITER //

DROP PROCEDURE IF EXISTS sp_complete_swap //

CREATE PROCEDURE sp_complete_swap(
    IN p_swap_id BIGINT
)
BEGIN
    UPDATE currency_swap SET status = 'completed' WHERE id = p_swap_id;
END //

DELIMITER ;

-- PROCEDURE: sp_get_swap
-- Retrieves swap details by ID
DELIMITER //

DROP PROCEDURE IF EXISTS sp_get_swap //

CREATE PROCEDURE sp_get_swap(
    IN p_swap_id BIGINT
)
BEGIN
    SELECT CAST(id AS SIGNED), CAST(maker_id AS SIGNED), CAST(taker_id AS SIGNED), 
           CAST(maker_currency_id AS SIGNED), CAST(taker_currency_id AS SIGNED), 
           CAST(maker_amount AS DOUBLE), CAST(taker_amount AS DOUBLE), status 
    FROM currency_swap WHERE id = p_swap_id;
END //

DELIMITER ;

-- PROCEDURE: sp_get_pending_swaps_by_maker
-- Retrieves all pending swaps created by a maker
DELIMITER //

DROP PROCEDURE IF EXISTS sp_get_pending_swaps_by_maker //

CREATE PROCEDURE sp_get_pending_swaps_by_maker(
    IN p_maker_account_id BIGINT
)
BEGIN
    SELECT CAST(id AS SIGNED), CAST(maker_id AS SIGNED), CAST(taker_id AS SIGNED), 
           CAST(maker_currency_id AS SIGNED), CAST(taker_currency_id AS SIGNED), 
           CAST(maker_amount AS DOUBLE), CAST(taker_amount AS DOUBLE), status 
    FROM currency_swap WHERE maker_id = p_maker_account_id AND status = 'pending'
    ORDER BY date_created DESC;
END //

DELIMITER ;

-- PROCEDURE: sp_get_swaps_by_maker
-- Retrieves all swaps (any status) created by a maker
DELIMITER //

DROP PROCEDURE IF EXISTS sp_get_swaps_by_maker //

CREATE PROCEDURE sp_get_swaps_by_maker(
    IN p_maker_account_id BIGINT
)
BEGIN
    SELECT CAST(id AS SIGNED), CAST(maker_id AS SIGNED), CAST(taker_id AS SIGNED), 
           CAST(maker_currency_id AS SIGNED), CAST(taker_currency_id AS SIGNED), 
           CAST(maker_amount AS DOUBLE), CAST(taker_amount AS DOUBLE), status 
    FROM currency_swap WHERE maker_id = p_maker_account_id
    ORDER BY date_created DESC;
END //

DELIMITER ;

-- PROCEDURE: sp_get_swaps_by_taker
-- Retrieves all swaps (any status) where user is the taker
DELIMITER //

DROP PROCEDURE IF EXISTS sp_get_swaps_by_taker //

CREATE PROCEDURE sp_get_swaps_by_taker(
    IN p_taker_account_id BIGINT
)
BEGIN
    SELECT CAST(id AS SIGNED), CAST(maker_id AS SIGNED), CAST(taker_id AS SIGNED), 
           CAST(maker_currency_id AS SIGNED), CAST(taker_currency_id AS SIGNED), 
           CAST(maker_amount AS DOUBLE), CAST(taker_amount AS DOUBLE), status 
    FROM currency_swap WHERE taker_id = p_taker_account_id
    ORDER BY date_created DESC;
END //

DELIMITER ;

-- PROCEDURE: sp_get_all_pending_swaps
-- Retrieves all pending swaps (admin view)
DELIMITER //

DROP PROCEDURE IF EXISTS sp_get_all_pending_swaps //

CREATE PROCEDURE sp_get_all_pending_swaps()
BEGIN
    SELECT CAST(id AS SIGNED), CAST(maker_id AS SIGNED), CAST(taker_id AS SIGNED), 
           CAST(maker_currency_id AS SIGNED), CAST(taker_currency_id AS SIGNED), 
           CAST(maker_amount AS DOUBLE), CAST(taker_amount AS DOUBLE), status 
    FROM currency_swap WHERE status = 'pending'
    ORDER BY date_created DESC;
END //

DELIMITER ;

-- PROCEDURE: sp_get_all_open_swaps
-- Retrieves all open swaps (swaps without a designated taker)
DELIMITER //

DROP PROCEDURE IF EXISTS sp_get_all_open_swaps //

CREATE PROCEDURE sp_get_all_open_swaps()
BEGIN
    SELECT CAST(id AS SIGNED), CAST(maker_id AS SIGNED), CAST(taker_id AS SIGNED), 
           CAST(maker_currency_id AS SIGNED), CAST(taker_currency_id AS SIGNED), 
           CAST(maker_amount AS DOUBLE), CAST(taker_amount AS DOUBLE), status 
    FROM currency_swap WHERE taker_id IS NULL AND status = 'pending'
    ORDER BY date_created DESC;
END //

DELIMITER ;
