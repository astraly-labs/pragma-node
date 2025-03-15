CREATE OR REPLACE FUNCTION create_candlestick_view(
    p_name text,
    p_interval interval,
    p_start_offset interval,
    p_table_name text
)
RETURNS void AS $$
BEGIN
    EXECUTE format('
        CREATE MATERIALIZED VIEW %s
        WITH (timescaledb.continuous, timescaledb.materialized_only = false) AS
        SELECT
            time_bucket($1::interval, timestamp) AS bucket,
            pair_id,
            FIRST(price, timestamp)::numeric AS "open",
            MAX(price)::numeric AS high,
            MIN(price)::numeric AS low,
            LAST(price, timestamp)::numeric AS "close"
        FROM %I
        GROUP BY bucket, pair_id
        WITH NO DATA;', p_name, p_interval, p_table_name);

    EXECUTE format('
        SELECT add_continuous_aggregate_policy(''%s'',
            start_offset => $1,
            end_offset => $2,
            schedule_interval => $3);', p_name, p_start_offset, p_interval, p_interval);
END;
$$ LANGUAGE plpgsql;

-- Spot entries candlesticks
SELECT create_candlestick_view('candle_1_min', '1 minute'::interval, '3 minutes'::interval, 'price_1_s_agg');
SELECT create_candlestick_view('candle_5_min', '5 minutes'::interval, '15 minutes'::interval, 'price_10_s_agg');
SELECT create_candlestick_view('candle_15_min', '15 minutes'::interval, '45 minutes'::interval, 'price_10_s_agg');
SELECT create_candlestick_view('candle_1_h', '1 hour'::interval, '3 hours'::interval, 'price_10_s_agg');
SELECT create_candlestick_view('candle_1_day', '1 day'::interval, '3 days'::interval, 'price_10_s_agg');

-- Future entries candlesticks
SELECT create_candlestick_view('candle_1_min_future', '1 minute'::interval, '3 minutes'::interval, 'price_1_s_agg_future');
SELECT create_candlestick_view('candle_5_min_future', '5 minutes'::interval, '15 minutes'::interval, 'price_10_s_agg_future');
SELECT create_candlestick_view('candle_15_min_future', '15 minutes'::interval, '45 minutes'::interval, 'price_10_s_agg_future');
SELECT create_candlestick_view('candle_1_h_future', '1 hour'::interval, '3 hours'::interval, 'price_10_s_agg_future');
SELECT create_candlestick_view('candle_1_day_future', '1 day'::interval, '3 days'::interval, 'price_10_s_agg_future');