-- ===============================
-- Spot entries twap
-- ===============================
CREATE OR REPLACE FUNCTION create_spot_twap_aggregate(
    p_name text,
    p_interval interval,
    p_start_offset interval
)
RETURNS void AS $$
BEGIN
    -- Create the materialized view with continuous aggregate
    EXECUTE format('
        CREATE MATERIALIZED VIEW %I
        WITH (timescaledb.continuous, timescaledb.materialized_only = false)
        AS SELECT 
            pair_id,
            time_bucket(%L, timestamp) as bucket,
            average(time_weight(''Linear'', timestamp, price))::numeric as price_twap,
            COUNT(DISTINCT source) as num_sources
        FROM entries
        GROUP BY bucket, pair_id
        WITH NO DATA;', p_name, p_interval);

    -- Set the chunk time interval to 7 days
    EXECUTE format('SELECT set_chunk_time_interval(%L, INTERVAL ''7 days'');', p_name);

    -- Add the continuous aggregate refresh policy
    EXECUTE format('
        SELECT add_continuous_aggregate_policy(%L,
            start_offset => %L,
            end_offset => %L,
            schedule_interval => %L);', p_name, p_start_offset, '0'::interval, p_interval);
END;
$$ LANGUAGE plpgsql;

SELECT create_spot_twap_aggregate('twap_1_min_spot', '1 minute'::interval, '3 minutes'::interval);
SELECT create_spot_twap_aggregate('twap_5_min_spot', '5 minutes'::interval, '15 minutes'::interval);
SELECT create_spot_twap_aggregate('twap_15_min_spot', '15 minutes'::interval, '45 minutes'::interval);
SELECT create_spot_twap_aggregate('twap_1_h_spot', '1 hour'::interval, '3 hours'::interval);
SELECT create_spot_twap_aggregate('twap_2_h_spot', '2 hours'::interval, '6 hours'::interval);
SELECT create_spot_twap_aggregate('twap_1_day_spot', '1 day'::interval, '3 days'::interval);

-- ===============================
-- Perp entries twap
-- ===============================
CREATE OR REPLACE FUNCTION create_perp_twap_aggregate(
    p_name text,
    p_interval interval,
    p_start_offset interval
)
RETURNS void AS $$
BEGIN
    -- Create the materialized view with continuous aggregate
    EXECUTE format('
        CREATE MATERIALIZED VIEW %I
        WITH (timescaledb.continuous, timescaledb.materialized_only = false)
        AS SELECT 
            pair_id,
            time_bucket(%L, timestamp) as bucket,
            average(time_weight(''Linear'', timestamp, price))::numeric as price_twap,
            COUNT(DISTINCT source) as num_sources
        FROM future_entries
        WHERE expiration_timestamp = NULL
        GROUP BY bucket, pair_id
        WITH NO DATA;', p_name, p_interval);

    -- Set the chunk time interval to 7 days
    EXECUTE format('SELECT set_chunk_time_interval(%L, INTERVAL ''7 days'');', p_name);

    -- Add the continuous aggregate refresh policy
    EXECUTE format('
        SELECT add_continuous_aggregate_policy(%L,
            start_offset => %L,
            end_offset => %L,
            schedule_interval => %L);', p_name, p_start_offset, '0'::interval, p_interval);
END;
$$ LANGUAGE plpgsql;

SELECT create_perp_twap_aggregate('twap_1_min_perp', '1 minute'::interval, '3 minutes'::interval);
SELECT create_perp_twap_aggregate('twap_5_min_perp', '5 minutes'::interval, '15 minutes'::interval);
SELECT create_perp_twap_aggregate('twap_15_min_perp', '15 minutes'::interval, '45 minutes'::interval);
SELECT create_perp_twap_aggregate('twap_1_h_perp', '1 hour'::interval, '3 hours'::interval);
SELECT create_perp_twap_aggregate('twap_2_h_perp', '2 hours'::interval, '6 hours'::interval);
SELECT create_perp_twap_aggregate('twap_1_day_perp', '1 day'::interval, '3 days'::interval);