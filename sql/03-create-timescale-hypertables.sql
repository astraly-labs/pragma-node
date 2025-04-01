SELECT create_hypertable('mainnet_spot_entry', by_range('timestamp', INTERVAL '7 days'));
SELECT create_hypertable('spot_entry', by_range('timestamp', INTERVAL '7 days'));
SELECT create_hypertable('mainnet_future_entry', by_range('timestamp', INTERVAL '7 days'));
SELECT create_hypertable('future_entry', by_range('timestamp', INTERVAL '7 days'));
SELECT create_hypertable('mainnet_spot_checkpoints', by_range('timestamp', INTERVAL '7 days'));
SELECT create_hypertable('spot_checkpoints', by_range('timestamp', INTERVAL '7 days'));

ALTER TABLE mainnet_spot_entry SET (
    timescaledb.segmentby = 'pair_id'
);

ALTER TABLE spot_entry SET (
    timescaledb.segmentby = 'pair_id'
);

ALTER TABLE mainnet_future_entry SET (
    timescaledb.segmentby = 'pair_id'
);

ALTER TABLE future_entry SET (
    timescaledb.segmentby = 'pair_id'
);
