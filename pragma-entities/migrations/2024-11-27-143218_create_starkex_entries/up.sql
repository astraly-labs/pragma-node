-- Your SQL goes here

CREATE TABLE starkex_entries (
    id SERIAL PRIMARY KEY,
    pair_id VARCHAR NOT NULL,
    publisher VARCHAR NOT NULL,
    source VARCHAR NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    publisher_signature VARCHAR NOT NULL,
    price NUMERIC NOT NULL,
    CONSTRAINT unique_starkex_entry UNIQUE (pair_id, publisher, source, timestamp)
);

CREATE INDEX idx_starkex_entries_pair_id ON starkex_entries(pair_id);
CREATE INDEX idx_starkex_entries_timestamp ON starkex_entries(timestamp);
CREATE INDEX idx_starkex_entries_publisher ON starkex_entries(publisher);

SELECT create_hypertable('starkex_entries', 'timestamp');


CREATE TABLE starkex_future_entries (
    id SERIAL PRIMARY KEY,
    pair_id VARCHAR NOT NULL,
    publisher VARCHAR NOT NULL,
    source VARCHAR NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    expiration_timestamp TIMESTAMPTZ,
    publisher_signature VARCHAR NOT NULL,
    price NUMERIC NOT NULL,
    CONSTRAINT unique_starkex_future_entry UNIQUE (pair_id, publisher, source, timestamp, expiration_timestamp)
);

CREATE INDEX idx_starkex_future_entries_pair_id ON starkex_future_entries(pair_id);
CREATE INDEX idx_starkex_future_entries_timestamp ON starkex_future_entries(timestamp);
CREATE INDEX idx_starkex_future_entries_publisher ON starkex_future_entries(publisher);

SELECT create_hypertable('starkex_future_entries', 'timestamp');
