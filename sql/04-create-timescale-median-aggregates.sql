CREATE FUNCTION create_onchain_median_aggregate(
    p_name text,          -- e.g., 'mainnet_spot_median_100_ms'
    p_table_name text,    -- e.g., 'mainnet_spot_entry'
    p_interval interval,  -- e.g., '100 milliseconds'
    p_start_offset interval, -- e.g., '300 milliseconds'
    p_type text           -- 'spot' or 'perp'
)
RETURNS void AS $$
DECLARE
    where_condition text;
BEGIN
    -- Set the WHERE condition based on p_type and p_table_name
    IF p_type = 'spot' AND (p_table_name = 'mainnet_spot_entry' OR p_table_name = 'spot_entry') THEN
        where_condition := '"timestamp" IS NOT NULL';
    ELSIF p_type = 'perp' AND (p_table_name = 'mainnet_future_entry' OR p_table_name = 'future_entry') THEN
        where_condition := '"timestamp" IS NOT NULL AND expiration_timestamp IS NULL';
    ELSE
        RAISE EXCEPTION 'Invalid combination of p_type % and p_table_name %', p_type, p_table_name;
    END IF;

    -- Create the per-source materialized view
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
    p_name, p_interval, p_table_name, where_condition);

    -- Create the main materialized view with median across sources
    EXECUTE format('
        CREATE MATERIALIZED VIEW %I
        WITH (timescaledb.continuous, timescaledb.materialized_only = false)
        AS SELECT
            pair_id,
            time_bucket(%L, subbucket) AS bucket,
            percentile_cont(0.5) WITHIN GROUP (ORDER BY source_median_price)::numeric(1000,0) AS median_price,
            COUNT(DISTINCT source) AS num_sources
        FROM %I
        GROUP BY pair_id, bucket
        WITH NO DATA;',
    p_name, p_interval, p_name || '_per_source');

    -- Set chunk time interval to 7 days
    EXECUTE format('SELECT set_chunk_time_interval(%L, INTERVAL ''7 days'');', p_name || '_per_source');
    EXECUTE format('SELECT set_chunk_time_interval(%L, INTERVAL ''7 days'');', p_name);

    -- Add continuous aggregate refresh policies
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

-- Mainnet Spot Median
SELECT create_onchain_median_aggregate('mainnet_spot_median_10_s', 'mainnet_spot_entry', '10 seconds'::interval, '30 seconds'::interval, 'spot');
SELECT create_onchain_median_aggregate('mainnet_spot_median_1_min', 'mainnet_spot_entry', '1 minute'::interval, '3 minutes'::interval, 'spot');
SELECT create_onchain_median_aggregate('mainnet_spot_median_15_min', 'mainnet_spot_entry', '15 minutes'::interval, '45 minutes'::interval, 'spot');
SELECT create_onchain_median_aggregate('mainnet_spot_median_1_h', 'mainnet_spot_entry', '1 hour'::interval, '3 hours'::interval, 'spot');
SELECT create_onchain_median_aggregate('mainnet_spot_median_2_h', 'mainnet_spot_entry', '2 hours'::interval, '6 hours'::interval, 'spot');
SELECT create_onchain_median_aggregate('mainnet_spot_median_1_day', 'mainnet_spot_entry', '1 day'::interval, '3 days'::interval, 'spot');
SELECT create_onchain_median_aggregate('mainnet_spot_median_1_week', 'mainnet_spot_entry', '1 week'::interval, '3 weeks'::interval, 'spot');

-- Testnet Spot Median
SELECT create_onchain_median_aggregate('spot_median_10_s', 'spot_entry', '10 seconds'::interval, '30 seconds'::interval, 'spot');
SELECT create_onchain_median_aggregate('spot_median_1_min', 'spot_entry', '1 minute'::interval, '3 minutes'::interval, 'spot');
SELECT create_onchain_median_aggregate('spot_median_15_min', 'spot_entry', '15 minutes'::interval, '45 minutes'::interval, 'spot');
SELECT create_onchain_median_aggregate('spot_median_1_h', 'spot_entry', '1 hour'::interval, '3 hours'::interval, 'spot');
SELECT create_onchain_median_aggregate('spot_median_2_h', 'spot_entry', '2 hours'::interval, '6 hours'::interval, 'spot');
SELECT create_onchain_median_aggregate('spot_median_1_day', 'spot_entry', '1 day'::interval, '3 days'::interval, 'spot');
SELECT create_onchain_median_aggregate('spot_median_1_week', 'spot_entry', '1 week'::interval, '3 weeks'::interval, 'spot');

-- Mainnet Perp Median
SELECT create_onchain_median_aggregate('mainnet_perp_median_10_s', 'mainnet_future_entry', '10 seconds'::interval, '30 seconds'::interval, 'perp');
SELECT create_onchain_median_aggregate('mainnet_perp_median_1_min', 'mainnet_future_entry', '1 minute'::interval, '3 minutes'::interval, 'perp');
SELECT create_onchain_median_aggregate('mainnet_perp_median_15_min', 'mainnet_future_entry', '15 minutes'::interval, '45 minutes'::interval, 'perp');
SELECT create_onchain_median_aggregate('mainnet_perp_median_1_h', 'mainnet_future_entry', '1 hour'::interval, '3 hours'::interval, 'perp');
SELECT create_onchain_median_aggregate('mainnet_perp_median_2_h', 'mainnet_future_entry', '2 hours'::interval, '6 hours'::interval, 'perp');
SELECT create_onchain_median_aggregate('mainnet_perp_median_1_day', 'mainnet_future_entry', '1 day'::interval, '3 days'::interval, 'perp');
SELECT create_onchain_median_aggregate('mainnet_perp_median_1_week', 'mainnet_future_entry', '1 week'::interval, '3 weeks'::interval, 'perp');

-- Testnet Perp Median
SELECT create_onchain_median_aggregate('perp_median_10_s', 'future_entry', '10 seconds'::interval, '30 seconds'::interval, 'perp');
SELECT create_onchain_median_aggregate('perp_median_1_min', 'future_entry', '1 minute'::interval, '3 minutes'::interval, 'perp');
SELECT create_onchain_median_aggregate('perp_median_15_min', 'future_entry', '15 minutes'::interval, '45 minutes'::interval, 'perp');
SELECT create_onchain_median_aggregate('perp_median_1_h', 'future_entry', '1 hour'::interval, '3 hours'::interval, 'perp');
SELECT create_onchain_median_aggregate('perp_median_2_h', 'future_entry', '2 hours'::interval, '6 hours'::interval, 'perp');
SELECT create_onchain_median_aggregate('perp_median_1_day', 'future_entry', '1 day'::interval, '3 days'::interval, 'perp');
SELECT create_onchain_median_aggregate('perp_median_1_week', 'future_entry', '1 week'::interval, '3 weeks'::interval, 'perp');
