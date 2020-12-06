-- Modified from: https://gist.github.com/001101/a40713947923f7c4bd2921fb40c9e11c

CREATE TABLE "addresses"(
	"id" serial PRIMARY KEY,
	"bytes" BYTEA NOT NULL
);

CREATE TABLE "networks"(
	"id" INTEGER NOT NULL PRIMARY KEY,
	"name" VARCHAR NOT NULL
);

INSERT INTO "networks" VALUES
    (0, 'Bitcoin'),
    (1, 'Ethereum'),
    (2, 'Ellipticoin');

CREATE TABLE "tokens"(
	"id" serial PRIMARY KEY,
	"network_id" INTEGER NOT NULL REFERENCES "networks"("id") ON DELETE RESTRICT,
	"id_bytes" BYTEA NOT NULL,
	"name" VARCHAR NOT NULL
);

INSERT INTO "tokens" ("network_id", "id_bytes", "name") VALUES
    (0, decode('c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2', 'hex'), 'Ether'),
    (0, decode('eb4c2781e4eba804ce9a9803c67d0893436bb27d', 'hex'),'Bitcoin'),
    (0, decode('6b175474e89094c44da98b954eedeac495271d0f', 'hex'), 'USD'),
    (1, (sha256('ELC')), 'Ellipticoin');



CREATE TABLE "ledger_entries"(
	"id" serial PRIMARY KEY,
	"transaction_id" INTEGER NOT NULL REFERENCES "transactions"("id") ON DELETE RESTRICT,
	"token_id" INTEGER NOT NULL REFERENCES "tokens"("id") ON DELETE RESTRICT,
	"amount" BIGINT NOT NULL CHECK ("amount" > 0),
	"credit_id" INTEGER NOT NULL REFERENCES "addresses"("id") ON DELETE RESTRICT,
	"debit_id" INTEGER NOT NULL REFERENCES "addresses"("id") ON DELETE RESTRICT
);
CREATE INDEX ON "ledger_entries"("credit_id");
CREATE INDEX ON "ledger_entries"("debit_id");

CREATE VIEW "ledger_entries_by_account"(
	"account_id",
	"entry_id",
	"token_id",
	"amount"
) AS
	SELECT
		"ledger_entries"."credit_id",
		"ledger_entries"."id",
		"ledger_entries"."token_id",
		"ledger_entries"."amount"
	FROM
		"ledger_entries"
	UNION ALL
	SELECT
		"ledger_entries"."debit_id",
		"ledger_entries"."id",
		"ledger_entries"."token_id",
		(0 - "ledger_entries"."amount")
	FROM
		"ledger_entries";

CREATE MATERIALIZED VIEW "balances"(
	"id",
	"token_id",
	"balance"
) AS
	SELECT
		"addresses"."id",
        "ledger_entries_by_account"."token_id",
		CAST(COALESCE(sum("ledger_entries_by_account"."amount"), 0) AS BIGINT)
	FROM
        "addresses"
		LEFT OUTER JOIN "ledger_entries_by_account"
		ON "addresses"."id" = "ledger_entries_by_account"."account_id"
	GROUP BY "addresses"."id", "ledger_entries_by_account", "token_id";

CREATE UNIQUE INDEX ON "balances"("id");

CREATE FUNCTION "update_balances"() RETURNS TRIGGER AS $$
BEGIN
	REFRESH MATERIALIZED VIEW "balances";
	RETURN NULL;
END
$$ LANGUAGE plpgsql;

CREATE TRIGGER "trigger_update_balance_ledger_entries"
AFTER INSERT 
OR UPDATE OF "amount", "credit_id", "debit_id" 
OR DELETE OR TRUNCATE
ON "ledger_entries"
FOR EACH STATEMENT
EXECUTE PROCEDURE "update_balances"();

CREATE TRIGGER "trigger_update_balance_addresses"
AFTER INSERT 
OR UPDATE OF "id"
OR DELETE OR TRUNCATE
ON "addresses"
FOR EACH STATEMENT
EXECUTE PROCEDURE "update_balances"();
