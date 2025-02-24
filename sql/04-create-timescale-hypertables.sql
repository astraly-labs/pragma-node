SELECT create_hypertable('mainnet_spot_entry', 'timestamp');
SELECT create_hypertable('spot_entry', 'timestamp');
SELECT create_hypertable('mainnet_future_entry', 'timestamp');
SELECT create_hypertable('future_entry', 'timestamp');
SELECT create_hypertable('mainnet_spot_checkpoints', 'timestamp');
SELECT create_hypertable('spot_checkpoints', 'timestamp');
SELECT create_hypertable('vrf_requests', 'updated_at');
SELECT create_hypertable('oo_requests', 'updated_at');
