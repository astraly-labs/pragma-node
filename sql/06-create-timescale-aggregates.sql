-- Create aggregations for mainnet_spot_entry
-- 10 seconds aggregation
SELECT create_price_aggregation(
    'mainnet_spot_entry',
    '10 seconds',
    '10_s',
    '1 day'
);

-- 1 day aggregation
SELECT create_price_aggregation(
    'mainnet_spot_entry',
    '1 day',
    '1_day'
);

-- 1 minute aggregation
SELECT create_price_aggregation(
    'mainnet_spot_entry',
    '1 min',
    '1_min'
);

-- 15 minutes aggregation
SELECT create_price_aggregation(
    'mainnet_spot_entry',
    '15 min',
    '15_min'
);

-- 1 hour aggregation
SELECT create_price_aggregation(
    'mainnet_spot_entry',
    '1 hour',
    '1_h'     -- Updated to match your convention
);

-- 2 hours aggregation
SELECT create_price_aggregation(
    'mainnet_spot_entry',
    '2 hour',
    '2_h'     -- Updated to match your convention
);

-- 1 week aggregation
SELECT create_price_aggregation(
    'mainnet_spot_entry',
    '1 week',
    '1_week'
);

-- Create aggregations for mainnet_future_entry
-- 10 seconds aggregation
SELECT create_price_aggregation(
    'mainnet_future_entry',
    '10 seconds',
    '10_s',
    '1 day'
);

-- 1 day aggregation
SELECT create_price_aggregation(
    'mainnet_future_entry',
    '1 day',
    '1_day'
);

-- 1 minute aggregation
SELECT create_price_aggregation(
    'mainnet_future_entry',
    '1 min',
    '1_min'
);

-- 15 minutes aggregation
SELECT create_price_aggregation(
    'mainnet_future_entry',
    '15 min',
    '15_min'
);

-- 1 hour aggregation
SELECT create_price_aggregation(
    'mainnet_future_entry',
    '1 hour',
    '1_h'     -- Updated to match your convention
);

-- 2 hours aggregation
SELECT create_price_aggregation(
    'mainnet_future_entry',
    '2 hour',
    '2_h'     -- Updated to match your convention
);

-- 1 week aggregation
SELECT create_price_aggregation(
    'mainnet_future_entry',
    '1 week',
    '1_week'
);