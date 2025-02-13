CREATE MATERIALIZED VIEW price_1_s_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 second'::interval, timestamp) as bucket,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_1_s_agg',
  start_offset => INTERVAL '1 day',
  end_offset => INTERVAL '2 seconds',
  schedule_interval => INTERVAL '1 second');
