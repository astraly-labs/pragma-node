-- Your SQL goes here
-- 1 day candle
CREATE MATERIALIZED VIEW 1_day_candle_new
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 day', bucket) AS bucket,
        pair_id,
        FIRST(price, bucket) AS "open",
        MAX(price) AS high,
        MIN(price) AS low,
        LAST(price, bucket) AS "close"
    FROM price_10_s_agg
    GROUP BY bucket, pair_id
    WITH NO DATA;


SELECT add_continuous_aggregate_policy('1_day_candle_new',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');

-- 1 hour candle
CREATE MATERIALIZED VIEW 1_h_candle_new
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 hour', bucket) AS bucket,
        pair_id,
        FIRST(price, bucket) AS "open",
        MAX(price) AS high,
        MIN(price) AS low,
        LAST(price, bucket) AS "close"
    FROM price_10_s_agg
    GROUP BY bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('1_h_candle_new',
    start_offset => INTERVAL '3 hours',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');

-- 15 minute candle
CREATE MATERIALIZED VIEW 15_min_candle_new
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('15 minutes', bucket) AS bucket,
        pair_id,
        FIRST(price, bucket)::numeric AS "open",
        MAX(price)::numeric AS high,
        MIN(price)::numeric AS low,
        LAST(price, bucket)::numeric AS "close"
    FROM price_10_s_agg
    GROUP BY bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('15_min_candle_new',
    start_offset => INTERVAL '45 minutes',
    end_offset => INTERVAL '15 minutes',
    schedule_interval => INTERVAL '15 minutes');

-- 5 minute candle
CREATE MATERIALIZED VIEW 5_min_candle_new
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('5 minutes', bucket) AS bucket,
        pair_id,
        FIRST(price, bucket) AS "open",
        MAX(price) AS high,
        MIN(price) AS low,
        LAST(price, bucket) AS "close"
    FROM price_10_s_agg
    GROUP BY bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('5_min_candle_new',
    start_offset => INTERVAL '15 minutes',
    end_offset => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '5 minutes');

-- 1 minute candle
CREATE MATERIALIZED VIEW 1_min_candle_new
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 minute', bucket) AS bucket,
        pair_id,
        FIRST(price, bucket) AS "open",
        MAX(price) AS high,
        MIN(price) AS low,
        LAST(price, bucket) AS "close"
    FROM price_10_s_agg
    GROUP BY bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('1_min_candle_new',
    start_offset => INTERVAL '3 minutes',
    end_offset => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute');