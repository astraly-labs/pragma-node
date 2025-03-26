SELECT create_hypertable('mainnet_spot_entry', by_range('timestamp', INTERVAL '1 day'));
SELECT create_hypertable('spot_entry', by_range('timestamp', INTERVAL '1 day'));
SELECT create_hypertable('mainnet_future_entry', by_range('timestamp', INTERVAL '1 day'));
SELECT create_hypertable('future_entry', by_range('timestamp', INTERVAL '1 day'));
SELECT create_hypertable('mainnet_spot_checkpoints', by_range('timestamp', INTERVAL '1 day'));
SELECT create_hypertable('spot_checkpoints', by_range('timestamp', INTERVAL '1 day'));
SELECT create_hypertable('vrf_requests', by_range('updated_at', INTERVAL '1 day'));
SELECT create_hypertable('oo_requests', by_range('updated_at', INTERVAL '1 day'));
