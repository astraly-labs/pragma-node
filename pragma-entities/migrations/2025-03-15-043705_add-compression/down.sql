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
            'price_100_ms_agg', 'price_1_s_agg', 'price_5_s_agg',
            'price_100_ms_agg_future', 'price_1_s_agg_future', 'price_5_s_agg_future',
            'candle_10_s', 'candle_10_s_future',

            -- 1-15min aggregates
            'price_1_min_agg', 'price_15_min_agg',
            'price_1_min_agg_future', 'price_15_min_agg_future',
            'candle_1_min', 'candle_5_min', 'candle_15_min',
            'candle_1_min_future', 'candle_5_min_future', 'candle_15_min_future',
            'twap_1_min_agg', 'twap_5_min_agg', 'twap_15_min_agg',
            'twap_1_min_agg_future', 'twap_5_min_agg_future', 'twap_15_min_agg_future',

            -- 1-2h aggregates
            'price_1_h_agg', 'price_2_h_agg',
            'price_1_h_agg_future', 'price_2_h_agg_future',
            'candle_1_h', 'candle_1_h_future',
            'twap_1_h_agg', 'twap_2_h_agg',
            'twap_1_h_agg_future', 'twap_2_h_agg_future',

            -- Daily aggregates
            'price_1_day_agg', 'price_1_day_agg_future',
            'candle_1_day', 'candle_1_day_future',
            'twap_1_day_agg', 'twap_1_day_agg_future',

            -- Weekly aggregates
            'price_1_week_agg', 'price_1_week_agg_future'
        ]))
    LOOP
        -- Remove compression policy
        CALL remove_compression_policy(view_name, if_exists => true);
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

-- Remove compression from base tables
ALTER TABLE entries SET (timescaledb.enable_columnstore = false);
ALTER TABLE future_entries SET (timescaledb.enable_columnstore = false);
