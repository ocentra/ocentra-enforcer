// FAIL fixture for MONEY-CRIT-CLASSIFY.1: a balance-crediting function.
// Multiple value-touching signal families fire (balance mutation +
// economic calculation), so this crosses CLASSIFY_THRESHOLD and MUST be
// classified money-critical.

fn credit_balance(account_id: u64, amount_cents: u64, fee_cents: u64) -> u64 {
    let net_credit = amount_cents.saturating_sub(fee_cents);
    let ledger_balance = read_balance(account_id);
    ledger_balance.saturating_add(net_credit)
}

fn read_balance(_account_id: u64) -> u64 {
    0
}
