-- Your SQL goes here
CREATE MATERIALIZED VIEW price_10_s_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('10 seconds'::interval, timestamp) as bucket,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM entries
GROUP BY bucket, pair_id
WITH NO DATA;

CALL refresh_continuous_aggregate('price_10_s_agg', NULL, localtimestamp - INTERVAL '1 day');

SELECT add_continuous_aggregate_policy('price_10_s_agg',
  start_offset => INTERVAL '1 day',
  end_offset => INTERVAL '10 seconds',
  schedule_interval => INTERVAL '10 seconds');
