table! {
    blocks (number) {
        number -> Int4,
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
    transactions (id) {
        id -> Int4,
        network_id -> Int8,
        block_number -> Int4,
        position -> Int4,
        contract -> Varchar,
        sender -> Bytea,
        nonce -> Int4,
        function -> Varchar,
        arguments -> Bytea,
        return_value -> Bytea,
        raw -> Bytea,
    }
}

joinable!(transactions -> blocks (block_number));

allow_tables_to_appear_in_same_query!(blocks, hash_onion, transactions,);
