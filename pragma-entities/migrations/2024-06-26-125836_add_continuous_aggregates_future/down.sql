-- This file should undo anything in `up.sql`
DROP MATERIALIZED VIEW IF EXISTS price_1_min_agg_future;
DROP MATERIALIZED VIEW IF EXISTS price_15_min_agg_future;
DROP MATERIALIZED VIEW IF EXISTS price_1_h_agg_future;
DROP MATERIALIZED VIEW IF EXISTS price_2_h_agg_future;