CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE mainnet_spot_entry (
    network character varying(255),
    pair_id character varying(255),
    data_id character varying(255) NOT NULL,
    block_hash character varying(255),
    block_number bigint,
    block_timestamp timestamp without time zone,
    transaction_hash character varying(255),
    price numeric,
    timestamp timestamp without time zone,
    publisher character varying(255),
    source character varying(255),
    volume numeric,
    _cursor bigint
);

CREATE TABLE spot_entry (
    network character varying(255),
    pair_id character varying(255),
    data_id character varying(255) NOT NULL,
    block_hash character varying(255),
    block_number bigint,
    block_timestamp timestamp without time zone,
    transaction_hash character varying(255),
    price numeric,
    timestamp timestamp without time zone,
    publisher character varying(255),
    source character varying(255),
    volume numeric,
    _cursor bigint
);

CREATE TABLE mainnet_future_entry (
    network character varying(255),
    pair_id character varying(255),
    data_id character varying(255),
    block_hash character varying(255),
    block_number bigint,
    block_timestamp timestamp without time zone,
    transaction_hash character varying(255),
    price numeric,
    timestamp timestamp without time zone,
    publisher character varying(255),
    source character varying(255),
    volume numeric,
    _cursor bigint,
    expiration_timestamp timestamp without time zone
);

CREATE TABLE future_entry (
    network character varying(255),
    pair_id character varying(255),
    data_id character varying(255),
    block_hash character varying(255),
    block_number bigint,
    block_timestamp timestamp without time zone,
    transaction_hash character varying(255),
    price numeric,
    timestamp timestamp without time zone,
    publisher character varying(255),
    source character varying(255),
    volume numeric,
    _cursor bigint,
    expiration_timestamp timestamp without time zone
);

CREATE TABLE mainnet_spot_checkpoints (
    network character varying(255),
    pair_id character varying(255),
    data_id character varying(255) NOT NULL,
    block_hash character varying(255),
    block_number bigint,
    block_timestamp timestamp without time zone,
    transaction_hash character varying(255),
    price numeric,
    sender_address character varying(255),
    aggregation_mode numeric,
    _cursor bigint,
    timestamp timestamp without time zone,
    nb_sources_aggregated numeric
);

CREATE TABLE spot_checkpoints (
    network character varying(255),
    pair_id character varying(255),
    data_id character varying(255) NOT NULL,
    block_hash character varying(255),
    block_number bigint,
    block_timestamp timestamp without time zone,
    transaction_hash character varying(255),
    price numeric,
    sender_address character varying(255),
    aggregation_mode numeric,
    _cursor bigint,
    timestamp timestamp without time zone,
    nb_sources_aggregated numeric
);

CREATE TABLE publishers (
    name VARCHAR NOT NULL,
    website_url VARCHAR NOT NULL,
    mainnet_address VARCHAR,
    testnet_address VARCHAR,
    publisher_type INTEGER NOT NULL CHECK (publisher_type IN (0, 1))
);

CREATE TYPE price_component AS (
    source text,
    price numeric(1000,0),
    "timestamp" timestamptz
);
