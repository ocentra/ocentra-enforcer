// Representative-triple companion fixture (workpack "Acceptance And
// Proof" triples): the same balance-crediting shape as
// `classify/bad/credit_balance.rs`, registered via
// `#[money_critical(registered)]`. The T2 classifier still classifies this
// unit (annotation state does not change the score) — this fixture is
// exercised by the T1 annotated-gate tests (see
// `annotated/good/sign_payment_registered.rs` for the primary parity
// pair); kept here alongside its unannotated sibling for readability of
// the classify/annotated relationship the workpack's proof triples
// describe.

#[money_critical(registered)]
fn credit_balance(account_id: u64, amount_cents: u64, fee_cents: u64) -> u64 {
    let net_credit = amount_cents.saturating_sub(fee_cents);
    let ledger_balance = read_balance(account_id);
    ledger_balance.saturating_add(net_credit)
}

#[money_critical(registered)]
fn read_balance(_account_id: u64) -> u64 {
    0
}
