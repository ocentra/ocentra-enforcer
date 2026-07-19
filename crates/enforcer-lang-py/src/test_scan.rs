//! `python/test-scan` validator: PY-2.1 and PY-6.2 (2 rules). Distinct from
//! [`crate::source_scan`] because both rules are specifically about TEST
//! file shape (skip/focus markers, weak assertions) rather than general
//! source bans, matching the `rules/rules.json` `validator` partition.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::BuiltInPythonRule;
use enforcer_validator::validator::Validator;

use crate::boundary::line_marker::{Guard, LineMarkerValidator, WeakAssertionValidator};

/// Build every `python/test-scan`-keyed validator this crate registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(LineMarkerValidator::new(
            BuiltInPythonRule::Py2Rule1.id(),
            "Skipped/focused Python tests are forbidden",
            Guard::NotInCommentOrString,
            &["pytest.mark.skip", "pytest.skip(", "unittest.skip"],
        )),
        Box::new(WeakAssertionValidator::new(
            BuiltInPythonRule::Py6Rule2.id(),
            "Weak Python assertions are forbidden",
            &["user", "result", "value"],
        )),
    ])
}
