-- This file should undo anything in `up.sql`
DROP MATERIALIZED VIEW IF EXISTS twap_1_min_agg_future;
DROP MATERIALIZED VIEW IF EXISTS twap_15_min_agg_future;
DROP MATERIALIZED VIEW IF EXISTS twap_1_hour_agg_future;
DROP MATERIALIZED VIEW IF EXISTS twap_2_hours_agg_future;