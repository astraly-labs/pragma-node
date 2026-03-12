-- +goose Up
CREATE TABLE IF NOT EXISTS trades_v2 (
    id UUID,
    market_id String,
    instrument_type String,
    pair_id String,
    price String,
    size String,
    side String,
    exchange_timestamp DateTime64(3, 'UTC'),
    received_timestamp DateTime64(3, 'UTC'),
    source String,
    buyer_address String,
    seller_address String
) ENGINE = ReplacingMergeTree()
ORDER BY (market_id, source, exchange_timestamp, id)
PARTITION BY toYYYYMM(exchange_timestamp)
TTL exchange_timestamp + INTERVAL 14 DAY;

-- +goose Down
DROP TABLE IF EXISTS trades_v2;
