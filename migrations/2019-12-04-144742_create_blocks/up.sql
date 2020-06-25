CREATE TABLE "blocks" (
  "hash" BYTEA NOT NULL,
  "parent_hash" BYTEA, --CONSTRAINT "blocks_parent_hash_fkey" REFERENCES "blocks" ("hash"),
  "winner" BYTEA NOT NULL,
  "number" BIGINT NOT NULL,
  "memory_changeset_hash" BYTEA NOT NULL,
  "storage_changeset_hash" BYTEA NOT NULL,
  "sealed" BOOL NOT NULL DEFAULT false,
  PRIMARY KEY ("hash")
);

CREATE UNIQUE INDEX "blocks_hash_index" ON "blocks" ("hash");
