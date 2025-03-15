CREATE OR REPLACE FUNCTION create_twap_aggregate(
    p_name text,
    p_interval interval,
    p_start_offset interval,
    p_table_name text
)
RETURNS void AS $$
BEGIN
    EXECUTE format('
        CREATE MATERIALIZED VIEW %s
        WITH (timescaledb.continuous, timescaledb.materialized_only = false)
        AS SELECT 
            pair_id,
            time_bucket($1::interval, timestamp) as bucket,
            average(time_weight(''Linear'', timestamp, price))::numeric as price_twap,
            COUNT(DISTINCT source) as num_sources
        FROM %I
        GROUP BY bucket, pair_id
        WITH NO DATA;', p_name, p_interval, p_table_name);

    EXECUTE format('
        SELECT add_continuous_aggregate_policy(''%s'',
            start_offset => $1,
            end_offset => $2,
            schedule_interval => $3);', p_name, p_start_offset, p_interval, p_interval);
END;
$$ LANGUAGE plpgsql;

-- Spot entries TWAP
SELECT create_twap_aggregate('twap_1_min_agg', '1 minute'::interval, '3 minutes'::interval, 'entries');
SELECT create_twap_aggregate('twap_5_min_agg', '5 minutes'::interval, '15 minutes'::interval, 'entries');
SELECT create_twap_aggregate('twap_15_min_agg', '15 minutes'::interval, '45 minutes'::interval, 'entries');
SELECT create_twap_aggregate('twap_1_h_agg', '1 hour'::interval, '3 hours'::interval, 'entries');
SELECT create_twap_aggregate('twap_2_h_agg', '2 hours'::interval, '6 hours'::interval, 'entries');
SELECT create_twap_aggregate('twap_1_day_agg', '1 day'::interval, '3 days'::interval, 'entries');

-- Future entries TWAP
SELECT create_twap_aggregate('twap_1_min_agg_future', '1 minute'::interval, '3 minutes'::interval, 'future_entries');
SELECT create_twap_aggregate('twap_5_min_agg_future', '5 minutes'::interval, '15 minutes'::interval, 'future_entries');
SELECT create_twap_aggregate('twap_15_min_agg_future', '15 minutes'::interval, '45 minutes'::interval, 'future_entries');
SELECT create_twap_aggregate('twap_1_h_agg_future', '1 hour'::interval, '3 hours'::interval, 'future_entries');
SELECT create_twap_aggregate('twap_2_h_agg_future', '2 hours'::interval, '6 hours'::interval, 'future_entries');
SELECT create_twap_aggregate('twap_1_day_agg_future', '1 day'::interval, '3 days'::interval, 'future_entries');