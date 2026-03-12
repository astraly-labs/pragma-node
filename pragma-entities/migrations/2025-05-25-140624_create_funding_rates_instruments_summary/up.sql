-- Your SQL goes here

-- Create a regular materialized view for funding rates instruments summary
-- This pre-computes MIN/MAX timestamps per pair/source combination for much faster queries
-- We use a regular materialized view instead of continuous aggregate since we want 
-- overall MIN/MAX per pair/source, not time-bucketed aggregations
CREATE MATERIALIZED VIEW IF NOT EXISTS funding_rates_instruments_summary AS
SELECT
    pair,
    source,
    MIN(timestamp) AS first_ts,
    MAX(timestamp) AS last_ts
FROM funding_rates
GROUP BY pair, source;

-- Create an index on the materialized view for optimal query performance
CREATE INDEX IF NOT EXISTS idx_funding_rates_instruments_summary_pair_source 
ON funding_rates_instruments_summary (pair, source);

-- Create a function to refresh the materialized view
CREATE OR REPLACE FUNCTION refresh_funding_rates_instruments_summary()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW funding_rates_instruments_summary;
END;
$$ LANGUAGE plpgsql;
-- Optionally, you can set up a scheduled job to refresh this view periodically
-- This would typically be done outside of the migration, but here's an example:
-- SELECT cron.schedule('refresh-funding-rates-summary', '*/10 * * * *', 'SELECT refresh_funding_rates_instruments_summary();');

