-- This file should undo anything in `up.sql`
DROP MATERIALIZED VIEW IF EXISTS twap_1_min_agg;
DROP MATERIALIZED VIEW IF EXISTS twap_15_min_agg;
DROP MATERIALIZED VIEW IF EXISTS twap_1_hour_agg;