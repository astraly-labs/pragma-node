// @generated automatically by Diesel CLI.

diesel::table! {
    entries (id) {
        id -> Int4,
        pair_id -> Varchar,
        publisher -> Text,
        timestamp -> Timestamp,
        price -> Numeric,
    }
}
