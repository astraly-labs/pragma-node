-- +goose Up
-- Create prices table for spot price entries
-- Engine: ReplacingMergeTree for automatic deduplication
-- Partitioning: Monthly partitions by timestamp

CREATE TABLE IF NOT EXISTS prices (
    id UUID,
    pair_id String,
    price String,
    timestamp DateTime('UTC'),
    source String
) ENGINE = ReplacingMergeTree()
ORDER BY (pair_id, source, timestamp, id)
PARTITION BY toYYYYMM(timestamp);

-- +goose Down
DROP TABLE IF EXISTS prices;
