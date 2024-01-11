-- Your SQL goes here
CREATE MATERIALIZED VIEW price_1_min_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = true)
AS SELECT source,
    time_bucket('1 min'::interval, ts) as bucket,
    percentile_agg(price)
FROM entries
GROUP BY source, bucket;

CREATE MATERIALIZED VIEW price_15_min_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = true)
AS SELECT source,
    time_bucket('15 min'::interval, ts) as bucket,
    percentile_agg(price)
FROM entries
GROUP BY source, bucket;

CREATE MATERIALIZED VIEW price_1_h_agg
WITH (timescaledb.continuous, timescaledb.materialized_only = true)
AS SELECT source,
    time_bucket('1 h'::interval, ts) as bucket,
    percentile_agg(price)
FROM entries
GROUP BY source, bucket;