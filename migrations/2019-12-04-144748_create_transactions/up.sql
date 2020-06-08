CREATE TABLE "transactions" (
  "block_hash" BYTEA CONSTRAINT "transactions_block_hash_fkey" REFERENCES "blocks" ("hash") NOT NULL,
  "hash" BYTEA NOT NULL,
  "contract_address" BYTEA NOT NULL,
  "sender" BYTEA NOT NULL,
  "gas_limit" BIGINT NOT NULL,
  "nonce" BIGINT NOT NULL,
  "function" VARCHAR NOT NULL,
  "arguments" BYTEA NOT NULL,
  "return_value" BYTEA NOT NULL,
  PRIMARY KEY ("hash")
);

CREATE INDEX "transactions_block_hash_index" ON "transactions" ("block_hash");
CREATE INDEX "transactions_contract_address_index" ON "transactions" ("contract_address");
