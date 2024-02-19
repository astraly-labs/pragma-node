-- Your SQL goes here
ALTER MATERIALIZED VIEW price_1_min_agg set (timescaledb.materialized_only = false);
ALTER MATERIALIZED VIEW price_15_min_agg set (timescaledb.materialized_only = false);
ALTER MATERIALIZED VIEW price_1_h_agg set (timescaledb.materialized_only = false);