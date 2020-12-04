CREATE TABLE "blocks" (
  "number" INTEGER PRIMARY KEY,
  "memory_changeset_hash" BYTEA NOT NULL,
  "storage_changeset_hash" BYTEA NOT NULL,
  "sealed" BOOL NOT NULL DEFAULT false
);
