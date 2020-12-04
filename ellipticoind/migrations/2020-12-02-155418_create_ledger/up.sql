// Modified from: https://gist.github.com/001101/a40713947923f7c4bd2921fb40c9e11c

CREATE TABLE "addresses"(
	id serial PRIMARY KEY,
	bytes BYTEA NOT NULL
);

CREATE TABLE "ledger_entries"(
	"id" serial PRIMARY KEY,
	"transaction_id" INTEGER NOT NULL REFERENCES "transactions"(id) ON DELETE RESTRICT,
	"amount" BIGINT NOT NULL CHECK (amount > 0),
	"credit_id" INTEGER NOT NULL REFERENCES "addresses"(id) ON DELETE RESTRICT,
	"debit_id" INTEGER NOT NULL REFERENCES "addresses"(id) ON DELETE RESTRICT
);
CREATE INDEX ON "ledger_entries"(credit_id);
CREATE INDEX ON "ledger_entries"(debit_id);

CREATE VIEW "ledger_entries_by_account"(
	"account_id",
	"entry_id",
	"amount"
) AS
	SELECT
		"ledger_entries"."credit_id",
		"ledger_entries"."id",
		"ledger_entries"."amount"
	FROM
		"ledger_entries"
	UNION ALL
	SELECT
		"ledger_entries"."debit_id",
		"ledger_entries"."id",
		(0 - "ledger_entries"."amount")
	FROM
		"ledger_entries";

CREATE MATERIALIZED VIEW "balances"(
	"id",
	"balance"
) AS
	SELECT
		"addresses"."id",
		CAST(COALESCE(sum("ledger_entries_by_account"."amount"), 0) AS BIGINT)
	FROM
		"addresses"
		LEFT OUTER JOIN "ledger_entries_by_account"
		ON "addresses"."id" = "ledger_entries_by_account"."account_id"
	GROUP BY "addresses"."id";

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
