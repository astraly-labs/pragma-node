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
    entries (id) {
        id -> Uuid,
        pair_id -> Varchar,
        publisher -> Text,
        timestamp -> Timestamp,
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

diesel::allow_tables_to_appear_in_same_query!(currencies, entries, publishers,);
