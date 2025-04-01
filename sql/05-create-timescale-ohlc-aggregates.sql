CREATE FUNCTION create_onchain_candlestick_view(
    p_candle_name text,      -- e.g., 'mainnet_spot_candle_10_s'
    p_interval interval,     -- e.g., '10 seconds'
    p_start_offset interval, -- e.g., '30 seconds'
    p_median_view_name text  -- e.g., 'mainnet_spot_median_1_s'
)
RETURNS void AS $$
BEGIN
    -- Create the candlestick materialized view
    EXECUTE format('
        CREATE MATERIALIZED VIEW %I
        WITH (timescaledb.continuous, timescaledb.materialized_only = false) AS
        SELECT
            time_bucket(%L, subbucket) AS ohlc_bucket,
            pair_id,
            FIRST(source_median_price, subbucket)::numeric AS "open",
            MAX(source_median_price)::numeric AS high,
            MIN(source_median_price)::numeric AS low,
            LAST(source_median_price, subbucket)::numeric AS "close"
        FROM %I_per_source
        GROUP BY ohlc_bucket, pair_id
        WITH NO DATA;',
        p_candle_name, p_interval, p_median_view_name);

    -- Set chunk time interval to 7 days
    EXECUTE format('SELECT set_chunk_time_interval(%L, INTERVAL ''7 days'');', p_candle_name);

    -- Add continuous aggregate refresh policy
    EXECUTE format('
        SELECT add_continuous_aggregate_policy(%L,
            start_offset => %L,
            end_offset => %L,
            schedule_interval => %L);', p_candle_name, p_start_offset, '0'::interval, p_interval);
END;
$$ LANGUAGE plpgsql;

-- Mainnet Spot Candlesticks
SELECT create_onchain_candlestick_view('mainnet_spot_candle_5_min', '5 minutes'::interval, '15 minutes'::interval, 'mainnet_spot_median_10_s');
SELECT create_onchain_candlestick_view('mainnet_spot_candle_15_min', '15 minutes'::interval, '45 minutes'::interval, 'mainnet_spot_median_10_s');
SELECT create_onchain_candlestick_view('mainnet_spot_candle_1_h', '1 hour'::interval, '3 hours'::interval, 'mainnet_spot_median_10_s');
SELECT create_onchain_candlestick_view('mainnet_spot_candle_1_day', '1 day'::interval, '3 days'::interval, 'mainnet_spot_median_15_min');
SELECT create_onchain_candlestick_view('mainnet_spot_candle_1_week', '1 week'::interval, '3 weeks'::interval, 'mainnet_spot_median_1_h');

-- Testnet Spot Candlesticks
SELECT create_onchain_candlestick_view('spot_candle_5_min', '5 minutes'::interval, '15 minutes'::interval, 'spot_median_10_s');
SELECT create_onchain_candlestick_view('spot_candle_15_min', '15 minutes'::interval, '45 minutes'::interval, 'spot_median_10_s');
SELECT create_onchain_candlestick_view('spot_candle_1_h', '1 hour'::interval, '3 hours'::interval, 'spot_median_10_s');
SELECT create_onchain_candlestick_view('spot_candle_1_day', '1 day'::interval, '3 days'::interval, 'spot_median_15_min');
SELECT create_onchain_candlestick_view('spot_candle_1_week', '1 week'::interval, '3 weeks'::interval, 'spot_median_1_h');

-- Mainnet Perp Candlesticks
SELECT create_onchain_candlestick_view('mainnet_perp_candle_5_min', '5 minutes'::interval, '15 minutes'::interval, 'mainnet_perp_median_10_s');
SELECT create_onchain_candlestick_view('mainnet_perp_candle_15_min', '15 minutes'::interval, '45 minutes'::interval, 'mainnet_perp_median_10_s');
SELECT create_onchain_candlestick_view('mainnet_perp_candle_1_h', '1 hour'::interval, '3 hours'::interval, 'mainnet_perp_median_10_s');
SELECT create_onchain_candlestick_view('mainnet_perp_candle_1_day', '1 day'::interval, '3 days'::interval, 'mainnet_perp_median_15_min');
SELECT create_onchain_candlestick_view('mainnet_perp_candle_1_week', '1 week'::interval, '3 weeks'::interval, 'mainnet_perp_median_1_h');

-- Testnet Perp Candlesticks
SELECT create_onchain_candlestick_view('perp_candle_5_min', '5 minutes'::interval, '15 minutes'::interval, 'perp_median_10_s');
SELECT create_onchain_candlestick_view('perp_candle_15_min', '15 minutes'::interval, '45 minutes'::interval, 'perp_median_10_s');
SELECT create_onchain_candlestick_view('perp_candle_1_h', '1 hour'::interval, '3 hours'::interval, 'perp_median_10_s');
SELECT create_onchain_candlestick_view('perp_candle_1_day', '1 day'::interval, '3 days'::interval, 'perp_median_15_min');
SELECT create_onchain_candlestick_view('perp_candle_1_week', '1 week'::interval, '3 weeks'::interval, 'perp_median_1_h');
