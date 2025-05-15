-- This file should undo anything in `up.sql`

-- Remove compression policy first
SELECT remove_compression_policy('open_interest');

-- Disable compression
ALTER TABLE open_interest SET (timescaledb.compress = false);

-- Drop the hypertable
DROP TABLE open_interest CASCADE;
