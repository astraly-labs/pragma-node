-- Function to create a single time-based aggregation for price data
CREATE OR REPLACE FUNCTION create_price_aggregation(
    table_name TEXT,      -- Source table name (e.g., 'spot_entry', 'future_entry')
    interval_value TEXT,  -- The time interval as text (e.g., '10 seconds', '1 min')
    name_suffix TEXT,     -- Suffix for the view name (e.g., '10_s', '1_min')
    start_offset TEXT = NULL, -- Starting offset for the policy (NULL or interval)
    materialized_only BOOLEAN = TRUE -- Whether the view is materialized only
)
RETURNS TEXT AS $$
DECLARE
    view_name TEXT;
    view_prefix TEXT;
    network_prefix TEXT;
    result TEXT;
BEGIN
    -- Extract network prefix if present (mainnet_)
    IF table_name LIKE 'mainnet_%' THEN
        network_prefix := 'mainnet_';
        table_name := REPLACE(table_name, 'mainnet_', '');
    ELSE
        network_prefix := '';
    END IF;
    
    -- Get table prefix from table_name
    view_prefix := CASE 
        WHEN table_name = 'spot_entry' THEN 'spot'
        WHEN table_name = 'future_entry' THEN 'future'
        ELSE SPLIT_PART(table_name, '_', 1)
    END;
    
    -- Create view name with network prefix if applicable
    view_name := network_prefix || view_prefix || '_price_' || name_suffix || '_agg';
    
    -- Create the materialized view with materialized_only parameter
    EXECUTE format('
        CREATE MATERIALIZED VIEW %I
        WITH (timescaledb.continuous, timescaledb.materialized_only = %L)
        AS SELECT 
            pair_id,
            time_bucket(%L::interval, timestamp) as bucket,
            percentile_disc(0.5) WITHIN GROUP (ORDER BY price)::numeric AS median_price,
            COUNT(DISTINCT source) as num_sources
        FROM %I
        GROUP BY bucket, pair_id
        WITH NO DATA;
    ', view_name, materialized_only, interval_value, network_prefix || table_name);

    -- Create the policy expression based on start_offset (NULL or interval)
    IF start_offset IS NOT NULL THEN
        EXECUTE format('
            SELECT add_continuous_aggregate_policy(%L,
                start_offset => INTERVAL %L,
                end_offset => INTERVAL %L,
                schedule_interval => INTERVAL %L);
        ', view_name, start_offset, interval_value, interval_value);
    ELSE
        EXECUTE format('
            SELECT add_continuous_aggregate_policy(%L,
                start_offset => NULL,
                end_offset => INTERVAL %L,
                schedule_interval => INTERVAL %L);
        ', view_name, interval_value, interval_value);
    END IF;
    
    result := 'Created aggregation view ' || view_name || ' with interval ' || interval_value;
    RETURN result;
END;
$$ LANGUAGE plpgsql;

