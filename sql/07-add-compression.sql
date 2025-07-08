CREATE FUNCTION add_compression_to_onchain_aggregates()
RETURNS void AS $$
DECLARE
    median_views text[] := ARRAY[
        'mainnet_spot_median_100_ms', 'mainnet_spot_median_1_s', 'mainnet_spot_median_5_s', 'mainnet_spot_median_10_s',
        'mainnet_spot_median_1_min', 'mainnet_spot_median_15_min', 'mainnet_spot_median_1_h', 'mainnet_spot_median_2_h',
        'mainnet_spot_median_1_day', 'mainnet_spot_median_1_week',
        'spot_median_100_ms', 'spot_median_1_s', 'spot_median_5_s', 'spot_median_10_s',
        'spot_median_1_min', 'spot_median_15_min', 'spot_median_1_h', 'spot_median_2_h',
        'spot_median_1_day', 'spot_median_1_week',
        'mainnet_perp_median_100_ms', 'mainnet_perp_median_1_s', 'mainnet_perp_median_5_s', 'mainnet_perp_median_10_s',
        'mainnet_perp_median_1_min', 'mainnet_perp_median_15_min', 'mainnet_perp_median_1_h', 'mainnet_perp_median_2_h',
        'mainnet_perp_median_1_day', 'mainnet_perp_median_1_week',
        'perp_median_100_ms', 'perp_median_1_s', 'perp_median_5_s', 'perp_median_10_s',
        'perp_median_1_min', 'perp_median_15_min', 'perp_median_1_h', 'perp_median_2_h',
        'perp_median_1_day', 'perp_median_1_week'
    ];
    twap_views text[] := ARRAY[
        'mainnet_spot_twap_1_min', 'mainnet_spot_twap_5_min', 'mainnet_spot_twap_15_min',
        'mainnet_spot_twap_1_h', 'mainnet_spot_twap_2_h', 'mainnet_spot_twap_1_day',
        'spot_twap_1_min', 'spot_twap_5_min', 'spot_twap_15_min',
        'spot_twap_1_h', 'spot_twap_2_h', 'spot_twap_1_day',
        'mainnet_perp_twap_1_min', 'mainnet_perp_twap_5_min', 'mainnet_perp_twap_15_min',
        'mainnet_perp_twap_1_h', 'mainnet_perp_twap_2_h', 'mainnet_perp_twap_1_day',
        'perp_twap_1_min', 'perp_twap_5_min', 'perp_twap_15_min',
        'perp_twap_1_h', 'perp_twap_2_h', 'perp_twap_1_day'
    ];
    candle_views text[] := ARRAY[
        'mainnet_spot_candle_10_s', 'mainnet_spot_candle_1_min', 'mainnet_spot_candle_5_min', 'mainnet_spot_candle_15_min',
        'mainnet_spot_candle_1_h', 'mainnet_spot_candle_1_day',
        'spot_candle_10_s', 'spot_candle_1_min', 'spot_candle_5_min', 'spot_candle_15_min',
        'spot_candle_1_h', 'spot_candle_1_day',
        'mainnet_perp_candle_10_s', 'mainnet_perp_candle_1_min', 'mainnet_perp_candle_5_min', 'mainnet_perp_candle_15_min',
        'mainnet_perp_candle_1_h', 'mainnet_perp_candle_1_day',
        'perp_candle_10_s', 'perp_candle_1_min', 'perp_candle_5_min', 'perp_candle_15_min',
        'perp_candle_1_h', 'perp_candle_1_day'
    ];
    view_to_compress text;
    compress_after interval;
BEGIN
    FOR view_to_compress IN
        SELECT view_n
        FROM (
            SELECT unnest(median_views) AS view_n
            UNION
            SELECT unnest(median_views) || '_per_source'
            UNION
            SELECT unnest(twap_views)
            UNION
            SELECT unnest(candle_views)
        ) AS all_views
    LOOP
        -- Perform operations sequentially within a transaction
        EXECUTE format('ALTER MATERIALIZED VIEW %I SET (timescaledb.enable_columnstore = true, timescaledb.segmentby = ''pair_id'')', view_to_compress);
        EXECUTE format('CALL add_columnstore_policy(%L, after => INTERVAL ''1 day'')', view_to_compress);
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Execute the compression function
SELECT add_compression_to_onchain_aggregates();
