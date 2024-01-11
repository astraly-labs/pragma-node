-- Your SQL goes here
SELECT create_hypertable('entries', by_range('timestamp'));
SELECT * FROM add_dimension('entries', by_hash('pair_id', 4))
SELECT * FROM add_dimension('entries', by_hash('source', 4))