-- Your SQL goes here

-- aggregate
CREATE MATERIALIZED VIEW price_1_day_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 day'::interval, timestamp) as bucket,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_1_day_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 day',
  schedule_interval => INTERVAL '1 day');

CREATE MATERIALIZED VIEW price_1_week_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 week'::interval, timestamp) as bucket,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_1_week_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 week',
  schedule_interval => INTERVAL '1 week');

-- aggregate future

CREATE MATERIALIZED VIEW price_1_day_agg_future
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 day'::interval, timestamp) as bucket,
    expiration_timestamp,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM future_entries
GROUP BY bucket, pair_id, expiration_timestamp
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_1_day_agg_future',
  start_offset => NULL,
  end_offset => INTERVAL '1 day',
  schedule_interval => INTERVAL '1 day');

CREATE MATERIALIZED VIEW price_1_week_agg_future
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 week'::interval, timestamp) as bucket,
    expiration_timestamp,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM future_entries
GROUP BY bucket, pair_id, expiration_timestamp
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_1_week_agg_future',
  start_offset => NULL,
  end_offset => INTERVAL '1 week',
  schedule_interval => INTERVAL '1 week');


-- twap
CREATE MATERIALIZED VIEW twap_1_day_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 day'::interval, timestamp) as bucket,
    average(time_weight('Linear', timestamp, price))::numeric as price_twap,
    COUNT(DISTINCT source) as num_sources
FROM entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('twap_1_day_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 day',
  schedule_interval => INTERVAL '1 day');

CREATE MATERIALIZED VIEW twap_1_week_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 week'::interval, timestamp) as bucket,
    average(time_weight('Linear', timestamp, price))::numeric as price_twap,
    COUNT(DISTINCT source) as num_sources
FROM entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('twap_1_week_agg',
  start_offset => NULL,
  end_offset => INTERVAL '1 week',
  schedule_interval => INTERVAL '1 week');

-- twap future

CREATE MATERIALIZED VIEW twap_1_day_agg_future
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 day'::interval, timestamp) as bucket,
    expiration_timestamp,
    average(time_weight('Linear', timestamp, price))::numeric as price_twap,
    COUNT(DISTINCT source) as num_sources
FROM future_entries
GROUP BY bucket, pair_id, expiration_timestamp
WITH NO DATA;


SELECT add_continuous_aggregate_policy('twap_1_day_agg_future',
  start_offset => NULL,
  end_offset => INTERVAL '1 day',
  schedule_interval => INTERVAL '1 day');

CREATE MATERIALIZED VIEW twap_1_week_agg_future
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('1 week'::interval, timestamp) as bucket,
    expiration_timestamp,
    average(time_weight('Linear', timestamp, price))::numeric as price_twap,
    COUNT(DISTINCT source) as num_sources
FROM future_entries
GROUP BY bucket, pair_id, expiration_timestamp
WITH NO DATA;

SELECT add_continuous_aggregate_policy('twap_1_week_agg_future',
  start_offset => NULL,
  end_offset => INTERVAL '1 week',
  schedule_interval => INTERVAL '1 week');


-- ohlc

CREATE MATERIALIZED VIEW new_1_week_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 week', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;


SELECT add_continuous_aggregate_policy('new_1_week_candle',
    start_offset => INTERVAL '3 week',
    end_offset => INTERVAL '1 week',
    schedule_interval => INTERVAL '1 week');