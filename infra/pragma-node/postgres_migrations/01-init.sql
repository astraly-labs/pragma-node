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

CREATE TABLE publishers (
    id uuid DEFAULT uuid_generate_v4(),
    name VARCHAR NOT NULL,
    website_url VARCHAR NOT NULL,
    publisher_type INTEGER NOT NULL CHECK (publisher_type IN (0, 1)), -- 0 = first party, 1 = 3rd party
    master_key VARCHAR NOT NULL,
    active_key VARCHAR NOT NULL,
    active BOOLEAN NOT NULL,
    account_address VARCHAR NOT NULL DEFAULT '',
    PRIMARY KEY (id)
);

INSERT INTO public.publishers (name, website_url, publisher_type, master_key, active_key, active, account_address) 
VALUES
(
    'PRAGMA',
    'https://www.pragma.build/',
    1,
    '0x05e6361b53afbb451d1326ed4e37aecff9ef68af8318eb3c8dc58bcadfc16705',
    '0x05e6361b53afbb451d1326ed4e37aecff9ef68af8318eb3c8dc58bcadfc16705', 
    true, 
    '0x624EBFB99865079BD58CFCFB925B6F5CE940D6F6E41E118B8A72B7163FB435C'
),
(
    'AVNU',
    'https://www.avnu.fi/',
    0,
    '0x0000000000000000000000000000000000000000000000000000000000000042',
    '0x0000000000000000000000000000000000000000000000000000000000000042',
    true, 
    '0x624EBFB99865079BD58CFCFB925FAKEFAKEFAKE6E41E118B8A72B7163FB435C'
);
