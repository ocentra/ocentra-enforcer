//! d21 change discipline — `OWNERSET-1.1`: owner-set marker preservation
//! across an external doc rewrite.
//!
//! Mechanizes lesson L39
//! ([`docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md`]):
//! TWICE an external doc-hardening branch silently dropped a protected
//! `(owner-set` requirement line while restructuring the surrounding doc.
//! The fix is not "diff carefully" — it is a mechanical T1 rule: every
//! occurrence of the `(owner-set` marker present in a BASE version of a
//! file MUST survive into the CHANGED (HEAD) version, matched by the
//! marker LINE's normalized text (not byte-equality of the whole file, so
//! reordering/moving a marker line, or editing unrelated lines around it,
//! is not itself a violation). A marker line present in BASE but absent
//! from HEAD is a violation naming the file and the lost marker text. New
//! marker lines may be freely added in HEAD.
//!
//! This is a base/head (diff-aware) rule, unlike this crate's other
//! `rules::*` siblings ([`crate::rules::deferred_work`], [`crate::rules::
//! fsm`], [`crate::rules::size_shape`]), which all inspect one file's
//! source text in isolation. No diff-aware `Validator` seam exists yet in
//! `enforcer-validator`, so the core logic is exposed as the pure function
//! [`check_ownerset`] (base, head, path -> findings) per the workpack's
//! fallback API. [`OwnersetValidator`] adapts that pure function onto the
//! existing single-source [`Validator`] trait so this rule can still ride
//! the standard fixture/parity harness and the d01 5-way parity oracle:
//! its fixture files carry BOTH versions in one file, split on the
//! [`HEAD_DELIMITER`] line, and `validate` splits `input.source` on that
//! delimiter before delegating to [`check_ownerset`]. The eventual c04
//! pre-push hook / CI wiring that has real base/head git blobs to compare
//! should call [`check_ownerset`] directly instead of routing through the
//! single-file adapter — that wiring is noted as follow-up, not built
//! here.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The protected marker text this rule watches for. Any line containing
/// this substring is a "marker line" whose presence in BASE must survive
/// into HEAD.
pub const OWNERSET_MARKER: &str = "(owner-set";

/// Fixture-file-only delimiter line separating a fixture's BASE section
/// (before) from its HEAD section (after), so one fixture file can carry
/// both versions for [`OwnersetValidator`]'s single-source adapter. Not
/// part of the real base/head content this rule protects — a production
/// caller with real git blobs calls [`check_ownerset`] directly and never
/// sees this delimiter.
pub const HEAD_DELIMITER: &str = "===HEAD===";

/// Normalize a marker line's identity for comparison: trim surrounding
/// whitespace. Matching is by the marker LINE's normalized text, not
/// whole-file byte-equality, so a marker line that only moved position (or
/// was reflowed with different leading/trailing whitespace) still counts
/// as present.
fn normalize_marker_line(line: &str) -> String {
    line.trim().to_owned()
}

/// Every line in `source` that contains the protected [`OWNERSET_MARKER`]
/// substring, normalized for identity comparison. Order is preserved but
/// irrelevant to the check (a moved/reordered marker still counts as
/// present).
fn marker_lines(source: &str) -> Vec<String> {
    source
        .lines()
        .filter(|line| line.contains(OWNERSET_MARKER))
        .map(normalize_marker_line)
        .collect()
}

/// Core rule logic (T1): every `(owner-set` marker line present in `base`
/// must have a normalized match somewhere in `head`. Returns one
/// [`Finding`] per dropped marker line (in BASE order), naming `path` and
/// quoting the lost marker text. An empty result means every BASE marker
/// survived (including the case of zero markers in BASE — nothing to
/// protect is not a violation). New marker lines added in `head` with no
/// counterpart in `base` are never flagged.
pub fn check_ownerset(base: &str, head: &str, path: &RelPath, rule_id: &RuleId) -> Vec<Finding> {
    let head_markers: std::collections::BTreeSet<String> = marker_lines(head).into_iter().collect();
    marker_lines(base)
        .into_iter()
        .filter(|base_line| !head_markers.contains(base_line))
        .map(|dropped| Finding {
            rule_id: rule_id.clone(),
            severity: Severity::Error,
            title: "change-discipline: owner-set marker dropped".to_owned(),
            detail: format!(
                "`{}` lost a protected owner-set marker present in the base version: `{dropped}`. \
                 Owner-set requirements are protected invariants (L39) — restore the marker via \
                 union resolution, or waive it explicitly by name.",
                path.as_str()
            ),
            file: path.clone(),
            line: 1,
            snippet: Some(dropped),
        })
        .collect()
}

