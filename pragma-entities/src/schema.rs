// @generated automatically by Diesel CLI.

diesel::table! {
    entries (id, timestamp) {
        id -> Uuid,
        pair_id -> Varchar,
        price -> Numeric,
        timestamp -> Timestamptz,
        publisher -> Text,
        publisher_signature -> Nullable<Text>,
        source -> Varchar,
    }
}

diesel::table! {
    funding_rates (id, timestamp) {
        id -> Uuid,
        source -> Varchar,
        pair -> Varchar,
        annualized_rate -> Float8,
        timestamp -> Timestamptz,
        created_at -> Timestamptz,
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
        publisher_signature -> Nullable<Text>,
        source -> Varchar,
    }
}

diesel::table! {
    publishers (id) {
        id -> Uuid,
        name -> Varchar,
        master_key -> Varchar,
        active_key -> Varchar,
        account_address -> Varchar,
        active -> Bool,
    }
}

diesel::allow_tables_to_appear_in_same_query!(entries, funding_rates, future_entries, publishers,);
