-- 1 day candle
CREATE MATERIALIZED VIEW future_1_day_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 day', timestamp) AS bucket,
        pair_id,
        FIRST(price, timestamp) AS "open",
        MAX(price) AS high,
        MIN(price) AS low,
        LAST(price, timestamp) AS "close"
    FROM future_entry
    GROUP BY bucket, pair_id
    WITH NO DATA;


SELECT add_continuous_aggregate_policy('future_1_day_candle',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');

-- 1 hour candle
CREATE MATERIALIZED VIEW future_1_hour_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 hour', timestamp) AS bucket,
        pair_id,
        FIRST(price, timestamp) AS "open",
        MAX(price) AS high,
        MIN(price) AS low,
        LAST(price, timestamp) AS "close"
    FROM future_entry
    GROUP BY bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('future_1_hour_candle',
    start_offset => INTERVAL '3 hours',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');

-- 15 minute candle
CREATE MATERIALIZED VIEW future_15_min_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('15 minutes', timestamp) AS bucket,
        pair_id,
        FIRST(price, timestamp)::numeric AS "open",
        MAX(price)::numeric AS high,
        MIN(price)::numeric AS low,
        LAST(price, timestamp)::numeric AS "close"
    FROM future_entry
    GROUP BY bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('future_15_min_candle',
    start_offset => INTERVAL '45 minutes',
    end_offset => INTERVAL '15 minutes',
    schedule_interval => INTERVAL '15 minutes');

-- 5 minute candle
CREATE MATERIALIZED VIEW future_5_min_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('5 minutes', timestamp) AS bucket,
        pair_id,
        FIRST(price, timestamp) AS "open",
        MAX(price) AS high,
        MIN(price) AS low,
        LAST(price, timestamp) AS "close"
    FROM future_entry
    GROUP BY bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('future_5_min_candle',
    start_offset => INTERVAL '15 minutes',
    end_offset => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '5 minutes');

-- 1 minute candle
CREATE MATERIALIZED VIEW future_1_min_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 minute', timestamp) AS bucket,
        pair_id,
        FIRST(price, timestamp) AS "open",
        MAX(price) AS high,
        MIN(price) AS low,
        LAST(price, timestamp) AS "close"
    FROM future_entry
    GROUP BY bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('future_1_min_candle',
    start_offset => INTERVAL '3 minutes',
    end_offset => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute');