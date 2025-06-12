-- Create continuous aggregates for funding rates to enable fast historical queries

CREATE FUNCTION create_funding_rates_aggregate(
    p_name text,             -- e.g., 'funding_rates_1_min'
    p_interval interval,     -- e.g., '1 minute'
    p_start_offset interval  -- e.g., '5 minutes'
)
RETURNS void AS $$
BEGIN
    -- Create the continuous aggregate materialized view
    EXECUTE format('
        CREATE MATERIALIZED VIEW %I
        WITH (timescaledb.continuous, timescaledb.materialized_only = false)
        AS SELECT
            pair,
            source,
            time_bucket(%L, timestamp) AS bucket,
            AVG(annualized_rate) AS avg_annualized_rate,
            FIRST(annualized_rate, timestamp) AS first_rate,
            LAST(annualized_rate, timestamp) AS last_rate,
            MIN(annualized_rate) AS min_rate,
            MAX(annualized_rate) AS max_rate,
            COUNT(*) AS data_points
        FROM funding_rates
        GROUP BY pair, source, bucket
        WITH NO DATA;',
        p_name, p_interval);

    -- Set chunk time interval to 7 days
    EXECUTE format('SELECT set_chunk_time_interval(%L, INTERVAL ''7 days'');', p_name);

    -- Add continuous aggregate refresh policy
    EXECUTE format('
        SELECT add_continuous_aggregate_policy(%L,
            start_offset => %L,
            end_offset => %L,
            schedule_interval => %L);',
        p_name, p_start_offset, '0'::interval, p_interval);
END;
$$ LANGUAGE plpgsql;

-- Create continuous aggregates for different time intervals
SELECT create_funding_rates_aggregate('funding_rates_1_min', '1 minute'::interval, '5 minutes'::interval);
SELECT create_funding_rates_aggregate('funding_rates_5_min', '5 minutes'::interval, '15 minutes'::interval);
SELECT create_funding_rates_aggregate('funding_rates_15_min', '15 minutes'::interval, '30 minutes'::interval);
SELECT create_funding_rates_aggregate('funding_rates_1_hour', '1 hour'::interval, '2 hours'::interval);
SELECT create_funding_rates_aggregate('funding_rates_4_hour', '4 hours'::interval, '8 hours'::interval);
SELECT create_funding_rates_aggregate('funding_rates_1_day', '1 day'::interval, '2 days'::interval);

-- Drop the helper function after creating the aggregates
DROP FUNCTION create_funding_rates_aggregate(text, interval, interval);