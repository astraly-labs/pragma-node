-- Drop materialized views for spot
DROP MATERIALIZED VIEW median_100_ms_spot;
DROP MATERIALIZED VIEW median_100_ms_spot_per_source;
DROP MATERIALIZED VIEW median_1_s_spot;
DROP MATERIALIZED VIEW median_1_s_spot_per_source;
DROP MATERIALIZED VIEW median_5_s_spot;
DROP MATERIALIZED VIEW median_5_s_spot_per_source;
DROP MATERIALIZED VIEW median_10_s_spot;
DROP MATERIALIZED VIEW median_10_s_spot_per_source;
DROP MATERIALIZED VIEW median_1_min_spot;
DROP MATERIALIZED VIEW median_1_min_spot_per_source;
DROP MATERIALIZED VIEW median_15_min_spot;
DROP MATERIALIZED VIEW median_15_min_spot_per_source;
DROP MATERIALIZED VIEW median_1_h_spot;
DROP MATERIALIZED VIEW median_1_h_spot_per_source;
DROP MATERIALIZED VIEW median_2_h_spot;
DROP MATERIALIZED VIEW median_2_h_spot_per_source;
DROP MATERIALIZED VIEW median_1_day_spot;
DROP MATERIALIZED VIEW median_1_day_spot_per_source;
DROP MATERIALIZED VIEW median_1_week_spot;
DROP MATERIALIZED VIEW median_1_week_spot_per_source;

-- Drop materialized views for perp
DROP MATERIALIZED VIEW median_100_ms_perp;
DROP MATERIALIZED VIEW median_100_ms_perp_per_source;
DROP MATERIALIZED VIEW median_1_s_perp;
DROP MATERIALIZED VIEW median_1_s_perp_per_source;
DROP MATERIALIZED VIEW median_5_s_perp;
DROP MATERIALIZED VIEW median_5_s_perp_per_source;
DROP MATERIALIZED VIEW median_10_s_perp;
DROP MATERIALIZED VIEW median_10_s_perp_per_source;
DROP MATERIALIZED VIEW median_1_min_perp;
DROP MATERIALIZED VIEW median_1_min_perp_per_source;
DROP MATERIALIZED VIEW median_15_min_perp;
DROP MATERIALIZED VIEW median_15_min_perp_per_source;
DROP MATERIALIZED VIEW median_1_h_perp;
DROP MATERIALIZED VIEW median_1_h_perp_per_source;
DROP MATERIALIZED VIEW median_2_h_perp;
DROP MATERIALIZED VIEW median_2_h_perp_per_source;
DROP MATERIALIZED VIEW median_1_day_perp;
DROP MATERIALIZED VIEW median_1_day_perp_per_source;
DROP MATERIALIZED VIEW median_1_week_perp;
DROP MATERIALIZED VIEW median_1_week_perp_per_source;

-- Drop the function
DROP FUNCTION create_median_aggregate(text, interval, interval, text);

-- Drop the custom type
DROP TYPE price_component;
