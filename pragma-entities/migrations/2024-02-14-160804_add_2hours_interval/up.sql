-- Your SQL goes here

-- aggregate
CREATE MATERIALIZED VIEW price_2_h_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('2 hours'::interval, timestamp) as bucket,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_2_h_agg',
  start_offset => NULL,
  end_offset => INTERVAL '2 hours',
  schedule_interval => INTERVAL '2 hours');

-- twap
CREATE MATERIALIZED VIEW twap_2_hours_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS SELECT 
    pair_id,
    time_bucket('2 hours'::interval, timestamp) as bucket,
    average(time_weight('Linear', timestamp, price))::numeric as price_twap,
    COUNT(DISTINCT source) as num_sources
FROM entries
GROUP BY bucket, pair_id
WITH NO DATA;

SELECT add_continuous_aggregate_policy('twap_2_hours_agg',
  start_offset => NULL,
  end_offset => INTERVAL '2 hours',
  schedule_interval => INTERVAL '2 hours');

-- ohlc
CREATE MATERIALIZED VIEW two_hour_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('2 hours', timestamp) AS bucket,
        pair_id,
        FIRST(price, timestamp) AS "open",
        MAX(price) AS high,
        MIN(price) AS low,
        LAST(price, timestamp) AS "close"
    FROM entries
    GROUP BY bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('two_hour_candle',
    start_offset => INTERVAL '6 hours',
    end_offset => INTERVAL '2 hours',
    schedule_interval => INTERVAL '2 hours');