CREATE MATERIALIZED VIEW mainnet_spot_price_10_s_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('10 seconds'::interval, timestamp) as bucket,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM mainnet_spot_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_spot_price_10_s_agg',
  start_offset => INTERVAL '1 day',
  end_offset => INTERVAL '10 seconds',
  schedule_interval => INTERVAL '10 seconds');


CREATE MATERIALIZED VIEW mainnet_spot_price_1_min_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 min'::interval, timestamp) as bucket,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM mainnet_spot_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_spot_price_1_min_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 min',
  schedule_interval => INTERVAL '1 min');


CREATE MATERIALIZED VIEW mainnet_spot_price_15_min_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('15 min'::interval, timestamp) as bucket,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM mainnet_spot_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_spot_price_15_min_agg',
  start_offset => NULL,
  end_offset => INTERVAL '15 min',
  schedule_interval => INTERVAL '15 min');


CREATE MATERIALIZED VIEW mainnet_spot_price_1_hour_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 hour'::interval, timestamp) as bucket,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM mainnet_spot_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_spot_price_1_hour_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 hour',
  schedule_interval => INTERVAL '1 hour');


CREATE MATERIALIZED VIEW mainnet_spot_price_2_hour_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('2 hour'::interval, timestamp) as bucket,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM mainnet_spot_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_spot_price_2_hour_agg',
  start_offset => NULL,
  end_offset => INTERVAL '2 hour',
  schedule_interval => INTERVAL '2 hour');