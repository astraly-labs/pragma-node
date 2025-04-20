-- Your SQL goes here

CREATE TABLE funding_rates (
    id SERIAL,
    source VARCHAR NOT NULL,
    pair VARCHAR NOT NULL,
    annualized_rate DOUBLE PRECISION NOT NULL,
    timestamp_ms BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source, pair, timestamp_ms)
);

-- Convert the table to a hypertable
SELECT create_hypertable('funding_rates', 'timestamp_ms', chunk_time_interval => INTERVAL '1 day');

-- Create an index for efficient querying by pair
CREATE INDEX idx_funding_rates_pair ON funding_rates(pair);

-- Enable compression
ALTER TABLE funding_rates SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'source,pair'
);

-- Add compression policy to compress chunks older than 7 days
SELECT add_compression_policy('funding_rates', INTERVAL '7 days');
