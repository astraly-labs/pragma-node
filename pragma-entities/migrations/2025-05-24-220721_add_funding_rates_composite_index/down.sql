-- This file should undo anything in `up.sql`

-- Drop the composite indexes
DROP INDEX IF EXISTS idx_funding_rates_pair_source_timestamp;
DROP INDEX IF EXISTS idx_funding_rates_source_timestamp;
