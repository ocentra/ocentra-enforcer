// Clean equivalent: no inline suppression directive of any kind. The
// only legitimate way to exempt a rule is a declarative, committed,
// gated waiver in enforcer-config (owner + reason + ruleId) -- never an
// inline comment or attribute.
fn safe_balance_update(raw: &str) -> Result<i64, std::num::ParseIntError> {
    raw.parse::<i64>()
}
