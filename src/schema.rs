table! {
    blocks (hash) {
        hash -> Bytea,
        parent_hash -> Nullable<Bytea>,
        number -> Int8,
        winner -> Bytea,
        memory_changeset_hash -> Bytea,
        storage_changeset_hash -> Bytea,
        proof_of_work_value -> Int8,
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
        block_hash -> Bytea,
        hash -> Bytea,
        contract_address -> Bytea,
        sender -> Bytea,
        gas_limit -> Int8,
        nonce -> Int8,
        function -> Varchar,
        arguments -> Bytea,
        return_value -> Bytea,
    }
}

joinable!(transactions -> blocks (block_hash));

allow_tables_to_appear_in_same_query!(blocks, hash_onion, transactions,);
