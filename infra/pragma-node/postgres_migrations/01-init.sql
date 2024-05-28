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

CREATE TABLE vrf_requests (
    network character varying(255),
    request_id numeric,
    seed numeric,
    created_at timestamp without time zone,
    created_at_tx character varying(255),
    callback_address character varying(255),
    callback_fee_limit numeric,
    num_words numeric,
    requestor_address character varying(255),
    updated_at timestamp without time zone,
    updated_at_tx character varying(255),
    status numeric,
    minimum_block_number numeric,
    _cursor bigint,
    data_id character varying(255)
);
