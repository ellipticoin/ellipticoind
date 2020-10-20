CREATE TABLE "blocks" (
  "number" SERIAL PRIMARY KEY,
  "memory_changeset_hash" BYTEA NOT NULL,
  "storage_changeset_hash" BYTEA NOT NULL,
  "sealed" BOOL NOT NULL DEFAULT false
);
ALTER SEQUENCE blocks_number_seq RESTART WITH 1
