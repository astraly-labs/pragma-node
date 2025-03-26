CREATE TYPE price_component AS (
    source text,
    publisher text,
    price numeric(1000,0),
    "timestamp" timestamptz
);

CREATE OR REPLACE FUNCTION create_continuous_aggregate(
    p_name text,
    p_interval interval,
    p_start_offset interval,
    p_table_name text
)
RETURNS void AS $$
DECLARE
    interval_text text;
    internal_base_view_name text;
    main_view_name text;
BEGIN
    interval_text := p_interval::text;
    internal_base_view_name := p_name || '_internal_base';
    main_view_name := p_name;
    
    -- Step 1: Create source-level continuous aggregate (internal)
    EXECUTE format('
        CREATE MATERIALIZED VIEW %I
        WITH (timescaledb.continuous)
        AS
        SELECT 
            pair_id,
            source,
            time_bucket(''%s''::interval, "timestamp") as bucket,
            percentile_cont(0.5) WITHIN GROUP (ORDER BY price)::numeric(1000,0) AS source_median_price,
            array_agg(ROW(source, publisher, price, "timestamp")::price_component) AS components
        FROM %I
        WHERE "timestamp" IS NOT NULL
        GROUP BY pair_id, source, bucket
        WITH NO DATA', 
        internal_base_view_name, interval_text, p_table_name);
        
    -- Step 2: Create median-of-medians as the main continuous aggregate
    -- CRITICAL: Must use a fresh time_bucket call directly on the bucket field
    EXECUTE format('
        CREATE MATERIALIZED VIEW %I
        WITH (timescaledb.continuous)
        AS
        SELECT 
            pair_id,
            time_bucket(''%s''::interval, bucket) as bucket,
            percentile_cont(0.5) WITHIN GROUP (ORDER BY source_median_price)::numeric(1000,0) AS median_price,
            COUNT(DISTINCT source) as num_sources,
            array_agg(components) AS components
        FROM %I
        GROUP BY pair_id, time_bucket(''%s''::interval, bucket)
        WITH NO DATA',
        main_view_name, interval_text, internal_base_view_name, interval_text);

    -- Add continuous aggregate policies
    EXECUTE format('
        SELECT add_continuous_aggregate_policy(%L,
            start_offset => ''%s''::interval,
            end_offset => ''0''::interval,
            schedule_interval => ''%s''::interval)', 
        internal_base_view_name, p_start_offset::text, p_interval::text);
        
    EXECUTE format('
        SELECT add_continuous_aggregate_policy(%L,
            start_offset => ''%s''::interval,
            end_offset => ''0''::interval,
            schedule_interval => ''%s''::interval)', 
        main_view_name, p_start_offset::text, p_interval::text);
END;
$$ LANGUAGE plpgsql;

-- Use consistent interval notation for all entries
SELECT create_continuous_aggregate('price_100_ms_agg', '0.1 seconds'::interval, '0.3 seconds'::interval, 'entries');
SELECT create_continuous_aggregate('price_1_s_agg', '1 second'::interval, '3 seconds'::interval, 'entries');
SELECT create_continuous_aggregate('price_10_s_agg', '10 seconds'::interval, '30 seconds'::interval, 'entries');
SELECT create_continuous_aggregate('price_5_s_agg', '5 seconds'::interval, '15 seconds'::interval, 'entries');
SELECT create_continuous_aggregate('price_1_min_agg', '1 minute'::interval, '3 minutes'::interval, 'entries');
SELECT create_continuous_aggregate('price_15_min_agg', '15 minutes'::interval, '45 minutes'::interval, 'entries');
SELECT create_continuous_aggregate('price_1_h_agg', '1 hour'::interval, '3 hours'::interval, 'entries');
SELECT create_continuous_aggregate('price_2_h_agg', '2 hours'::interval, '6 hours'::interval, 'entries');
SELECT create_continuous_aggregate('price_1_day_agg', '1 day'::interval, '3 days'::interval, 'entries');
SELECT create_continuous_aggregate('price_1_week_agg', '1 week'::interval, '3 weeks'::interval, 'entries');

-- Future entries with same approach
SELECT create_continuous_aggregate('price_100_ms_agg_future', '0.1 seconds'::interval, '0.3 seconds'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_1_s_agg_future', '1 second'::interval, '3 seconds'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_10_s_agg_future', '10 seconds'::interval, '30 seconds'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_5_s_agg_future', '5 seconds'::interval, '15 seconds'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_1_min_agg_future', '1 minute'::interval, '3 minutes'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_15_min_agg_future', '15 minutes'::interval, '45 minutes'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_1_h_agg_future', '1 hour'::interval, '3 hours'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_2_h_agg_future', '2 hours'::interval, '6 hours'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_1_day_agg_future', '1 day'::interval, '3 days'::interval, 'future_entries');
SELECT create_continuous_aggregate('price_1_week_agg_future', '1 week'::interval, '3 weeks'::interval, 'future_entries');

