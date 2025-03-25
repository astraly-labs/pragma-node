CREATE OR REPLACE FUNCTION create_candlestick_view(
    p_name text,
    p_interval interval,
    p_start_offset interval,
    p_table_name text
)
RETURNS void AS $$
BEGIN
    -- Create the materialized view with continuous aggregate
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

    -- Set the chunk time interval to 7 days
    EXECUTE format('SELECT set_chunk_time_interval(%L, INTERVAL ''7 days'');', p_name);

    -- Add the continuous aggregate refresh policy
    EXECUTE format('
        SELECT add_continuous_aggregate_policy(%L,
            start_offset => %L,
            end_offset => %L,
            schedule_interval => %L);', p_name, p_start_offset, '0'::interval, p_interval);
END;
$$ LANGUAGE plpgsql;

-- Spot entries candlesticks
SELECT create_candlestick_view('candle_10_s_spot', '10 seconds'::interval, '30 seconds'::interval, 'median_1_s_spot');
SELECT create_candlestick_view('candle_1_min_spot', '1 minute'::interval, '3 minutes'::interval, 'median_1_s_spot');
SELECT create_candlestick_view('candle_5_min_spot', '5 minutes'::interval, '15 minutes'::interval, 'median_10_s_spot');
SELECT create_candlestick_view('candle_15_min_spot', '15 minutes'::interval, '45 minutes'::interval, 'median_10_s_spot');
SELECT create_candlestick_view('candle_1_h_spot', '1 hour'::interval, '3 hours'::interval, 'median_10_s_spot');
SELECT create_candlestick_view('candle_1_day_spot', '1 day'::interval, '3 days'::interval, 'median_10_s_spot');

-- Perp entries candlesticks
SELECT create_candlestick_view('candle_10_s_perp', '10 seconds'::interval, '30 seconds'::interval, 'median_1_s_perp');
SELECT create_candlestick_view('candle_1_min_perp', '1 minute'::interval, '3 minutes'::interval, 'median_1_s_perp');
SELECT create_candlestick_view('candle_5_min_perp', '5 minutes'::interval, '15 minutes'::interval, 'median_10_s_perp');
SELECT create_candlestick_view('candle_15_min_perp', '15 minutes'::interval, '45 minutes'::interval, 'median_10_s_perp');
SELECT create_candlestick_view('candle_1_h_perp', '1 hour'::interval, '3 hours'::interval, 'median_10_s_perp');
SELECT create_candlestick_view('candle_1_day_perp', '1 day'::interval, '3 days'::interval, 'median_10_s_perp');
