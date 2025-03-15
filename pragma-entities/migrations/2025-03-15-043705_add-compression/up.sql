-- Your SQL goes here

-- Enable hypercore
ALTER TABLE entries SET (
   timescaledb.enable_columnstore = true, 
   timescaledb.segmentby = 'pair_id');

ALTER TABLE future_entries SET (
  timescaledb.enable_columnstore = true,
  timescaledb.segmentby = 'pair_id'
);

-- Add compression policies
CALL add_columnstore_policy('entries', after => INTERVAL '1d');
CALL add_columnstore_policy('future_entries', after => INTERVAL '1d');

-- Function to add columnstore policies to continuous aggregates
CREATE OR REPLACE FUNCTION add_compression_to_continuous_aggregates()
RETURNS void AS $$
DECLARE
    view_name text;
    compress_after interval;
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
        -- Check if view exists
        IF NOT EXISTS (SELECT 1 FROM pg_matviews WHERE matviewname = view_name) THEN
            RAISE NOTICE 'View % does not exist, skipping...', view_name;
            CONTINUE;
        END IF;

        -- Set compression interval based on view name pattern
        compress_after := 
            CASE 
                WHEN view_name LIKE '%100_ms%' OR view_name LIKE '%_s_%' THEN INTERVAL '1 hour'
                WHEN view_name LIKE '%min%' THEN INTERVAL '6 hours'
                WHEN view_name LIKE '%_h_%' OR view_name LIKE '%_2_h%' THEN INTERVAL '1 day'
                WHEN view_name LIKE '%day%' THEN INTERVAL '7 days'
                WHEN view_name LIKE '%week%' THEN INTERVAL '30 days'
            END;

        BEGIN
            -- Enable columnstore and set segmentby for each view
            EXECUTE format('ALTER MATERIALIZED VIEW %I SET (timescaledb.enable_columnstore = true, timescaledb.segmentby = ''pair_id'')', view_name);
            -- Add compression policy
            EXECUTE format('CALL add_columnstore_policy(%L, after => $1, if_not_exists => true)', view_name) USING compress_after;
        EXCEPTION
            WHEN OTHERS THEN
                RAISE WARNING 'Failed to add compression policy for view %: %', view_name, SQLERRM;
        END;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Add compression policies to all continuous aggregates
SELECT add_compression_to_continuous_aggregates();
