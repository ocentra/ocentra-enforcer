// FAIL fixture for MONEY-CRIT-ANNOTATED.1: a payment-signing function that
// is classified money-critical (signing family) but carries NEITHER
// `#[money_critical(registered)]` NOR `#[money_critical(exempt)]`. MUST be
// flagged by the T1 annotation gate.

fn sign_payment(payload_hash: [u8; 32], signing_key: &[u8]) -> [u8; 64] {
    authorize_payment(payload_hash, signing_key)
}

fn authorize_payment(_payload_hash: [u8; 32], _signing_key: &[u8]) -> [u8; 64] {
    [0u8; 64]
}
