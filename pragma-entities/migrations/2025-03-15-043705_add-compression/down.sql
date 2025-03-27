-- This file should undo anything in `up.sql`

-- Function to remove compression policies and disable columnstore from continuous aggregates
CREATE OR REPLACE FUNCTION remove_compression_from_continuous_aggregates()
RETURNS void AS $$
DECLARE
    view_name text;
BEGIN
    -- Loop over all relevant views, including _per_source variants where applicable
    FOR view_name IN 
        SELECT unnest(ARRAY[
            -- Sub-minute aggregates (main and _per_source where applicable)
            'median_100_ms_spot', 'median_100_ms_spot_per_source',
            'median_1_s_spot', 'median_1_s_spot_per_source',
            'median_5_s_spot', 'median_5_s_spot_per_source',
            'median_100_ms_perp', 'median_100_ms_perp_per_source',
            'median_1_s_perp', 'median_1_s_perp_per_source',
            'median_5_s_perp', 'median_5_s_perp_per_source',
            'candle_10_s_spot', 'candle_10_s_perp',

            -- 1-15min aggregates (main and _per_source where applicable)
            'median_1_min_spot', 'median_1_min_spot_per_source',
            'median_15_min_spot', 'median_15_min_spot_per_source',
            'median_1_min_perp', 'median_1_min_perp_per_source',
            'median_15_min_perp', 'median_15_min_perp_per_source',
            'candle_1_min_spot', 'candle_5_min_spot', 'candle_15_min_spot',
            'candle_1_min_perp', 'candle_5_min_perp', 'candle_15_min_perp',
            'twap_1_min_spot', 'twap_1_min_spot_per_source',
            'twap_5_min_spot', 'twap_5_min_spot_per_source',
            'twap_15_min_spot', 'twap_15_min_spot_per_source',
            'twap_1_min_perp', 'twap_1_min_perp_per_source',
            'twap_5_min_perp', 'twap_5_min_perp_per_source',
            'twap_15_min_perp', 'twap_15_min_perp_per_source',

            -- 1-2h aggregates (main and _per_source where applicable)
            'median_1_h_spot', 'median_1_h_spot_per_source',
            'median_2_h_spot', 'median_2_h_spot_per_source',
            'median_1_h_perp', 'median_1_h_perp_per_source',
            'median_2_h_perp', 'median_2_h_perp_per_source',
            'candle_1_h_spot', 'candle_1_h_perp',
            'twap_1_h_spot', 'twap_1_h_spot_per_source',
            'twap_2_h_spot', 'twap_2_h_spot_per_source',
            'twap_1_h_perp', 'twap_1_h_perp_per_source',
            'twap_2_h_perp', 'twap_2_h_perp_per_source',

            -- Daily aggregates (main and _per_source where applicable)
            'median_1_day_spot', 'median_1_day_spot_per_source',
            'median_1_day_perp', 'median_1_day_perp_per_source',
            'candle_1_day_spot', 'candle_1_day_perp',
            'twap_1_day_spot', 'twap_1_day_spot_per_source',
            'twap_1_day_perp', 'twap_1_day_perp_per_source',

            -- Weekly aggregates (main and _per_source where applicable)
            'median_1_week_spot', 'median_1_week_spot_per_source',
            'median_1_week_perp', 'median_1_week_perp_per_source'
        ])
    LOOP
        -- Remove compression policy if it exists, using a safer method
        EXECUTE format('SELECT remove_compression_policy(%L, if_exists => true)', view_name);

        -- Reset columnstore and segmentby settings to default
        EXECUTE format('ALTER MATERIALIZED VIEW %I RESET (timescaledb.enable_columnstore, timescaledb.segmentby)', view_name);
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Execute the function to apply the changes
SELECT remove_compression_from_continuous_aggregates();

-- Drop the function after execution
DROP FUNCTION remove_compression_from_continuous_aggregates();
DROP FUNCTION add_compression_to_continuous_aggregates();
