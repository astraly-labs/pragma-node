-- This file should undo anything in `up.sql`

-- Drop spot TWAP views
DROP MATERIALIZED VIEW IF EXISTS twap_1_min_agg;
DROP MATERIALIZED VIEW IF EXISTS twap_5_min_agg;
DROP MATERIALIZED VIEW IF EXISTS twap_15_min_agg;
DROP MATERIALIZED VIEW IF EXISTS twap_1_h_agg;
DROP MATERIALIZED VIEW IF EXISTS twap_2_h_agg;
DROP MATERIALIZED VIEW IF EXISTS twap_1_day_agg;

-- Drop future TWAP views
DROP MATERIALIZED VIEW IF EXISTS twap_1_min_agg_future;
DROP MATERIALIZED VIEW IF EXISTS twap_5_min_agg_future;
DROP MATERIALIZED VIEW IF EXISTS twap_15_min_agg_future;
DROP MATERIALIZED VIEW IF EXISTS twap_1_h_agg_future;
DROP MATERIALIZED VIEW IF EXISTS twap_2_h_agg_future;
DROP MATERIALIZED VIEW IF EXISTS twap_1_day_agg_future;

-- Drop the function
DROP FUNCTION IF EXISTS create_twap_aggregate;