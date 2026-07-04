//! Skeleton-scoped completeness test (this workpack, arc-19): the
//! registry must contain no duplicate rule id and must contain the
//! no-bypass meta-check's row (`H00-1.1`).
//!
//! This is deliberately NOT a full `rules/rules.json` family count-parity
//! test like `enforcer-lang-security`'s: the Track H money-critical
//! (h01-h08) and security-testing (h11) feature packs land their own
//! rule ids into `rules/rules.json` and their own registry rows as they
//! build on this skeleton (see the workpack's Parallel Ownership Notes).
//! Locking this test to today's full rules.json count would make it fail
//! the moment the first feature pack lands its own registry row/fixture
//! — this test proves only what this workpack itself owns.

use std::collections::BTreeSet;

use enforcer_security::rules::registry::build_all;

#[test]
fn registry_has_no_duplicates_and_contains_no_bypass_row() -> Result<(), Box<dyn std::error::Error>>
{
    let rows = build_all()?;

    let mut seen = BTreeSet::new();
    for row in &rows {
        assert!(
            seen.insert(row.rule_id.to_owned()),
            "duplicate registry row for rule id `{}`",
            row.rule_id
        );
    }

    assert!(
        seen.contains("H00-1.1"),
        "no-bypass meta-check row (`H00-1.1`) must be registered"
    );

    Ok(())
}
