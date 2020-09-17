table! {
    blocks (hash) {
        hash -> Bytea,
        parent_hash -> Nullable<Bytea>,
        winner -> Bytea,
        number -> Int8,
        memory_changeset_hash -> Bytea,
        storage_changeset_hash -> Bytea,
        sealed -> Bool,
    }
}

table! {
    hash_onion (id) {
        id -> Int4,
        layer -> Bytea,
    }
}

table! {
    transactions (hash) {
        network_id -> Int8,
        block_hash -> Bytea,
        hash -> Bytea,
        position -> Int8,
        contract -> Varchar,
        sender -> Bytea,
        nonce -> Int8,
        function -> Varchar,
        arguments -> Bytea,
        return_value -> Bytea,
        signature -> Bytea,
    }
}

joinable!(transactions -> blocks (block_hash));

allow_tables_to_appear_in_same_query!(
    blocks,
    hash_onion,
    transactions,
);
