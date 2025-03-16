-- This file should undo anything in `up.sql`

-- Drop spot candlestick views
DROP MATERIALIZED VIEW IF EXISTS candle_1_min;
DROP MATERIALIZED VIEW IF EXISTS candle_5_min;
DROP MATERIALIZED VIEW IF EXISTS candle_15_min;
DROP MATERIALIZED VIEW IF EXISTS candle_1_h;
DROP MATERIALIZED VIEW IF EXISTS candle_1_day;

-- Drop future candlestick views
DROP MATERIALIZED VIEW IF EXISTS candle_1_min_future;
DROP MATERIALIZED VIEW IF EXISTS candle_5_min_future;
DROP MATERIALIZED VIEW IF EXISTS candle_15_min_future;
DROP MATERIALIZED VIEW IF EXISTS candle_1_h_future;
DROP MATERIALIZED VIEW IF EXISTS candle_1_day_future;

-- Drop the function
DROP FUNCTION IF EXISTS create_candlestick_view;