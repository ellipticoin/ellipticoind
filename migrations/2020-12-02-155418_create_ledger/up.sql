CREATE TABLE addresses(
	id serial PRIMARY KEY,
	name VARCHAR(256) NOT NULL
);

CREATE TABLE ledger_entries(
	id serial PRIMARY KEY,
	description VARCHAR(1024) NOT NULL,
	amount NUMERIC(20, 2) NOT NULL CHECK (amount > 0.0),
	-- Every entry is a credit to one account...
	credit INTEGER NOT NULL REFERENCES addresses(id) ON DELETE RESTRICT,
	-- And a debit to another
	debit INTEGER NOT NULL REFERENCES addresses(id) ON DELETE RESTRICT
	-- In a paper ledger, the entry would be recorded once in each account, but
	-- that would be silly in a relational database

	-- Deletes are restricted because deleting an account with outstanding
	-- ledger_entries just doesn't make sense.  If the account's balance is nonzero,
	-- it would make assets or liabilities vanish, and even if it is zero,
	-- the account is still responsible for the nonzero balances of other
	-- addresses, so deleting it would lose important information.
);
ALTER TABLE "transactions"
ADD COLUMN "entry_id" INTEGER REFERENCES "ledger_entries";
CREATE INDEX ON ledger_entries(credit);
CREATE INDEX ON ledger_entries(debit);

CREATE VIEW account_ledgers(
	account_id,
	entry_id,
	amount
) AS
	SELECT
		ledger_entries.credit,
		ledger_entries.id,
		ledger_entries.amount
	FROM
		ledger_entries
	UNION ALL
	SELECT
		ledger_entries.debit,
		ledger_entries.id,
		(0.0 - ledger_entries.amount)
	FROM
		ledger_entries;


CREATE MATERIALIZED VIEW account_balances(
	-- Materialized so financial reports run fast.
	-- Modification of addresses and ledger_entries will require a
	-- REFRESH MATERIALIZED VIEW, which we can trigger
	-- automatically.
	id, -- INTEGER REFERENCES addresses(id) NOT NULL UNIQUE
	balance -- NUMERIC NOT NULL
) AS
	SELECT
		addresses.id,
		COALESCE(sum(account_ledgers.amount), 0.0)
	FROM
		addresses
		LEFT OUTER JOIN account_ledgers
		ON addresses.id = account_ledgers.account_id
	GROUP BY addresses.id;

CREATE UNIQUE INDEX ON account_balances(id);

CREATE FUNCTION update_balances() RETURNS TRIGGER AS $$
BEGIN
	REFRESH MATERIALIZED VIEW account_balances;
	RETURN NULL;
END
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_fix_balance_ledger_entries
AFTER INSERT 
OR UPDATE OF amount, credit, debit 
OR DELETE OR TRUNCATE
ON ledger_entries
FOR EACH STATEMENT
EXECUTE PROCEDURE update_balances();

CREATE TRIGGER trigger_fix_balance_addresses
AFTER INSERT 
OR UPDATE OF id 
OR DELETE OR TRUNCATE
ON addresses
FOR EACH STATEMENT
EXECUTE PROCEDURE update_balances();

