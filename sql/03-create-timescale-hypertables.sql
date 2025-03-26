SELECT create_hypertable('mainnet_spot_entry', by_range('timestamp', INTERVAL '1 day'));
SELECT create_hypertable('spot_entry', by_range('timestamp', INTERVAL '1 day'));
SELECT create_hypertable('mainnet_future_entry', by_range('timestamp', INTERVAL '1 day'));
SELECT create_hypertable('future_entry', by_range('timestamp', INTERVAL '1 day'));
SELECT create_hypertable('mainnet_spot_checkpoints', by_range('timestamp', INTERVAL '1 day'));
SELECT create_hypertable('spot_checkpoints', by_range('timestamp', INTERVAL '1 day'));

ALTER TABLE mainnet_spot_entry SET (
    timescaledb.enable_columnstore = true,
    timescaledb.segmentby = 'pair_id'
);
CALL add_columnstore_policy('mainnet_spot_entry', after => INTERVAL '1d');

ALTER TABLE spot_entry SET (
    timescaledb.enable_columnstore = true,
    timescaledb.segmentby = 'pair_id'
);
CALL add_columnstore_policy('spot_entry', after => INTERVAL '1d');

ALTER TABLE mainnet_future_entry SET (
    timescaledb.enable_columnstore = true,
    timescaledb.segmentby = 'pair_id'
);
CALL add_columnstore_policy('mainnet_future_entry', after => INTERVAL '1d');

ALTER TABLE future_entry SET (
    timescaledb.enable_columnstore = true,
    timescaledb.segmentby = 'pair_id'
);
CALL add_columnstore_policy('future_entry', after => INTERVAL '1d');
