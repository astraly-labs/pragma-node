-- Drop materialized views for spot twap (main views first)
DROP MATERIALIZED VIEW IF EXISTS twap_1_min_spot;
DROP MATERIALIZED VIEW IF EXISTS twap_5_min_spot;
DROP MATERIALIZED VIEW IF EXISTS twap_15_min_spot;
DROP MATERIALIZED VIEW IF EXISTS twap_1_h_spot;
DROP MATERIALIZED VIEW IF EXISTS twap_2_h_spot;
DROP MATERIALIZED VIEW IF EXISTS twap_1_day_spot;

-- Drop materialized views for spot twap (per-source views)
DROP MATERIALIZED VIEW IF EXISTS twap_1_min_spot_per_source;
DROP MATERIALIZED VIEW IF EXISTS twap_5_min_spot_per_source;
DROP MATERIALIZED VIEW IF EXISTS twap_15_min_spot_per_source;
DROP MATERIALIZED VIEW IF EXISTS twap_1_h_spot_per_source;
DROP MATERIALIZED VIEW IF EXISTS twap_2_h_spot_per_source;
DROP MATERIALIZED VIEW IF EXISTS twap_1_day_spot_per_source;

-- Drop materialized views for perp twap (main views first)
DROP MATERIALIZED VIEW IF EXISTS twap_1_min_perp;
DROP MATERIALIZED VIEW IF EXISTS twap_5_min_perp;
DROP MATERIALIZED VIEW IF EXISTS twap_15_min_perp;
DROP MATERIALIZED VIEW IF EXISTS twap_1_h_perp;
DROP MATERIALIZED VIEW IF EXISTS twap_2_h_perp;
DROP MATERIALIZED VIEW IF EXISTS twap_1_day_perp;

-- Drop materialized views for perp twap (per-source views)
DROP MATERIALIZED VIEW IF EXISTS twap_1_min_perp_per_source;
DROP MATERIALIZED VIEW IF EXISTS twap_5_min_perp_per_source;
DROP MATERIALIZED VIEW IF EXISTS twap_15_min_perp_per_source;
DROP MATERIALIZED VIEW IF EXISTS twap_1_h_perp_per_source;
DROP MATERIALIZED VIEW IF EXISTS twap_2_h_perp_per_source;
DROP MATERIALIZED VIEW IF EXISTS twap_1_day_perp_per_source;

-- Drop the function
DROP FUNCTION IF EXISTS create_twap_aggregate(text, interval, interval, text);
