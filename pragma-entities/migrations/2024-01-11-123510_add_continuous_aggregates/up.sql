-- Your SQL goes here
CREATE MATERIALIZED VIEW price_1_min_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = true)
AS SELECT source,
    pair_id,
    time_bucket('1 min'::interval, timestamp) as bucket,
    percentile_agg(price)
FROM entries
GROUP BY source, pair_id, bucket
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_1_min_agg',
  start_offset => INTERVAL '3 min',
  end_offset => INTERVAL '1 min',
  schedule_interval => INTERVAL '1 min');

CREATE MATERIALIZED VIEW price_15_min_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = true)
AS SELECT source,
    pair_id,
    time_bucket('15 min'::interval, timestamp) as bucket,
    percentile_agg(price)
FROM entries
GROUP BY source, pair_id, bucket
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_15_min_agg',
  start_offset => INTERVAL '45 min',
  end_offset => INTERVAL '15 min',
  schedule_interval => INTERVAL '15 min');

CREATE MATERIALIZED VIEW price_1_h_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = true)
AS SELECT source,
    pair_id,
    time_bucket('1 hour'::interval, timestamp) as bucket,
    percentile_agg(price)
FROM entries
GROUP BY source, pair_id, bucket
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_1_h_agg',
  start_offset => INTERVAL '3 hours',
  end_offset => INTERVAL '1 hour',
  schedule_interval => INTERVAL '1 hour');

