-- 1 day candle
CREATE MATERIALIZED VIEW mainnet_spot_1_day_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 day', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM mainnet_spot_price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;


SELECT add_continuous_aggregate_policy('mainnet_spot_1_day_candle',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');

-- 1 hour candle
CREATE MATERIALIZED VIEW mainnet_spot_1_hour_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 hour', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM mainnet_spot_price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_spot_1_hour_candle',
    start_offset => INTERVAL '3 hours',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');

-- 15 minute candle
CREATE MATERIALIZED VIEW mainnet_spot_15_min_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('15 minutes', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket)::numeric AS "open",
        MAX(median_price)::numeric AS high,
        MIN(median_price)::numeric AS low,
        LAST(median_price, bucket)::numeric AS "close"
    FROM mainnet_spot_price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_spot_15_min_candle',
    start_offset => INTERVAL '45 minutes',
    end_offset => INTERVAL '15 minutes',
    schedule_interval => INTERVAL '15 minutes');

-- 5 minute candle
CREATE MATERIALIZED VIEW mainnet_spot_5_min_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('5 minutes', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM mainnet_spot_price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_spot_5_min_candle',
    start_offset => INTERVAL '15 minutes',
    end_offset => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '5 minutes');

-- 1 minute candle
CREATE MATERIALIZED VIEW mainnet_spot_1_min_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 minute', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM mainnet_spot_price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_spot_1_min_candle',
    start_offset => INTERVAL '3 minutes',
    end_offset => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute');