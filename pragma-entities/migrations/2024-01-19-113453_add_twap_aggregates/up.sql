-- ===============================
-- Function used to create a twap continuous aggregate 
-- ===============================
CREATE OR REPLACE FUNCTION create_twap_aggregate(
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
        where_condition := '"timestamp" IS NOT NULL AND expiration_timestamp = NULL';
    ELSE
        RAISE EXCEPTION 'Invalid type: %', p_type;
    END IF;

    -- Create the sub materialized view for TWAP per source
    EXECUTE format('
        CREATE MATERIALIZED VIEW %s_per_source
        WITH (timescaledb.continuous, timescaledb.materialized_only = false)
        AS SELECT
            pair_id,
            source,
            time_bucket(%L, "timestamp") AS subbucket,
            average(time_weight(''Linear'', "timestamp", price))::numeric(1000,0) AS source_twap_price
        FROM %I
        WHERE %s
        GROUP BY pair_id, source, subbucket
        WITH NO DATA;',
        p_name, p_interval, table_name, where_condition);

    -- Create the main materialized view averaging source TWAPs
    EXECUTE format('
        CREATE MATERIALIZED VIEW %I
        WITH (timescaledb.continuous, timescaledb.materialized_only = false)
        AS SELECT
            pair_id,
            time_bucket(%L, subbucket) AS bucket,
            avg(source_twap_price)::numeric(1000,0) AS twap_price,
            COUNT(DISTINCT source) AS num_sources,
            array_agg(ROW(source, source_twap_price, subbucket)::price_component) AS components
        FROM %I
        GROUP BY pair_id, bucket
        WITH NO DATA;',
        p_name, p_interval, p_name || '_per_source');

    -- Set chunk time interval to 7 days for both views
    EXECUTE format('SELECT set_chunk_time_interval(%L, INTERVAL ''7 days'');', p_name || '_per_source');
    EXECUTE format('SELECT set_chunk_time_interval(%L, INTERVAL ''7 days'');', p_name);

    -- Add continuous aggregate policies
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

-- SPOT twap
SELECT create_twap_aggregate('twap_1_min_spot', '1 minute'::interval, '3 minutes'::interval, 'spot');
SELECT create_twap_aggregate('twap_5_min_spot', '5 minutes'::interval, '15 minutes'::interval, 'spot');
SELECT create_twap_aggregate('twap_15_min_spot', '15 minutes'::interval, '45 minutes'::interval, 'spot');
SELECT create_twap_aggregate('twap_1_h_spot', '1 hour'::interval, '3 hours'::interval, 'spot');
SELECT create_twap_aggregate('twap_2_h_spot', '2 hours'::interval, '6 hours'::interval, 'spot');
SELECT create_twap_aggregate('twap_1_day_spot', '1 day'::interval, '3 days'::interval, 'spot');

-- PERP twap
SELECT create_twap_aggregate('twap_1_min_perp', '1 minute'::interval, '3 minutes'::interval, 'perp');
SELECT create_twap_aggregate('twap_5_min_perp', '5 minutes'::interval, '15 minutes'::interval, 'perp');
SELECT create_twap_aggregate('twap_15_min_perp', '15 minutes'::interval, '45 minutes'::interval, 'perp');
SELECT create_twap_aggregate('twap_1_h_perp', '1 hour'::interval, '3 hours'::interval, 'perp');
SELECT create_twap_aggregate('twap_2_h_perp', '2 hours'::interval, '6 hours'::interval, 'perp');
SELECT create_twap_aggregate('twap_1_day_perp', '1 day'::interval, '3 days'::interval, 'perp');
