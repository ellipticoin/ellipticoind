CREATE TABLE "transactions" (
  "id" SERIAL PRIMARY KEY,
  "network_id" BIGINT NOT NULL,
  "block_number" INTEGER REFERENCES "blocks" NOT NULL,
  "position" INTEGER NOT NULL,
  "contract" VARCHAR NOT NULL,
  "sender" BYTEA NOT NULL,
  "nonce" INTEGER NOT NULL,
  "function" VARCHAR NOT NULL,
  "arguments" BYTEA NOT NULL,
  "return_value" BYTEA NOT NULL,
  "raw" BYTEA NOT NULL
);
