-- Your SQL goes here
CREATE INDEX IF NOT EXISTS idx_funding_rates_pair_source_ts ON funding_rates (pair, source, timestamp);
CREATE INDEX IF NOT EXISTS idx_open_interest_pair_source_ts ON open_interest (pair, source, timestamp);
