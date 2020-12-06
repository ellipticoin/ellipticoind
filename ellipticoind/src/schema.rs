table! {
    addresses (id) {
        id -> Int4,
        bytes -> Bytea,
    }
}

table! {
    tokens (id) {
        id -> Int4,
        name -> Varchar,
        id_bytes -> Bytea,
        network_id -> Int4,
    }
}

table! {
    balances (id) {
        id -> Int4,
        balance -> BigInt,
        token_id -> Int4,
    }
}

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
    ledger_entries (id) {
        id -> Int4,
        transaction_id -> Int4,
        token_id -> Int4,
        amount -> BigInt,
        credit_id -> Int4,
        debit_id -> Int4,
    }
}

table! {
    networks (id) {
        id -> Int4,
        name -> Varchar,
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

joinable!(ledger_entries -> transactions (transaction_id));
joinable!(transactions -> blocks (block_number));

allow_tables_to_appear_in_same_query!(addresses, blocks, hash_onion, ledger_entries, transactions,);
