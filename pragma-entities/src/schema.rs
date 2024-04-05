// @generated automatically by Diesel CLI.

diesel::table! {
    currencies (id) {
        id -> Uuid,
        name -> Varchar,
        decimals -> Numeric,
        #[sql_name = "abstract"]
        abstract_ -> Bool,
        ethereum_address -> Nullable<Varchar>,
    }
}

diesel::table! {
    entries (id, timestamp) {
        id -> Uuid,
        pair_id -> Varchar,
        publisher -> Text,
        timestamp -> Timestamptz,
        price -> Numeric,
        source -> Varchar,
    }
}

diesel::table! {
    publishers (id) {
        id -> Uuid,
        name -> Varchar,
        master_key -> Varchar,
        active_key -> Varchar,
        active -> Bool,
        account_address -> Varchar,
    }
}

diesel::table! {
    future_entry (data_id) {
        #[max_length = 255]
        network -> Varchar,
        #[max_length = 255]
        pair_id -> Varchar,
        #[max_length = 255]
        data_id -> Varchar,
        #[max_length = 255]
        block_hash -> Varchar,
        block_number -> Int8,
        block_timestamp -> Timestamp,
        #[max_length = 255]
        transaction_hash -> Varchar,
        price -> Numeric,
        timestamp -> Timestamp,
        #[max_length = 255]
        publisher -> Varchar,
        #[max_length = 255]
        source -> Varchar,
        volume -> Numeric,
        expiration_timestamp -> Nullable<Timestamp>,
        _cursor -> Int8,
    }
}

diesel::table! {
    spot_entry (timestamp) {
        #[max_length = 255]
        network -> Varchar,
        #[max_length = 255]
        pair_id -> Varchar,
        #[max_length = 255]
        data_id -> Varchar,
        #[max_length = 255]
        block_hash -> Varchar,
        block_number -> Int8,
        block_timestamp -> Timestamp,
        #[max_length = 255]
        transaction_hash -> Varchar,
        price -> Numeric,
        timestamp -> Timestamp,
        #[max_length = 255]
        publisher -> Varchar,
        #[max_length = 255]
        source -> Varchar,
        volume -> Numeric,
        _cursor -> Int8,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    currencies,
    spot_entry,
    future_entry,
    entries,
    publishers,
);
