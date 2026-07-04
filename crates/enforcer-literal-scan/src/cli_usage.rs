#[allow(clippy::print_stdout)]
pub(crate) fn print_usage() {
    println!(
        r#"enforcer-literal-scan

Usage:
  enforcer-literal-scan scan --root <repo> [options]
  enforcer-literal-scan scan --root <repo> --files <path...> [options]

Options:
  --json                  Print pretty JSON report.
  --jsonl                 Print one JSON object per finding.
  --human                 Print human-readable report.
  --min-score <0-100>     Minimum soft-risk score to include. Default: 40.
  --include-low           Include low-risk findings below min-score.
  --fail-above <0-100>    Turn risks at or above score into blocking hard findings.
  --languages <list>      Comma list of language IDs to scan.
  --include-ignored       Include files normally ignored by defaults/gitignore.
  --include-unknown-code  Use fallback lexer for unknown textual files.
  --no-respect-gitignore  Ignore .gitignore/.ignore rules.
  --max-file-bytes <n>    Skip files larger than this size. Default: 2097152.
"#
    );
}
