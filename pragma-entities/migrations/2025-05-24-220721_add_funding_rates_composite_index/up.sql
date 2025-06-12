-- Your SQL goes here

-- Optimized composite index for funding_rates instruments queries
-- This index will significantly speed up MIN/MAX aggregations grouped by pair and source
CREATE INDEX IF NOT EXISTS idx_funding_rates_pair_source_timestamp 
ON funding_rates (pair, source, timestamp DESC);

-- Additional index for source queries
CREATE INDEX IF NOT EXISTS idx_funding_rates_source_timestamp 
ON funding_rates (source, timestamp DESC);
