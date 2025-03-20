CREATE OR REPLACE FUNCTION create_continuous_aggregate(
    p_name text,
    p_interval interval,
    p_start_offset interval,
    p_table_name text
)
RETURNS void AS $$
BEGIN
    EXECUTE format('
        CREATE MATERIALIZED VIEW %I
        WITH (timescaledb.continuous, timescaledb.materialized_only = false)
        AS SELECT 
            pair_id,
            time_bucket(%L, timestamp) as bucket,
            (percentile_cont(0.5) WITHIN GROUP (ORDER BY price))::numeric(1000,0) AS median_price,
            COUNT(DISTINCT source) as num_sources,
            array_agg(ROW(source, publisher, price, timestamp) ORDER BY timestamp) AS components
        FROM %I
        GROUP BY bucket, pair_id
        WITH NO DATA;', p_name, p_interval, p_table_name);

    EXECUTE format('
        SELECT add_continuous_aggregate_policy(%L,
            start_offset => %L,
            end_offset => %L,
            schedule_interval => %L);', p_name, p_start_offset, '0'::interval, p_interval);
END;
$$ LANGUAGE plpgsql;

-- Spot entries aggregates
SELECT create_continuous_aggregate('price_100_ms_agg', '100 milliseconds'::interval, '300 milliseconds'::interval, 'entries');
SELECT create_continuous_aggregate('price_1_s_agg', '1 second'::interval, '3 seconds'::interval, 'entries');
SELECT create_continuous_aggregate('price_10_s_agg', '10 seconds'::interval, '30 seconds'::interval, 'entries');
SELECT create_continuous_aggregate('price_5_s_agg', '5 seconds'::interval, '15 seconds'::interval, 'entries');
SELECT create_continuous_aggregate('price_1_min_agg', '1 minute'::interval, '3 minutes'::interval, 'entries');
SELECT create_continuous_aggregate('price_15_min_agg', '15 minutes'::interval, '45 minutes'::interval, 'entries');
SELECT create_continuous_aggregate('price_1_h_agg', '1 hour'::interval, '3 hours'::interval, 'entries');
SELECT create_continuous_aggregate('price_2_h_agg', '2 hours'::interval, '6 hours'::interval, 'entries');
SELECT create_continuous_aggregate('price_1_day_agg', '1 day'::interval, '3 days'::interval, 'entries');
SELECT create_continuous_aggregate('price_1_week_agg', '1 week'::interval, '3 weeks'::interval, 'entries');

-- Future entries aggregates
SELECT create_continuous_aggregate('price_100_ms_agg_future', '100 milliseconds'::interval, '300 milliseconds'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_1_s_agg_future', '1 second'::interval, '3 seconds'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_10_s_agg_future', '10 seconds'::interval, '30 seconds'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_5_s_agg_future', '5 seconds'::interval, '15 seconds'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_1_min_agg_future', '1 minute'::interval, '3 minutes'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_15_min_agg_future', '15 minutes'::interval, '45 minutes'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_1_h_agg_future', '1 hour'::interval, '3 hours'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_2_h_agg_future', '2 hours'::interval, '6 hours'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_1_day_agg_future', '1 day'::interval, '3 days'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_1_week_agg_future', '1 week'::interval, '3 weeks'::interval, 'future_entries');

