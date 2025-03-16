CREATE OR REPLACE FUNCTION create_candlestick_view(
    p_name text,
    p_interval interval,
    p_start_offset interval,
    p_table_name text
)
RETURNS void AS $$
BEGIN
    EXECUTE format('
        CREATE MATERIALIZED VIEW %I
        WITH (timescaledb.continuous, timescaledb.materialized_only = false) AS
        SELECT
            time_bucket(%L, bucket) AS ohlc_bucket,
            pair_id,
            FIRST(median_price, bucket)::numeric AS "open",
            MAX(median_price)::numeric AS high,
            MIN(median_price)::numeric AS low,
            LAST(median_price, bucket)::numeric AS "close"
        FROM %I
        GROUP BY ohlc_bucket, pair_id
        WITH NO DATA;', p_name, p_interval, p_table_name);

    EXECUTE format('
        SELECT add_continuous_aggregate_policy(%L,
            start_offset => %L,
            end_offset => %L,
            schedule_interval => %L);', p_name, p_start_offset, '0'::interval, p_interval);
END;
$$ LANGUAGE plpgsql;

-- Spot entries candlesticks
SELECT create_candlestick_view('candle_10_s', '10 seconds'::interval, '30 seconds'::interval, 'price_1_s_agg');
SELECT create_candlestick_view('candle_1_min', '1 minute'::interval, '3 minutes'::interval, 'price_1_s_agg');
SELECT create_candlestick_view('candle_5_min', '5 minutes'::interval, '15 minutes'::interval, 'price_10_s_agg');
SELECT create_candlestick_view('candle_15_min', '15 minutes'::interval, '45 minutes'::interval, 'price_10_s_agg');
SELECT create_candlestick_view('candle_1_h', '1 hour'::interval, '3 hours'::interval, 'price_10_s_agg');
SELECT create_candlestick_view('candle_1_day', '1 day'::interval, '3 days'::interval, 'price_10_s_agg');

-- Future entries candlesticks
SELECT create_candlestick_view('candle_10_s_future', '10 seconds'::interval, '30 seconds'::interval, 'price_1_s_agg_future');
SELECT create_candlestick_view('candle_1_min_future', '1 minute'::interval, '3 minutes'::interval, 'price_1_s_agg_future');
SELECT create_candlestick_view('candle_5_min_future', '5 minutes'::interval, '15 minutes'::interval, 'price_10_s_agg_future');
SELECT create_candlestick_view('candle_15_min_future', '15 minutes'::interval, '45 minutes'::interval, 'price_10_s_agg_future');
SELECT create_candlestick_view('candle_1_h_future', '1 hour'::interval, '3 hours'::interval, 'price_10_s_agg_future');
SELECT create_candlestick_view('candle_1_day_future', '1 day'::interval, '3 days'::interval, 'price_10_s_agg_future');