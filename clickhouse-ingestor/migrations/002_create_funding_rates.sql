-- +goose Up
-- Create funding_rates table for funding rate entries
-- Engine: ReplacingMergeTree for automatic deduplication
-- Partitioning: Monthly partitions by timestamp

CREATE TABLE IF NOT EXISTS funding_rates (
    id UUID,
    pair_id String,
    annualized_rate Float64,
    timestamp DateTime('UTC'),
    source String
) ENGINE = ReplacingMergeTree()
ORDER BY (pair_id, source, timestamp, id)
PARTITION BY toYYYYMM(timestamp);

-- +goose Down
DROP TABLE IF EXISTS funding_rates;

