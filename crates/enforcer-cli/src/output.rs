//! The ONE sanctioned stdout/stderr transport module in this crate.
//!
//! Every stdout/stderr write anywhere in `enforcer-cli` funnels through
//! here. It writes through explicit `std::io::Write` boundaries, matching
//! `enforcer-mcp::sink`; no other CLI module owns process output.
//!
//! Renders an `enforcer_domain::findings::Report` to stdout with a terse
//! `Fix:` hint per finding ([`crate::fix_hints::fix_hint`]). There is NO
//! flag anywhere that changes this rendering to hide a finding -- the
//! report rendered here is always the complete report the engine
//! produced.

use enforcer_domain::findings::{Finding, FindingLine, Report, ReportOutcome};
use enforcer_domain::install_types::HookDecision;
use enforcer_domain::severity::Severity;
use std::io::{self, Write};

use crate::fix_hints::fix_hint;

fn write_line(mut output: impl Write, message: &str) -> io::Result<()> {
    output.write_all(message.as_bytes())?;
    output.write_all(b"\n")
}

fn emit_stdout(message: &str) {
    let stdout = io::stdout();
    let _ = write_line(stdout.lock(), message);
}

fn emit_stderr(message: &str) {
    let stderr = io::stderr();
    let _ = write_line(stderr.lock(), message);
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn render_finding(finding: &Finding) -> String {
    let line = match finding.line {
        FindingLine::Known(line) => line.to_string(),
        FindingLine::Unspecified => "0".to_owned(),
    };
    format!(
        "  [{}] {}:{} {} -- {}\n    {}",
        severity_label(finding.severity),
        finding.file.as_str(),
        line,
        finding.rule_id.as_str(),
        finding.title,
        fix_hint(&finding.rule_id),
    )
}

/// Render a full [`Report`] to stdout: a one-line summary, then every
/// finding (violations, warnings, waived) with its `Fix:` hint.
pub fn print_report(report: &Report) {
    if report.ok == ReportOutcome::Clean {
        emit_stdout(&format!(
            "enforcer: no violations ({} finding(s) total).",
            report.findings.len()
        ));
    } else {
        emit_stdout(&format!(
            "enforcer: {} violation(s), {} warning(s), {} waived.",
            report.violations.len(),
            report.warnings.len(),
            report.waived.len()
        ));
    }
    for violation in &report.violations {
        emit_stdout(&render_finding(violation.finding()));
    }
    for warning in &report.warnings {
        emit_stdout(&render_finding(warning));
    }
    for waived in &report.waived {
        emit_stdout(&format!("  [waived] {}", render_finding(waived)));
    }
}

/// Render the complete native report as one JSON value for machine callers.
pub fn print_report_json(report: &Report) {
    emit_stdout(&serde_json::to_string(report).unwrap_or_else(|error| {
        format!("{{\"ok\":false,\"error\":\"cannot serialize report: {error}\"}}")
    }));
}

/// Render a successful artifact path through the sole CLI stdout boundary.
pub fn print_artifact_path(path: &std::path::Path) {
    emit_stdout(&format!("enforcer: artifact written: {}", path.display()));
}

/// Print a usage-error message to stderr (clap parse failures that this
/// crate itself detects post-parse, e.g. "no scope given").
pub fn print_usage_error(message: &str) {
    emit_stderr(&format!("enforcer: usage error: {message}"));
}

/// Print an internal-error message to stderr -- reserved for failures
/// that point at the enforcer itself (panic, I/O, decode bug), never at
/// the scanned project. Distinct call site from
/// [`print_usage_error`]/[`print_report`] so `grep`ing this module shows
/// the three message classes never collapse into one generic path.
pub fn print_internal_error(message: &str) {
    emit_stderr(&format!("enforcer: internal error: {message}"));
}

/// Print a config-load error to stderr.
pub fn print_config_error(message: &str) {
    emit_stderr(&format!("enforcer: config error: {message}"));
}

/// Emit the Claude Code `PreToolUse` permission-decision envelope. The hook
/// command uses this JSON as the authoritative allow/deny signal; a denial
/// also exits with the canonical violations code so hosts that ignore the
/// structured envelope still fail closed.
pub fn print_pretooluse_decision(decision: &HookDecision) {
    let (permission, reason) = match decision {
        HookDecision::Allow => ("allow", "enforcer clean".to_owned()),
        HookDecision::AllowWithWarning { reason } => ("allow", reason.as_str().to_owned()),
        HookDecision::Deny { reason } => ("deny", reason.as_str().to_owned()),
    };
    let envelope = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": permission,
            "permissionDecisionReason": reason,
        }
    });
    emit_stdout(&envelope.to_string());
}

/// Render a literal-scan [`enforcer_literal_scan::ScanReport`] to stdout.
/// Kept in this sink module (not `commands.rs`) so the whole crate has
/// exactly one place that ever calls `print!`/`println!`/`eprintln!`.
pub fn print_literal_scan_report(report: &enforcer_literal_scan::ScanReport) {
    if matches!(report.ok, enforcer_domain::findings::ReportOutcome::Clean) {
        emit_stdout(&format!(
            "enforcer advise literals: clean ({} literal(s) scanned).",
            report.summary.literals_found
        ));
        return;
    }
    emit_stdout(&format!(
        "enforcer advise literals: {} hard finding(s), {} literal risk(s).",
        report.hard_findings.len(),
        report.literal_risks.len()
    ));
    for finding in report
        .hard_findings
        .iter()
        .chain(report.literal_risks.iter())
    {
        emit_stdout(&format!("  {finding:?}"));
    }
}

/// Render an enforcer-memory CLI outcome through this crate's sole stdio
/// boundary. The memory crate deliberately returns text instead of touching
/// stdio itself; its transport semantics (including JSON mode) stay intact.
#[cfg(feature = "full")]
pub fn print_memory_cli_outcome(outcome: &enforcer_memory::cli::CliOutcome) {
    if let Some(stdout) = &outcome.stdout {
        emit_stdout(stdout.as_str());
    }
    if let Some(stderr) = &outcome.stderr {
        emit_stderr(stderr.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::{render_finding, write_line};
    use enforcer_domain::findings::{Finding, FindingDetail, FindingLine, FindingTitle};
    use enforcer_domain::severity::Severity;
    use enforcer_domain::telemetry_types::SourceLine;
    use std::num::NonZeroU32;

    #[test]
    fn rendered_finding_always_carries_a_fix_hint() -> Result<(), Box<dyn std::error::Error>> {
        let finding = Finding {
            rule_id: "RR-6.1".parse()?,
            severity: Severity::Error,
            title: FindingTitle::new("unwrap() in first-party code".to_owned())?,
            detail: FindingDetail::new("d".to_owned())?,
            file: "src/lib.rs".parse()?,
            line: FindingLine::known(SourceLine::try_new(
                NonZeroU32::new(3).ok_or("test line must be positive")?,
            )),
            snippet: None,
        };
        let rendered = render_finding(&finding);
        assert_eq!(
            rendered,
            "  [error] src/lib.rs:3 RR-6.1 -- unwrap() in first-party code\n    Fix: replace unwrap()/expect()/panic! with a typed Result and `?`."
        );
        Ok(())
    }

    #[test]
    fn writer_transport_adds_one_line_terminator() -> Result<(), Box<dyn std::error::Error>> {
        let mut output = Vec::new();
        write_line(&mut output, "enforcer: output boundary")?;
        assert_eq!(output, b"enforcer: output boundary\n");
        Ok(())
    }
}
