-- This file should undo anything in `up.sql`

-- Function to remove compression policies from continuous aggregates
CREATE OR REPLACE FUNCTION remove_compression_from_continuous_aggregates()
RETURNS void AS $$
DECLARE
    view_name text;
BEGIN
    FOR view_name IN 
        SELECT format('%s', unnest(ARRAY[
            -- Sub-minute aggregates
            'median_100_ms_spot', 'median_1_s_spot', 'median_5_s_spot',
            'median_100_ms_perp', 'median_1_s_perp', 'median_5_s_perp',
            'candle_10_s_spot', 'candle_10_s_perp',

            -- 1-15min aggregates
            'median_1_min_spot', 'median_15_min_spot',
            'median_1_min_perp', 'median_15_min_perp',
            'candle_1_min_spot', 'candle_5_min_spot', 'candle_15_min_spot',
            'candle_1_min_perp', 'candle_5_min_perp', 'candle_15_min_perp',
            'twap_1_min_spot', 'twap_5_min_spot', 'twap_15_min_spot',
            'twap_1_min_perp', 'twap_5_min_perp', 'twap_15_min_perp',

            -- 1-2h aggregates
            'median_1_h_spot', 'median_2_h_spot',
            'median_1_h_perp', 'median_2_h_perp',
            'candle_1_h_spot', 'candle_1_h_perp',
            'twap_1_h_spot', 'twap_2_h_spot',
            'twap_1_h_perp', 'twap_2_h_perp',

            -- Daily aggregates
            'median_1_day_spot', 'median_1_day_perp',
            'candle_1_day_spot', 'candle_1_day_perp',
            'twap_1_day_spot', 'twap_1_day_perp',

            -- Weekly aggregates
            'median_1_week_spot', 'median_1_week_perp'
        ]))
    LOOP
        BEGIN
            -- Remove compression policy if it exists
            CALL remove_columnstore_policy(view_name);
        EXCEPTION WHEN OTHERS THEN
            -- Skip if policy doesn't exist
            RAISE NOTICE 'No compression policy found for %', view_name;
        END;
        
        -- Disable columnstore
        EXECUTE format('ALTER MATERIALIZED VIEW %I SET (timescaledb.enable_columnstore = false)', view_name);
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Remove compression policies from all continuous aggregates
SELECT remove_compression_from_continuous_aggregates();

-- Drop the functions
DROP FUNCTION IF EXISTS add_compression_to_continuous_aggregates;
DROP FUNCTION IF EXISTS remove_compression_from_continuous_aggregates;

CALL remove_columnstore_policy('entries');
CALL remove_columnstore_policy('future_entries');

-- Remove compression from base tables
ALTER TABLE entries SET (timescaledb.enable_columnstore = false);
ALTER TABLE future_entries SET (timescaledb.enable_columnstore = false);
