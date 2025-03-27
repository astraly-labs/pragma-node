-- Drop materialized views for spot candlesticks
DROP MATERIALIZED VIEW candle_10_s_spot;
DROP MATERIALIZED VIEW candle_1_min_spot;
DROP MATERIALIZED VIEW candle_5_min_spot;
DROP MATERIALIZED VIEW candle_15_min_spot;
DROP MATERIALIZED VIEW candle_1_h_spot;
DROP MATERIALIZED VIEW candle_1_day_spot;

-- Drop materialized views for perp candlesticks
DROP MATERIALIZED VIEW candle_10_s_perp;
DROP MATERIALIZED VIEW candle_1_min_perp;
DROP MATERIALIZED VIEW candle_5_min_perp;
DROP MATERIALIZED VIEW candle_15_min_perp;
DROP MATERIALIZED VIEW candle_1_h_perp;
DROP MATERIALIZED VIEW candle_1_day_perp;

-- Drop the function
DROP FUNCTION create_candlestick_view(text, interval, interval, text);
