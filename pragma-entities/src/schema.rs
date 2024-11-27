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
        publisher_signature -> Nullable<Varchar>,
    }
}

diesel::table! {
    starkex_future_entries (id, timestamp) {
        id -> Uuid,
        pair_id -> Varchar,
        publisher -> Text,
        timestamp -> Timestamptz,
        expiration_timestamp -> Nullable<Timestamptz>,
        price -> Numeric,
        source -> Varchar,
        publisher_signature -> Varchar,
    }
}

diesel::table! {
    starkex_entries (id, timestamp) {
        id -> Uuid,
        pair_id -> Varchar,
        publisher -> Text,
        timestamp -> Timestamptz,
        price -> Numeric,
        source -> Varchar,
        publisher_signature -> Nullable<Varchar>,
    }
}

diesel::table! {
    future_entries (id, timestamp) {
        id -> Uuid,
        pair_id -> Varchar,
        publisher -> Text,
        timestamp -> Timestamptz,
        expiration_timestamp -> Nullable<Timestamptz>,
        price -> Numeric,
        source -> Varchar,
        publisher_signature -> Varchar,
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

diesel::allow_tables_to_appear_in_same_query!(currencies, entries, future_entries, publishers,);
