// Seeded violation: an inline lint-disable escape hatch on an
// enforcer-governed lint. This is exactly the shape the no-bypass
// meta-check exists to ban.
#[allow(clippy::unwrap_used)]
fn risky_balance_update(raw: &str) -> i64 {
    raw.parse::<i64>().unwrap()
}
