-- A price component - it represents a sub price that has been used to compute a price.
-- For example, a price that has been used to compute a median for an ID.
CREATE TYPE price_component AS (
    source text,
    price numeric(1000,0),
    "timestamp" timestamptz
);

-- ===============================
-- Function used to create a median continuous aggregate 
-- ===============================
CREATE OR REPLACE FUNCTION create_median_aggregate(
    p_name text,
    p_interval interval,
    p_start_offset interval,
    p_type text -- 'spot' or 'perp'
)
RETURNS void AS $$
DECLARE
    table_name text;
    where_condition text;
BEGIN
    -- Set the table and WHERE condition based on p_type
    IF p_type = 'spot' THEN
        table_name := 'entries';
        where_condition := '"timestamp" IS NOT NULL';
    ELSIF p_type = 'perp' THEN
        table_name := 'future_entries';
        where_condition := '"timestamp" IS NOT NULL AND expiration_timestamp IS NULL';
    ELSE
        RAISE EXCEPTION 'Invalid type: %', p_type;
    END IF;

    -- Create the sub materialized view that contains the median price per source
    EXECUTE format('
        CREATE MATERIALIZED VIEW %s_per_source
        WITH (timescaledb.continuous, timescaledb.materialized_only = false)
        AS SELECT
            pair_id,
            source,
            time_bucket(%L, "timestamp") AS subbucket,
            percentile_cont(0.5) WITHIN GROUP (ORDER BY price)::numeric(1000,0) AS source_median_price
        FROM %I
        WHERE %s
        GROUP BY pair_id, source, subbucket
        WITH NO DATA;',
    p_name, p_interval, table_name, where_condition);

    -- Create the materialized view that contains the median price across all sources
    EXECUTE format('
        CREATE MATERIALIZED VIEW %I
        WITH (timescaledb.continuous, timescaledb.materialized_only = false)
        AS SELECT
            pair_id,
            time_bucket(%L, subbucket) AS bucket,
            percentile_cont(0.5) WITHIN GROUP (ORDER BY source_median_price)::numeric(1000,0) AS median_price,
            COUNT(DISTINCT source) AS num_sources,
            array_agg(ROW(source, source_median_price, subbucket)::price_component) AS components
        FROM %I
        GROUP BY pair_id, bucket
        WITH NO DATA;',
    p_name, p_interval, p_name || '_per_source');

    -- Set the chunk time interval to 12 hours
    EXECUTE format('SELECT set_chunk_time_interval(%L, INTERVAL ''12 hours'');', p_name || '_per_source');
    EXECUTE format('SELECT set_chunk_time_interval(%L, INTERVAL ''12 hours'');', p_name);

    -- Add the continuous aggregate refresh policy
    EXECUTE format('
        SELECT add_continuous_aggregate_policy(%L,
            start_offset => %L,
            end_offset => %L,
            schedule_interval => %L);',
    p_name || '_per_source', p_start_offset, '0'::interval, p_interval);
    EXECUTE format('
        SELECT add_continuous_aggregate_policy(%L,
            start_offset => %L,
            end_offset => %L,
            schedule_interval => %L);',
    p_name, p_start_offset, '0'::interval, p_interval);
END;
$$ LANGUAGE plpgsql;

-- SPOT median
SELECT create_median_aggregate('median_100_ms_spot', '100 milliseconds'::interval, '300 milliseconds'::interval, 'spot');
SELECT create_median_aggregate('median_1_s_spot', '1 second'::interval, '3 seconds'::interval, 'spot');
SELECT create_median_aggregate('median_5_s_spot', '5 seconds'::interval, '15 seconds'::interval, 'spot');
SELECT create_median_aggregate('median_10_s_spot', '10 seconds'::interval, '30 seconds'::interval, 'spot');
SELECT create_median_aggregate('median_1_min_spot', '1 minute'::interval, '3 minutes'::interval, 'spot');
SELECT create_median_aggregate('median_15_min_spot', '15 minutes'::interval, '45 minutes'::interval, 'spot');
SELECT create_median_aggregate('median_1_h_spot', '1 hour'::interval, '3 hours'::interval, 'spot');
SELECT create_median_aggregate('median_2_h_spot', '2 hours'::interval, '6 hours'::interval, 'spot');
SELECT create_median_aggregate('median_1_day_spot', '1 day'::interval, '3 days'::interval, 'spot');
SELECT create_median_aggregate('median_1_week_spot', '1 week'::interval, '3 weeks'::interval, 'spot');

-- PERP median
SELECT create_median_aggregate('median_100_ms_perp', '100 milliseconds'::interval, '300 milliseconds'::interval, 'perp');
SELECT create_median_aggregate('median_1_s_perp', '1 second'::interval, '3 seconds'::interval, 'perp');
SELECT create_median_aggregate('median_5_s_perp', '5 seconds'::interval, '15 seconds'::interval, 'perp');
SELECT create_median_aggregate('median_10_s_perp', '10 seconds'::interval, '30 seconds'::interval, 'perp');
SELECT create_median_aggregate('median_1_min_perp', '1 minute'::interval, '3 minutes'::interval, 'perp');
SELECT create_median_aggregate('median_15_min_perp', '15 minutes'::interval, '45 minutes'::interval, 'perp');
SELECT create_median_aggregate('median_1_h_perp', '1 hour'::interval, '3 hours'::interval, 'perp');
SELECT create_median_aggregate('median_2_h_perp', '2 hours'::interval, '6 hours'::interval, 'perp');
SELECT create_median_aggregate('median_1_day_perp', '1 day'::interval, '3 days'::interval, 'perp');
SELECT create_median_aggregate('median_1_week_perp', '1 week'::interval, '3 weeks'::interval, 'perp');
