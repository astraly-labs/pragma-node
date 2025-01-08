-- Drop dependent views first (in reverse order)
DROP MATERIALIZED VIEW new_1_week_candle;
DROP MATERIALIZED VIEW new_1_day_candle;
DROP MATERIALIZED VIEW new_1_h_candle;
DROP MATERIALIZED VIEW new_15_min_candle;
DROP MATERIALIZED VIEW new_5_min_candle;
DROP MATERIALIZED VIEW new_1_min_candle;

-- Drop and recreate the base 10s aggregate with source filtering and outlier removal
DROP MATERIALIZED VIEW price_10_s_agg;

CREATE MATERIALIZED VIEW price_10_s_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = false)
AS 
WITH price_stats AS (
    SELECT 
        pair_id,
        time_bucket('10 seconds'::interval, timestamp) as bucket,
        percentile_cont(0.25) WITHIN GROUP (ORDER BY price) AS q1,
        percentile_cont(0.75) WITHIN GROUP (ORDER BY price) AS q3,
        COUNT(DISTINCT source) as num_sources
    FROM entries
    GROUP BY bucket, pair_id
    HAVING COUNT(DISTINCT source) > 2  -- Only consider buckets with more than 2 sources
),
filtered_entries AS (
    SELECT 
        e.pair_id,
        e.timestamp,
        e.price,
        e.source
    FROM entries e
    JOIN price_stats ps 
        ON e.pair_id = ps.pair_id 
        AND time_bucket('10 seconds'::interval, e.timestamp) = ps.bucket
    WHERE e.price >= ps.q1 - 1.5 * (ps.q3 - ps.q1)  -- Filter lower outliers
        AND e.price <= ps.q3 + 1.5 * (ps.q3 - ps.q1)  -- Filter upper outliers
)
SELECT 
    pair_id,
    time_bucket('10 seconds'::interval, timestamp) as bucket,
    approx_percentile(0.5, percentile_agg(price))::numeric AS median_price,
    COUNT(DISTINCT source) as num_sources
FROM filtered_entries
GROUP BY bucket, pair_id
HAVING COUNT(DISTINCT source) > 2  -- Double-check after filtering outliers
WITH NO DATA;

SELECT add_continuous_aggregate_policy('price_10_s_agg',
  start_offset => INTERVAL '1 day',
  end_offset => INTERVAL '10 seconds',
  schedule_interval => INTERVAL '10 seconds');

-- Recreate all the OHLC views
CREATE MATERIALIZED VIEW new_1_min_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 minute', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('new_1_min_candle',
    start_offset => INTERVAL '3 minutes',
    end_offset => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute');

CREATE MATERIALIZED VIEW new_5_min_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('5 minutes', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('new_5_min_candle',
    start_offset => INTERVAL '15 minutes',
    end_offset => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '5 minutes');

CREATE MATERIALIZED VIEW new_15_min_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('15 minutes', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket)::numeric AS "open",
        MAX(median_price)::numeric AS high,
        MIN(median_price)::numeric AS low,
        LAST(median_price, bucket)::numeric AS "close"
    FROM price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('new_15_min_candle',
    start_offset => INTERVAL '45 minutes',
    end_offset => INTERVAL '15 minutes',
    schedule_interval => INTERVAL '15 minutes');

CREATE MATERIALIZED VIEW new_1_h_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 hour', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('new_1_h_candle',
    start_offset => INTERVAL '3 hours',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');

CREATE MATERIALIZED VIEW new_1_day_candle
WITH (timescaledb.continuous) AS
    SELECT
        time_bucket('1 day', bucket) AS ohlc_bucket,
        pair_id,
        FIRST(median_price, bucket) AS "open",
        MAX(median_price) AS high,
        MIN(median_price) AS low,
        LAST(median_price, bucket) AS "close"
    FROM price_10_s_agg
    GROUP BY ohlc_bucket, pair_id
    WITH NO DATA;

SELECT add_continuous_aggregate_policy('new_1_day_candle',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day');

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

-- Refresh all views with historical data
CALL refresh_continuous_aggregate('price_10_s_agg', NULL, NULL);
CALL refresh_continuous_aggregate('new_1_min_candle', NULL, NULL);
CALL refresh_continuous_aggregate('new_5_min_candle', NULL, NULL);
CALL refresh_continuous_aggregate('new_15_min_candle', NULL, NULL);
CALL refresh_continuous_aggregate('new_1_h_candle', NULL, NULL);
CALL refresh_continuous_aggregate('new_1_day_candle', NULL, NULL);
CALL refresh_continuous_aggregate('new_1_week_candle', NULL, NULL); 