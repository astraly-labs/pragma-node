-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE entries (
  id uuid DEFAULT uuid_generate_v4(),
  pair_id VARCHAR NOT NULL,
  publisher TEXT NOT NULL,
  timestamp TIMESTAMPTZ NOT NULL,
  price NUMERIC NOT NULL,
  PRIMARY KEY (id, timestamp)
);

CREATE INDEX entries_pair_id_timestamp_idx ON entries (pair_id, timestamp DESC);