/// `OWNERSET-1.1` — owner-set marker preservation, adapted onto the
/// single-source [`Validator`] trait for harness/parity-oracle
/// compatibility. Splits `input.source` on [`HEAD_DELIMITER`]: everything
/// before is BASE, everything after is HEAD. A fixture/source with no
/// delimiter is treated as HEAD-only (empty BASE — nothing to protect, so
/// it is always clean under this rule).
pub struct OwnersetValidator {
    rule_id: RuleId,
}

impl OwnersetValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "OWNERSET-1.1".parse()?,
        })
    }
}

impl Validator for OwnersetValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let (base, head) = match input.source.split_once(HEAD_DELIMITER) {
            Some((base, head)) => (base, head),
            None => ("", input.source),
        };
        check_ownerset(base, head, input.file, &self.rule_id)
    }
}

/// Build every `change_discipline` family validator this crate owns (d21).
pub fn validators(
) -> Result<Vec<Box<dyn Validator>>, enforcer_domain::boundary::decode_error::DecodeError> {
    Ok(vec![Box::new(OwnersetValidator::new()?)])
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::*;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn path(p: &str) -> Result<RelPath, enforcer_domain::boundary::decode_error::DecodeError> {
        p.parse()
    }

    fn rule_id() -> Result<RuleId, enforcer_domain::boundary::decode_error::DecodeError> {
        "OWNERSET-1.1".parse()
    }

    #[test]
    fn one_validator_registered() -> Result<(), enforcer_domain::boundary::decode_error::DecodeError>
    {
        let vs = validators()?;
        assert_eq!(vs.len(), 1);
        assert_eq!(vs[0].rule_id().as_str(), "OWNERSET-1.1");
        Ok(())
    }

    #[test]
    fn dropped_marker_is_flagged_naming_file_and_lost_text(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let base = "- [ ] Seam A (owner-set, RESTORED): do the thing\n\
                     - [ ] Seam B (owner-set): do the other thing\n";
        let head = "- [ ] Seam A (owner-set, RESTORED): do the thing\n";
        let findings = check_ownerset(base, head, &path("docs/x06.md")?, &rule_id()?);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "OWNERSET-1.1");
        assert!(findings[0].file.as_str().contains("x06.md"));
        assert!(findings[0].detail.contains("Seam B (owner-set)"));
        Ok(())
    }

    #[test]
    fn identical_markers_are_clean(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let base = "- [ ] Seam A (owner-set): keep this\n";
        let head = "- [ ] Seam A (owner-set): keep this\n";
        assert!(check_ownerset(base, head, &path("f.md")?, &rule_id()?).is_empty());
        Ok(())
    }

    #[test]
    fn moved_or_reordered_marker_is_clean(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let base = "intro\n- [ ] Seam A (owner-set): keep this\noutro\n";
        let head = "outro\nintro\n- [ ] Seam A (owner-set): keep this\n";
        assert!(check_ownerset(base, head, &path("f.md")?, &rule_id()?).is_empty());
        Ok(())
    }

    #[test]
    fn new_marker_added_is_clean(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let base = "- [ ] Seam A (owner-set): keep this\n";
        let head = "- [ ] Seam A (owner-set): keep this\n\
                     - [ ] Seam B (owner-set): brand new requirement\n";
        assert!(check_ownerset(base, head, &path("f.md")?, &rule_id()?).is_empty());
        Ok(())
    }

    #[test]
    fn unrelated_edit_near_marker_is_clean(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let base = "context line one\n- [ ] Seam A (owner-set): keep this\ncontext line two\n";
        let head =
            "context line one, reworded\n- [ ] Seam A (owner-set): keep this\ncontext line two, also reworded\n";
        assert!(check_ownerset(base, head, &path("f.md")?, &rule_id()?).is_empty());
        Ok(())
    }

    #[test]
    fn no_markers_in_base_is_clean(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        assert!(check_ownerset(
            "plain doc\n",
            "plain doc, hardened\n",
            &path("f.md")?,
            &rule_id()?
        )
        .is_empty());
        Ok(())
    }

    #[test]
    fn ownerset_validator_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = OwnersetValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/change_discipline/ownerset_dropped/bad.md",
            "tests/fixtures/change_discipline/ownerset_reordered/good.md",
        )?;
        Ok(())
    }

    #[test]
    fn ownerset_validator_new_marker_added_pass_fixture_stays_clean(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = OwnersetValidator::new()?;
        let source = std::fs::read_to_string(
            manifest_dir().join("tests/fixtures/change_discipline/ownerset_new_added/good.md"),
        )?;
        let findings = validator.validate(ValidationInput {
            file: &path("tests/fixtures/change_discipline/ownerset_new_added/good.md")?,
            source: &source,
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn ownerset_validator_unrelated_edit_pass_fixture_stays_clean(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = OwnersetValidator::new()?;
        let source = std::fs::read_to_string(
            manifest_dir().join("tests/fixtures/change_discipline/ownerset_unrelated_edit/good.md"),
        )?;
        let findings = validator.validate(ValidationInput {
            file: &path("tests/fixtures/change_discipline/ownerset_unrelated_edit/good.md")?,
            source: &source,
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    /// Real-world regression fixture modeled on the actual x06 incident
    /// (L39): a hardening rewrite of a checklist drops the seam line.
    /// Proves the fail leg via fixture parity against a same-shaped pass
    /// fixture where the seam survives the rewrite.
    #[test]
    fn ownerset_validator_x06_regression_fixture_parity() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = OwnersetValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/change_discipline/ownerset_x06_regression/bad.md",
            "tests/fixtures/change_discipline/ownerset_x06_regression/good.md",
        )?;
        Ok(())
    }

    /// Anti-vacuous guard: a validator that never fires must be caught by
    /// the harness, not silently accepted as "passing". Mirrors this
    /// crate's other `rules::*` modules' harness-failure expectations.
    #[test]
    fn harness_fails_closed_when_validator_never_fires_on_a_real_drop(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        struct NeverFiresValidator {
            rule_id: RuleId,
        }
        impl Validator for NeverFiresValidator {
            fn rule_id(&self) -> &RuleId {
                &self.rule_id
            }
            fn validate(&self, _input: ValidationInput<'_>) -> Vec<Finding> {
                Vec::new()
            }
        }
        let never_fires = NeverFiresValidator {
            rule_id: rule_id()?,
        };
        let result = run_fixture_parity(
            &never_fires,
            &manifest_dir(),
            "tests/fixtures/change_discipline/ownerset_dropped/bad.md",
            "tests/fixtures/change_discipline/ownerset_reordered/good.md",
        );
        assert!(
            result.is_err(),
            "a validator that never fires must fail the fixture-parity harness closed"
        );
        Ok(())
    }

    /// Pins the real invariant this rule protects: the LIVE x06 workpack
    /// doc must still carry at least two `(owner-set` markers today. If
    /// this test ever fails, the live doc itself regressed the exact
    /// requirement L39 and this rule exist to prevent.
    #[test]
    fn live_x06_workpack_still_carries_at_least_two_ownerset_markers(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo_root = manifest_dir()
            .parent()
            .and_then(|p| p.parent())
            .map(std::path::Path::to_path_buf)
            .ok_or("could not resolve repo root from CARGO_MANIFEST_DIR")?;
        let x06 = repo_root
            .join("docs/plans/enforcer-selfhost-plan/workpacks/x06-harness-memory-graph.md");
        let source = std::fs::read_to_string(&x06)?;
        let count = marker_lines(&source).len();
        assert!(
            count >= 2,
            "expected the live x06 workpack to carry >= 2 (owner-set markers, found {count}"
        );
        Ok(())
    }
}
