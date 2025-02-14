CREATE MATERIALIZED VIEW price_1_s_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 second'::interval, timestamp) as bucket,
    -- Force full numeric display using 1000 for numeric, so we don't loose digits
    (percentile_cont(0.5) WITHIN GROUP (ORDER BY price))::numeric(1000,0) AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_1_s_agg',
  start_offset => INTERVAL '10 seconds',
  end_offset => '1 second',
  schedule_interval => INTERVAL '1 second');