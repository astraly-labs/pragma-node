-- 1min TWAP
CREATE MATERIALIZED VIEW twap_1_min_agg_future
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 min'::interval, timestamp) as bucket,
    average(time_weight('Linear', timestamp, price))::numeric as price_twap,
    COUNT(DISTINCT source) as num_sources
FROM future_entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('twap_1_min_agg_future',
  start_offset => NULL,
  end_offset => INTERVAL '1 min',
  schedule_interval => INTERVAL '1 min');

-- 15min TWAP
CREATE MATERIALIZED VIEW twap_15_min_agg_future
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('15 min'::interval, timestamp) as bucket,
    average(time_weight('Linear', timestamp, price))::numeric as price_twap,
    COUNT(DISTINCT source) as num_sources
FROM future_entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('twap_15_min_agg_future',
  start_offset => NULL,
  end_offset => INTERVAL '15 min',
  schedule_interval => INTERVAL '15 min');

-- 1hour TWAP
CREATE MATERIALIZED VIEW twap_1_hour_agg_future
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 hour'::interval, timestamp) as bucket,
    average(time_weight('Linear', timestamp, price))::numeric as price_twap,
    COUNT(DISTINCT source) as num_sources
FROM future_entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('twap_1_hour_agg_future',
  start_offset => NULL,
  end_offset => INTERVAL '1 hour',
  schedule_interval => INTERVAL '1 hour');

-- 2hours TWAP
CREATE MATERIALIZED VIEW twap_2_hours_agg_future
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('2 hours'::interval, timestamp) as bucket,
    average(time_weight('Linear', timestamp, price))::numeric as price_twap,
    COUNT(DISTINCT source) as num_sources
FROM future_entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('twap_2_hours_agg_future',
  start_offset => NULL,
  end_offset => INTERVAL '2 hours',
  schedule_interval => INTERVAL '2 hours');