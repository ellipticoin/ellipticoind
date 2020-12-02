DROP INDEX ledger_entries_credit;
DROP INDEX ledger_entries_debit;
-- DROP VIEW account_ledgers;
-- DROP MATERIALIZED VIEW account_balances;
-- DROP INDEX account_balances_id;
-- DROP FUNCTION update_balances;
-- DROP TRIGGER trigger_fix_balance_ledger_entries;
-- DROP TRIGGER trigger_fix_balance_addresses;
ALTER TABLE "transactions"
DROP COLUMN "entry_id";
DROP TABLE "ledger_entries";
