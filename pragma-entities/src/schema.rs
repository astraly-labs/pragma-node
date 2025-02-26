// @generated automatically by Diesel CLI.

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
    future_entries (id, timestamp) {
        id -> Uuid,
        pair_id -> Varchar,
        price -> Numeric,
        timestamp -> Timestamptz,
        expiration_timestamp -> Nullable<Timestamptz>,
        publisher -> Text,
        publisher_signature -> Text,
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

diesel::allow_tables_to_appear_in_same_query!(entries, future_entries, publishers,);
