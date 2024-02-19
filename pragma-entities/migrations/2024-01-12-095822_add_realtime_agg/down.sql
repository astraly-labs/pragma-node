-- This file should undo anything in `up.sql`
ALTER MATERIALIZED VIEW price_1_min_agg set (timescaledb.materialized_only = true);
ALTER MATERIALIZED VIEW price_15_min_agg set (timescaledb.materialized_only = true);
ALTER MATERIALIZED VIEW price_1_h_agg set (timescaledb.materialized_only = true);