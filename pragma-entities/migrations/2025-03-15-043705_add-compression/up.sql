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
        SELECT unnest(ARRAY[
            'median_100_ms_spot', 'median_1_s_spot', 'median_5_s_spot',
            'median_100_ms_perp', 'median_1_s_perp', 'median_5_s_perp',
            'candle_10_s_spot', 'candle_10_s_perp',
            'median_1_min_spot', 'median_15_min_spot',
            'median_1_min_perp', 'median_15_min_perp',
            'candle_1_min_spot', 'candle_5_min_spot', 'candle_15_min_spot',
            'candle_1_min_perp', 'candle_5_min_perp', 'candle_15_min_perp',
            'twap_1_min_spot', 'twap_5_min_spot', 'twap_15_min_spot',
            'twap_1_min_perp', 'twap_5_min_perp', 'twap_15_min_perp',
            'median_1_h_spot', 'median_2_h_spot',
            'median_1_h_perp', 'median_2_h_perp',
            'candle_1_h_spot', 'candle_1_h_perp',
            'twap_1_h_spot', 'twap_2_h_spot',
            'twap_1_h_perp', 'twap_2_h_perp',
            'median_1_day_spot', 'median_1_day_perp',
            'candle_1_day_spot', 'candle_1_day_perp',
            'twap_1_day_spot', 'twap_1_day_perp',
            'median_1_week_spot', 'median_1_week_perp'
        ])
    LOOP
        -- Set compression interval based on view name pattern
        compress_after := 
            CASE 
                WHEN view_name LIKE '%100_ms%' OR view_name LIKE '%s%' THEN INTERVAL '1 hour'
                WHEN view_name LIKE '%min%' THEN INTERVAL '6 hours'
                WHEN view_name LIKE '%h%' OR view_name LIKE '%2_h%' THEN INTERVAL '1 day'
                WHEN view_name LIKE '%day%' THEN INTERVAL '7 days'
                WHEN view_name LIKE '%week%' THEN INTERVAL '30 days'
            END;

        -- Enable columnstore and set segmentby for each view
        EXECUTE format('ALTER MATERIALIZED VIEW %I SET (timescaledb.enable_columnstore = true, timescaledb.segmentby = ''pair_id'')', view_name);

        -- Add compression policy
        EXECUTE format('CALL add_columnstore_policy(%L, after => $1)', view_name) USING compress_after;

    END LOOP;
END;
$$ LANGUAGE plpgsql;


-- Add compression policies to all continuous aggregates
SELECT add_compression_to_continuous_aggregates();
