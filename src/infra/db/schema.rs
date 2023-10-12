// @generated automatically by Diesel CLI.

diesel::table! {
    entries (id) {
        id -> Uuid,
        pair_id -> Varchar,
        publisher -> Text,
        timestamp -> Timestamp,
        price -> Numeric,
    }
}
