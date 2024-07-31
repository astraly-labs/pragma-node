CREATE INDEX spot_idx_publisher_pair_timestamp ON spot_entry (publisher, pair_id, timestamp);
CREATE INDEX mainnet_spot_idx_publisher_pair_timestamp ON mainnet_spot_entry (publisher, pair_id, timestamp);
CREATE INDEX future_idx_publisher_pair_timestamp ON future_entry (publisher, pair_id, timestamp);
CREATE INDEX mainnet_future_idx_publisher_pair_timestamp ON mainnet_future_entry (publisher, pair_id, timestamp);

CREATE INDEX spot_idx_publisher_pair_id ON spot_entry (publisher, pair_id);
CREATE INDEX mainnet_spot_idx_publisher_pair_id ON mainnet_spot_entry (publisher, pair_id);
CREATE INDEX future_idx_publisher_pair_id ON future_entry (publisher, pair_id);
CREATE INDEX mainnet_future_idx_publisher_pair_id ON mainnet_future_entry (publisher, pair_id);

CREATE INDEX spot_idx_pair_id ON spot_entry (pair_id);
CREATE INDEX mainnet_spot_idx_pair_id ON mainnet_spot_entry (pair_id);
CREATE INDEX future_idx_pair_id ON future_entry (pair_id);
CREATE INDEX mainnet_future_idx_pair_id ON mainnet_future_entry (pair_id);

CREATE INDEX spot_idx_publisher_timestamp ON spot_entry (publisher, timestamp);
CREATE INDEX mainnet_spot_idx_publisher_timestamp ON mainnet_spot_entry (publisher, timestamp);
CREATE INDEX future_idx_publisher_timestamp ON future_entry (publisher, timestamp);
CREATE INDEX mainnet_future_idx_publisher_timestamp ON mainnet_future_entry (publisher, timestamp);