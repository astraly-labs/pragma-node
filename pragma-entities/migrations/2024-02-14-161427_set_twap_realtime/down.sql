-- This file should undo anything in `up.sql`
ALTER MATERIALIZED VIEW twap_1_min_agg set (timescaledb.materialized_only = true);
ALTER MATERIALIZED VIEW twap_15_min_agg set (timescaledb.materialized_only = true);
ALTER MATERIALIZED VIEW twap_1_hour_agg set (timescaledb.materialized_only = true);