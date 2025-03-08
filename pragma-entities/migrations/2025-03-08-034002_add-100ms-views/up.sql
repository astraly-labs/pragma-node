-- Your SQL goes here
CREATE MATERIALIZED VIEW price_100_ms_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('100 ms'::interval, timestamp) as bucket,
    -- Force full numeric display using 1000 for numeric, so we don't loose digits
    (percentile_cont(0.5) WITHIN GROUP (ORDER BY price))::numeric(1000,0) AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_100_ms_agg',
  start_offset => INTERVAL '1000 ms',
  end_offset => '100 ms',
  schedule_interval => INTERVAL '100 ms');