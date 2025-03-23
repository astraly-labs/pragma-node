-- This file should undo anything in `up.sql`

-- Drop spot price aggregates
DROP MATERIALIZED VIEW IF EXISTS price_100_ms_agg;
DROP MATERIALIZED VIEW IF EXISTS price_1_s_agg;
DROP MATERIALIZED VIEW IF EXISTS price_5_s_agg;
DROP MATERIALIZED VIEW IF EXISTS price_1_min_agg;
DROP MATERIALIZED VIEW IF EXISTS price_15_min_agg;
DROP MATERIALIZED VIEW IF EXISTS price_1_h_agg;
DROP MATERIALIZED VIEW IF EXISTS price_2_h_agg;
DROP MATERIALIZED VIEW IF EXISTS price_1_day_agg;
DROP MATERIALIZED VIEW IF EXISTS price_1_week_agg;

-- Drop future price aggregates
DROP MATERIALIZED VIEW IF EXISTS price_100_ms_agg_future;
DROP MATERIALIZED VIEW IF EXISTS price_1_s_agg_future;
DROP MATERIALIZED VIEW IF EXISTS price_5_s_agg_future;
DROP MATERIALIZED VIEW IF EXISTS price_1_min_agg_future;
DROP MATERIALIZED VIEW IF EXISTS price_15_min_agg_future;
DROP MATERIALIZED VIEW IF EXISTS price_1_h_agg_future;
DROP MATERIALIZED VIEW IF EXISTS price_2_h_agg_future;
DROP MATERIALIZED VIEW IF EXISTS price_1_day_agg_future;
DROP MATERIALIZED VIEW IF EXISTS price_1_week_agg_future;

-- Drop the function
DROP FUNCTION IF EXISTS create_continuous_aggregate;

-- Drop the type
DROP TYPE IF EXISTS price_component;