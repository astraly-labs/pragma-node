-- 500ms aggregation view with components
CREATE TYPE entry_component AS (
    pair_id varchar,
    price numeric,
    timestamp timestamptz,
    publisher text,
    publisher_signature varchar
);
