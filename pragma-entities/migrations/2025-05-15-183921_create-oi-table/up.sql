-- Your SQL goes here

CREATE TABLE open_interest (
    id uuid DEFAULT uuid_generate_v4(),
    source VARCHAR NOT NULL,
    pair VARCHAR NOT NULL,
    open_interest DOUBLE PRECISION NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id, timestamp)
);

-- Convert the table to a hypertable
SELECT create_hypertable('open_interest', 'timestamp', chunk_time_interval => INTERVAL '1 day');

-- Create an index for efficient querying by pair
CREATE INDEX idx_open_interest_pair ON open_interest(pair);

-- Enable compression
ALTER TABLE open_interest SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'source,pair'
);

-- Add compression policy to compress chunks older than 7 days
SELECT add_compression_policy('open_interest', INTERVAL '7 days');
