-- This file should undo anything in `up.sql`
ALTER MATERIALIZED VIEW twap_2_hours_agg SET SCHEMA (
    SELECT 
        pair_id,
        time_bucket('2 hours'::interval, timestamp) as bucket,
        average(time_weight('Linear', timestamp, price))::numeric as price_twap,
        COUNT(DISTINCT source) as num_sources
    FROM entries
    GROUP BY bucket, pair_id
)