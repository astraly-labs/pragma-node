-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- SPOT entries

CREATE TABLE entries (
  id uuid DEFAULT uuid_generate_v4(),
  pair_id VARCHAR NOT NULL,
  price NUMERIC NOT NULL,
  timestamp TIMESTAMPTZ NOT NULL,
  publisher TEXT NOT NULL,
  publisher_signature TEXT,
  source VARCHAR NOT NULL,
  PRIMARY KEY (id, timestamp)
);

CREATE UNIQUE INDEX idx_entries_unique
  ON entries(pair_id, source, timestamp DESC);
CREATE INDEX entries_pair_id_timestamp_idx ON entries (pair_id, timestamp DESC);

SELECT
  create_hypertable('entries', 'timestamp');

-- FUTURE (PERP) entries

CREATE TABLE future_entries (
  id uuid DEFAULT uuid_generate_v4(),
  pair_id VARCHAR NOT NULL,
  price NUMERIC NOT NULL,
  timestamp TIMESTAMPTZ NOT NULL,
  expiration_timestamp TIMESTAMPTZ, -- can be NULL for perp contracts
  publisher TEXT NOT NULL,
  publisher_signature TEXT,
  source VARCHAR NOT NULL,
  PRIMARY KEY (id, timestamp)
);

CREATE UNIQUE INDEX idx_future_entries_unique ON future_entries(pair_id, source, timestamp, expiration_timestamp);
CREATE INDEX idx_future_entries_pair_id_timestamp ON future_entries (pair_id, timestamp DESC);
CREATE INDEX idx_future_entries_pair_id_timestamp_expiration_timestamp ON future_entries (pair_id, expiration_timestamp, timestamp DESC);

SELECT
  create_hypertable('future_entries', 'timestamp');
