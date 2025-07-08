-- Add compression to all continuous aggregates, including _per_source sub-tables
CREATE OR REPLACE FUNCTION add_compression_to_continuous_aggregates()
RETURNS void AS $$
DECLARE
    -- Define arrays for each type of view
    median_views text[] := ARRAY[
        'median_100_ms_spot', 'median_1_s_spot', 'median_5_s_spot', 'median_10_s_spot',
        'median_1_min_spot', 'median_15_min_spot', 'median_1_h_spot', 'median_2_h_spot',
        'median_1_day_spot', 'median_1_week_spot',
        'median_100_ms_perp', 'median_1_s_perp', 'median_5_s_perp', 'median_10_s_perp',
        'median_1_min_perp', 'median_15_min_perp', 'median_1_h_perp', 'median_2_h_perp',
        'median_1_day_perp', 'median_1_week_perp'
    ];
    twap_views text[] := ARRAY[
        'twap_1_min_spot', 'twap_5_min_spot', 'twap_15_min_spot',
        'twap_1_h_spot', 'twap_2_h_spot', 'twap_1_day_spot',
        'twap_1_min_perp', 'twap_5_min_perp', 'twap_15_min_perp',
        'twap_1_h_perp', 'twap_2_h_perp', 'twap_1_day_perp'
    ];
    candle_views text[] := ARRAY[
        'candle_10_s_spot', 'candle_1_min_spot', 'candle_5_min_spot', 'candle_15_min_spot',
        'candle_1_h_spot', 'candle_1_day_spot',
        'candle_10_s_perp', 'candle_1_min_perp', 'candle_5_min_perp', 'candle_15_min_perp',
        'candle_1_h_perp', 'candle_1_day_perp'
    ];
    view_to_compress text;
    compress_after interval;
BEGIN
    -- Loop over all views: main median, median _per_source, main twap, twap _per_source, and candle views
    FOR view_to_compress IN
        SELECT view_n
        FROM (
            -- Main median views
            SELECT unnest(median_views) AS view_n
            UNION
            -- Median _per_source sub-tables
            SELECT unnest(median_views) || '_per_source'
            UNION
            -- Main TWAP views
            SELECT unnest(twap_views)
            UNION
            -- TWAP _per_source sub-tables
            SELECT unnest(twap_views) || '_per_source'
            UNION
            -- Candlestick views (no _per_source sub-tables)
            SELECT unnest(candle_views)
        ) AS all_views
    LOOP
        -- Determine compress_after interval based on view name pattern
        compress_after := 
            CASE 
                WHEN view_to_compress LIKE '%100_ms%' OR view_to_compress LIKE '%s%' THEN INTERVAL '1 hour'
                WHEN view_to_compress LIKE '%min%' THEN INTERVAL '6 hours'
                WHEN view_to_compress LIKE '%h%' OR view_to_compress LIKE '%2_h%' THEN INTERVAL '1 day'
                WHEN view_to_compress LIKE '%day%' THEN INTERVAL '7 days'
                WHEN view_to_compress LIKE '%week%' THEN INTERVAL '30 days'
            END;

        -- Enable columnstore compression and set segmentby
        EXECUTE format('ALTER MATERIALIZED VIEW %I SET (timescaledb.enable_columnstore = true, timescaledb.segmentby = ''pair_id'')', view_to_compress);

        -- Add compression policy
        EXECUTE format('CALL add_columnstore_policy(%L, after => $1)', view_to_compress) USING compress_after;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Execute the function to apply compression policies
SELECT add_compression_to_continuous_aggregates();
