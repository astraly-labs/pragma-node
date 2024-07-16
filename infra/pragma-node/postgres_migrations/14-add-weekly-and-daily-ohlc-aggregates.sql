-- testnet spot
CREATE MATERIALIZED VIEW spot_1_week_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 week', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM spot_price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('spot_1_week_candle',
    start_offset => INTERVAL '3 week',
    end_offset => INTERVAL '1 week',
    schedule_interval => INTERVAL '1 week');

CREATE MATERIALIZED VIEW spot_1_day_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 day', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM spot_price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('spot_1_day_candle',
    start_offset => INTERVAL '3 day',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');

--testnet future
CREATE MATERIALIZED VIEW future_1_week_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 week', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM future_price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('future_1_week_candle',
    start_offset => INTERVAL '3 week',
    end_offset => INTERVAL '1 week',
    schedule_interval => INTERVAL '1 week');


CREATE MATERIALIZED VIEW future_1_day_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 day', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM future_price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('future_1_day_candle',
    start_offset => INTERVAL '3 day',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');


-- mainnet spot
CREATE MATERIALIZED VIEW mainnet_spot_1_week_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 week', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM mainnet_spot_price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_spot_1_week_candle',
    start_offset => INTERVAL '3 week',
    end_offset => INTERVAL '1 week',
    schedule_interval => INTERVAL '1 week');


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
    start_offset => INTERVAL '3 day',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');


-- mainnet future
-- 1 week candle
CREATE MATERIALIZED VIEW mainnet_future_1_week_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 week', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM mainnet_future_price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_future_1_week_candle',
    start_offset => INTERVAL '3 week',
    end_offset => INTERVAL '1 week',
    schedule_interval => INTERVAL '1 week');

-- 1 day candle
CREATE MATERIALIZED VIEW mainnet_future_1_day_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 day', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM mainnet_future_price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('mainnet_future_1_day_candle',
    start_offset => INTERVAL '3 day',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');

