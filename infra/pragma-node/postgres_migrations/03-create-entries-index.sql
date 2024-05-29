CREATE INDEX idx_spot_entry_publisher_pair_id ON spot_entry (publisher, pair_id);
CREATE INDEX idx_mainnet_spot_entry_publisher_pair_id ON mainnet_spot_entry (publisher, pair_id);
CREATE INDEX idx_future_entry_publisher_pair_id ON future_entry (publisher, pair_id);
CREATE INDEX idx_mainnet_future_entry_publisher_pair_id ON mainnet_future_entry (publisher, pair_id);

CREATE INDEX idx_spot_entry_publisher_pair_id_timestamp ON spot_entry (publisher, pair_id, timestamp DESC);
CREATE INDEX idx_mainnet_spot_entry_publisher_pair_id_timestamp ON mainnet_spot_entry (publisher, pair_id, timestamp DESC);
CREATE INDEX idx_future_entry_publisher_pair_id_timestamp ON future_entry (publisher, pair_id, timestamp DESC);
CREATE INDEX idx_mainnet_future_entry_publisher_pair_id_timestamp ON mainnet_future_entry (publisher, pair_id, timestamp DESC);
