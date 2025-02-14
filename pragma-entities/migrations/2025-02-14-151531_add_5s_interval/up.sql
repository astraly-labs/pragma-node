-- Your SQL goes here
CREATE MATERIALIZED VIEW price_5_s_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('5 seconds'::interval, timestamp) as bucket,
    -- Force full numeric display using 1000 for numeric, so we don't loose digits
    (percentile_cont(0.5) WITHIN GROUP (ORDER BY price))::numeric(1000,0) AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_5_s_agg',
  start_offset => INTERVAL '15 seconds',
  end_offset => '5 seconds',
  schedule_interval => INTERVAL '5 seconds');