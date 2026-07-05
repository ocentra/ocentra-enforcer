//! The pluggable fix-generator contract dispatched by [`super::run_fix_loop`].
//!
//! A [`FixGenerator`] is handed the current on-disk snapshot root plus the
//! [`Finding`](enforcer_domain::findings::Finding)s the last re-scan
//! produced, and attempts ONE bounded editing move. It never decides
//! whether its own edit was an improvement — that judgment belongs solely
//! to the loop's re-scan-and-compare gate (`super::run_fix_loop`), which is
//! what keeps the loop from trusting a fix generator's self-report.

use std::path::Path;

use enforcer_domain::findings::Finding;

use crate::error::Result;

/// One attempted editing move against the working tree at `root`.
///
/// Implementors mutate files under `root` directly (the loop has already
/// snapshotted the tree before calling this, and will restore it if the
/// attempt does not improve the finding count). Returning `Ok(false)` means
/// "I had nothing to try this iteration" — the loop treats a no-op attempt
/// as ineligible to keep (same rule as a non-improving edit) and, more
/// importantly, as a signal to stop iterating rather than spin the cap.
pub trait FixGenerator {
    /// Attempt to address (a subset of) `findings` by editing files under
    /// `root`. Returns `Ok(true)` if any edit was made, `Ok(false)` if the
    /// generator declined to act this iteration (e.g. no findings it knows
    /// how to address), or `Err` on an operational failure (I/O, etc).
    fn attempt_fix(&self, root: &Path, findings: &[Finding]) -> Result<bool>;

    /// Human-readable name recorded on the loop's typed events, for
    /// observability (which generator produced/declined a given attempt).
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use std::fs;

    use enforcer_domain::severity::Severity;

    use super::*;

    fn sample_finding() -> Result<Finding> {
        Ok(Finding {
            rule_id: "RR-1.1".parse()?,
            severity: Severity::Error,
            title: "t".to_owned(),
            detail: "d".to_owned(),
            file: "a.txt".parse()?,
            line: 1,
            snippet: None,
        })
    }

    /// A generator that removes the literal marker `BAD` from `a.txt`.
    struct MarkerRemover;

    impl FixGenerator for MarkerRemover {
        fn attempt_fix(&self, root: &Path, findings: &[Finding]) -> Result<bool> {
            if findings.is_empty() {
                return Ok(false);
            }
            let path = root.join("a.txt");
            let content = fs::read_to_string(&path)?;
            if !content.contains("BAD") {
                return Ok(false);
            }
            fs::write(&path, content.replace("BAD", "GOOD"))?;
            Ok(true)
        }

        fn name(&self) -> &str {
            "marker-remover"
        }
    }

    #[test]
    fn generator_reports_no_op_when_findings_empty() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let generator = MarkerRemover;
        assert!(!generator.attempt_fix(dir.path(), &[])?);
        Ok(())
    }

    #[test]
    fn generator_edits_and_reports_true_on_a_match() -> Result<()> {
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("a.txt"), "BAD stuff")?;
        let generator = MarkerRemover;
        let changed = generator.attempt_fix(dir.path(), &[sample_finding()?])?;
        assert!(changed);
        assert_eq!(fs::read_to_string(dir.path().join("a.txt"))?, "GOOD stuff");
        Ok(())
    }
}
