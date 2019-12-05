CREATE TABLE "blocks" (
  "hash" BYTEA NOT NULL,
  "parent_hash" BYTEA CONSTRAINT "blocks_parent_hash_fkey" REFERENCES "blocks" ("hash"),
  "number" BIGINT NOT NULL,
  "winner" BYTEA NOT NULL,
  "memory_changeset_hash" BYTEA NOT NULL,
  "storage_changeset_hash" BYTEA NOT NULL,
  "proof_of_work_value" BIGINT NOT NULL,
  PRIMARY KEY ("hash")
);

CREATE UNIQUE INDEX "blocks_hash_index" ON "blocks" ("hash");
