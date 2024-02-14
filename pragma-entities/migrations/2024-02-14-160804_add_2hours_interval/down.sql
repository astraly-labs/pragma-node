-- This file should undo anything in `up.sql`
DROP MATERIALIZED VIEW IF EXISTS twap_2_hours_agg;
DROP MATERIALIZED VIEW IF EXISTS price_2_h_agg;
DROP MATERIALIZED VIEW IF EXISTS two_hour_candle;