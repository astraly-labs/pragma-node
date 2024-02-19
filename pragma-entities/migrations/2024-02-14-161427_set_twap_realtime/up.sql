-- Your SQL goes here
ALTER MATERIALIZED VIEW twap_1_min_agg set (timescaledb.materialized_only = false);
ALTER MATERIALIZED VIEW twap_15_min_agg set (timescaledb.materialized_only = false);
ALTER MATERIALIZED VIEW twap_1_hour_agg set (timescaledb.materialized_only = false);