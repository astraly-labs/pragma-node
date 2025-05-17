-- This file should undo anything in `up.sql`

-- Remove compression policy first
SELECT remove_compression_policy('funding_rates');

-- Disable compression
ALTER TABLE funding_rates SET (timescaledb.compress = false);

-- Drop the hypertable
DROP TABLE funding_rates CASCADE;
