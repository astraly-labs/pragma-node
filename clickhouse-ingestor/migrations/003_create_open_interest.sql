-- +goose Up
-- Create open_interest table for open interest entries
-- Engine: ReplacingMergeTree for automatic deduplication
-- Partitioning: Monthly partitions by timestamp

CREATE TABLE IF NOT EXISTS open_interest (
    id UUID,
    pair_id String,
    open_interest_value Float64,
    timestamp DateTime('UTC'),
    source String
) ENGINE = ReplacingMergeTree()
ORDER BY (pair_id, source, timestamp, id)
PARTITION BY toYYYYMM(timestamp);

-- +goose Down
DROP TABLE IF EXISTS open_interest;

