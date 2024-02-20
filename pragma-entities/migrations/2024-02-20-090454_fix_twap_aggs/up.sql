-- Your SQL goes here
ALTER MATERIALIZED VIEW twap_2_hours_agg SET SCHEMA (
    SELECT 
        pair_id,
        time_bucket('2 hours'::interval, timestamp) as bucket,
        interpolated_average(
            time_weight('Linear', timestamp, price), 
            bucket, 
            '2 hours'::interval, 
            lag(agg) OVER (ORDER BY bucket),
            lead(agg) OVER (ORDER BY bucket)
        )::numeric as price_twap,
        COUNT(DISTINCT source) as num_sources
    FROM entries
    GROUP BY bucket, pair_id
)