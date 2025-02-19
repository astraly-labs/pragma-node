--testnet spot
CREATE MATERIALIZED VIEW spot_price_1_day_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 day'::interval, timestamp) as bucket,
    percentile_disc(0.5) WITHIN GROUP (ORDER BY price)::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM spot_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('spot_price_1_day_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 day',
  schedule_interval => INTERVAL '1 day');

CREATE MATERIALIZED VIEW spot_price_1_week_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 week'::interval, timestamp) as bucket,
    percentile_disc(0.5) WITHIN GROUP (ORDER BY price)::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM spot_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('spot_price_1_week_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 week',
  schedule_interval => INTERVAL '1 week');

--testnet future
CREATE MATERIALIZED VIEW future_price_1_day_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 day'::interval, timestamp) as bucket,
    percentile_disc(0.5) WITHIN GROUP (ORDER BY price)::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM future_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('future_price_1_day_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 day',
  schedule_interval => INTERVAL '1 day');

CREATE MATERIALIZED VIEW future_price_1_week_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 week'::interval, timestamp) as bucket,
    percentile_disc(0.5) WITHIN GROUP (ORDER BY price)::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM future_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('future_price_1_week_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 week',
  schedule_interval => INTERVAL '1 week');

--mainnet spot
CREATE MATERIALIZED VIEW mainnet_spot_price_1_day_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 day'::interval, timestamp) as bucket,
    percentile_disc(0.5) WITHIN GROUP (ORDER BY price)::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM mainnet_spot_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_spot_price_1_day_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 day',
  schedule_interval => INTERVAL '1 day');

CREATE MATERIALIZED VIEW mainnet_spot_price_1_week_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 week'::interval, timestamp) as bucket,
    percentile_disc(0.5) WITHIN GROUP (ORDER BY price)::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM mainnet_spot_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_spot_price_1_week_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 week',
  schedule_interval => INTERVAL '1 week');

--mainnet future
CREATE MATERIALIZED VIEW mainnet_future_price_1_day_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 day'::interval, timestamp) as bucket,
    percentile_disc(0.5) WITHIN GROUP (ORDER BY price)::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM mainnet_future_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_future_price_1_day_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 day',
  schedule_interval => INTERVAL '1 day');

CREATE MATERIALIZED VIEW mainnet_future_price_1_week_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 week'::interval, timestamp) as bucket,
    percentile_disc(0.5) WITHIN GROUP (ORDER BY price)::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM mainnet_future_entry
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_future_price_1_week_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 week',
  schedule_interval => INTERVAL '1 week');
