// T2-only fixture (workpack c04): trips `LIT-1.1` (Severity::Warning, a
// scored literal-scan finding) via `ENFORCER_LIT_1_1_MARKER` below, with no
// T1 hard-Validator finding present -- the hook must ALLOW-WITH-WARNING,
// never deny.
// ENFORCER_LIT_1_1_MARKER
fn parse_it(raw: &str) -> Result<i32, std::num::ParseIntError> {
    let value = raw.parse::<i32>()?;
    Ok(value)
}
