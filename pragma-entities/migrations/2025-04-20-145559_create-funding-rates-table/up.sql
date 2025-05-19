-- Your SQL goes here

CREATE TABLE funding_rates (
    id uuid DEFAULT uuid_generate_v4(),
    source VARCHAR NOT NULL,
    pair VARCHAR NOT NULL,
    annualized_rate DOUBLE PRECISION NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id, timestamp)
);

-- Convert the table to a hypertable
SELECT create_hypertable('funding_rates', 'timestamp', chunk_time_interval => INTERVAL '1 day');

-- Create an index for efficient querying by pair
CREATE INDEX idx_funding_rates_pair ON funding_rates(pair);
