//! The ONE sanctioned print-sink module in this crate.
//!
//! Every stdout/stderr write anywhere in `enforcer-cli` funnels through
//! here. Every other module in this crate obeys the workspace
//! `[lints]` deny wall (`clippy::print_stdout`/`clippy::print_stderr`
//! denied); this module carries the scoped, documented
//! `#![allow(clippy::print_stdout, clippy::print_stderr)]` that makes it
//! the sole exception, matching `enforcer-mcp::sink`'s identical posture
//! on the MCP side.
//!
//! Renders an `enforcer_domain::findings::Report` to stdout with a terse
//! `Fix:` hint per finding ([`crate::fix_hints::fix_hint`]). There is NO
//! flag anywhere that changes this rendering to hide a finding -- the
//! report rendered here is always the complete report the engine
//! produced.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use enforcer_domain::findings::{Finding, Report};
use enforcer_domain::severity::Severity;

use crate::fix_hints::fix_hint;

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn render_finding(finding: &Finding) -> String {
    format!(
        "  [{}] {}:{} {} -- {}\n    {}",
        severity_label(finding.severity),
        finding.file.as_str(),
        finding.line,
        finding.rule_id.as_str(),
        finding.title,
        fix_hint(&finding.rule_id),
    )
}

/// Render a full [`Report`] to stdout: a one-line summary, then every
/// finding (violations, warnings, waived) with its `Fix:` hint.
pub fn print_report(report: &Report) {
    if report.ok {
        println!(
            "enforcer: no violations ({} finding(s) total).",
            report.findings.len()
        );
    } else {
        println!(
            "enforcer: {} violation(s), {} warning(s), {} waived.",
            report.violations.len(),
            report.warnings.len(),
            report.waived.len()
        );
    }
    for violation in &report.violations {
        println!("{}", render_finding(violation.finding()));
    }
    for warning in &report.warnings {
        println!("{}", render_finding(warning));
    }
    for waived in &report.waived {
        println!("  [waived] {}", render_finding(waived));
    }
}

/// Print a usage-error message to stderr (clap parse failures that this
/// crate itself detects post-parse, e.g. "no scope given").
pub fn print_usage_error(message: &str) {
    eprintln!("enforcer: usage error: {message}");
}

/// Print an internal-error message to stderr -- reserved for failures
/// that point at the enforcer itself (panic, I/O, decode bug), never at
/// the scanned project. Distinct call site from
/// [`print_usage_error`]/[`print_report`] so `grep`ing this module shows
/// the three message classes never collapse into one generic path.
pub fn print_internal_error(message: &str) {
    eprintln!("enforcer: internal error: {message}");
}

/// Print a config-load error to stderr.
pub fn print_config_error(message: &str) {
    eprintln!("enforcer: config error: {message}");
}

/// Render a literal-scan [`enforcer_literal_scan::ScanReport`] to stdout.
/// Kept in this sink module (not `commands.rs`) so the whole crate has
/// exactly one place that ever calls `print!`/`println!`/`eprintln!`.
pub fn print_literal_scan_report(report: &enforcer_literal_scan::ScanReport) {
    if report.ok {
        println!(
            "enforcer advise literals: clean ({} literal(s) scanned).",
            report.summary.literals_found
        );
        return;
    }
    println!(
        "enforcer advise literals: {} hard finding(s), {} literal risk(s).",
        report.hard_findings.len(),
        report.literal_risks.len()
    );
    for finding in report
        .hard_findings
        .iter()
        .chain(report.literal_risks.iter())
    {
        println!("  {finding:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::render_finding;
    use enforcer_domain::findings::Finding;
    use enforcer_domain::severity::Severity;

    #[test]
    fn rendered_finding_always_carries_a_fix_hint() -> Result<(), Box<dyn std::error::Error>> {
        let finding = Finding {
            rule_id: "RR-6.1".parse()?,
            severity: Severity::Error,
            title: "unwrap() in first-party code".to_owned(),
            detail: "d".to_owned(),
            file: "src/lib.rs".parse()?,
            line: 3,
            snippet: None,
        };
        let rendered = render_finding(&finding);
        assert!(rendered.contains("Fix:"));
        assert!(rendered.contains("RR-6.1"));
        Ok(())
    }
}
