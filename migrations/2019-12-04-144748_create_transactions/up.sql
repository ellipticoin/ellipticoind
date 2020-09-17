CREATE TABLE "transactions" (
  "network_id" BIGINT NOT NULL,
  "block_hash" BYTEA CONSTRAINT "transactions_block_hash_fkey" REFERENCES "blocks" ("hash") NOT NULL,
  "hash" BYTEA NOT NULL,
  "position" BIGINT NOT NULL,
  "contract" VARCHAR NOT NULL,
  "sender" BYTEA NOT NULL,
  "nonce" BIGINT NOT NULL,
  "function" VARCHAR NOT NULL,
  "arguments" BYTEA NOT NULL,
  "return_value" BYTEA NOT NULL,
  "signature" BYTEA NOT NULL,
  PRIMARY KEY ("hash")
);

CREATE INDEX "transactions_block_hash_index" ON "transactions" ("block_hash");
