-- This file should undo anything in `up.sql`
DROP MATERIALIZED VIEW IF EXISTS one_day_candle;
DROP MATERIALIZED VIEW IF EXISTS one_hour_candle;
DROP MATERIALIZED VIEW IF EXISTS fifteen_minute_candle;
DROP MATERIALIZED VIEW IF EXISTS five_minute_candle;
DROP MATERIALIZED VIEW IF EXISTS one_minute_candle;