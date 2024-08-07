-- Your SQL goes here
CREATE MATERIALIZED VIEW price_1_min_agg_future
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 min'::interval, timestamp) as bucket,
    expiration_timestamp,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM future_entries
GROUP BY bucket, pair_id, expiration_timestamp
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_1_min_agg_future',
  start_offset => NULL,
  end_offset => INTERVAL '1 min',
  schedule_interval => INTERVAL '1 min');

CREATE MATERIALIZED VIEW price_15_min_agg_future
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('15 min'::interval, timestamp) as bucket,
    expiration_timestamp,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM future_entries
GROUP BY bucket, pair_id, expiration_timestamp
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_15_min_agg_future',
  start_offset => NULL,
  end_offset => INTERVAL '15 min',
  schedule_interval => INTERVAL '15 min');

CREATE MATERIALIZED VIEW price_1_h_agg_future
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 hour'::interval, timestamp) as bucket,
    expiration_timestamp,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM future_entries
GROUP BY bucket, pair_id, expiration_timestamp
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_1_h_agg_future',
  start_offset => NULL,
  end_offset => INTERVAL '1 hour',
  schedule_interval => INTERVAL '1 hour');

CREATE MATERIALIZED VIEW price_2_h_agg_future
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('2 hours'::interval, timestamp) as bucket,
    expiration_timestamp,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM future_entries
GROUP BY bucket, pair_id, expiration_timestamp
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_2_h_agg_future',
  start_offset => NULL,
  end_offset => INTERVAL '2 hours',
  schedule_interval => INTERVAL '2 hours');