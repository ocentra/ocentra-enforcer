// PASS fixture for MONEY-CRIT-ANNOTATED.1: the same payment-signing shape
// as the fail fixture, but explicitly registered via
// `#[money_critical(registered)]`. Must stay clean under the T1 gate.

#[money_critical(registered)]
fn sign_payment(payload_hash: [u8; 32], signing_key: &[u8]) -> [u8; 64] {
    authorize_payment(payload_hash, signing_key)
}

#[money_critical(registered)]
fn authorize_payment(_payload_hash: [u8; 32], _signing_key: &[u8]) -> [u8; 64] {
    [0u8; 64]
}
