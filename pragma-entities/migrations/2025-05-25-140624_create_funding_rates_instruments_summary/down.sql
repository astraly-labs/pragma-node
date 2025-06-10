-- This file should undo anything in `up.sql`

-- Drop the refresh function
DROP FUNCTION IF EXISTS refresh_funding_rates_instruments_summary();

-- Drop the index
DROP INDEX IF EXISTS idx_funding_rates_instruments_summary_pair_source;

-- Drop the materialized view
DROP MATERIALIZED VIEW IF EXISTS funding_rates_instruments_summary;
