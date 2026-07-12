//! `CYBER-CMD-INJECT.1` (T1) — command-injection sink detector.
//!
//! Harvest note: the workpack's harvest pointer for this rule,
//! `vendor/anthropic-cybersecurity-skills/skills/performing-directory-traversal-testing`,
//! is a path-traversal/LFI penetration-testing playbook (curl payloads,
//! `ffuf`/`dotdotpwn` fuzzing, `/etc/passwd`/`win.ini` exfiltration
//! recipes) — it has no command-injection detection predicate to port,
//! and there is no dedicated command-injection vendor skill anywhere in
//! `vendor/anthropic-cybersecurity-skills/`. Per the h11 workpack
//! fallback, this validator instead implements the well-known,
//! deterministic Semgrep-style command-injection ruleset directly: flag a
//! command/code-execution sink only when its argument is clearly
//! non-literal (string concatenation `+`, an f-string prefix, `${...}` or
//! `#{...}` interpolation, or a bare variable with no string-literal quote
//! character at all) — never a fully static string literal. The one
//! documented exception is `subprocess.*(..., shell=True)`: ANY
//! `shell=True` is flagged regardless of whether the command argument
//! itself is literal, because `shell=True` alone is the well-known
//! unsafe-by-default Python footgun (a literal command string can still
//! carry shell metacharacters injected via string formatting done
//! elsewhere, and enabling the shell is itself the risky choice).
//!
//! Sinks covered (source scanned line by line, matching the `waf_sqli`
//! template's per-line regex-table approach):
//! - Python `subprocess.*(..., shell=True)` — always flagged.
//! - Python `os.system(...)` — flagged only for a non-literal argument.
//! - Node `child_process.exec(...)` / `execSync(...)` — flagged only for a
//!   non-literal argument (backtick template `${...}`, concatenation, ...).
//! - `eval(...)` / `exec(...)` of a non-literal expression (generic
//!   Python/Ruby call form; excludes qualified `foo.exec(...)` method
//!   calls so it does not double-report the Java sink below).
//! - Ruby backtick command substitution containing `#{...}` interpolation.
//! - Ruby `system(...)` with `#{...}` interpolation (excludes qualified
//!   `os.system(...)` / `Foo.system(...)` calls).
//! - Java `Runtime.getRuntime().exec(...)` — flagged only for a
//!   non-literal argument (typically `+` concatenation).

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// Whether a sink is flagged unconditionally on a bare regex match, or only
/// when its captured argument (capture group 1) is clearly non-literal.
enum SinkCheck {
    /// The regex match alone is sufficient (e.g. `shell=True`, or a
    /// backtick/`#{}` combo that already encodes non-literal interpolation).
    AlwaysFlag,
    /// Only flag when the captured argument (group 1) passes
    /// [`is_dynamic_argument`].
    DynamicArgument,
}

struct SinkPattern {
    label: &'static str,
    regex: Regex,
    check: SinkCheck,
}

/// A captured call argument is treated as non-literal when it contains
/// string concatenation (`+`), template/shell-style interpolation (`${` /
/// `#{`), an f-string prefix (`f"`/`F'`/...), or is a bare
/// variable/expression with no string-literal quote character at all. A
/// fully static, fully-quoted literal (e.g. `"ls -la"`) is never flagged.
fn is_dynamic_argument(argument: &str, fstring_prefix: &Regex) -> bool {
    let trimmed = argument.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains('+') || trimmed.contains("${") || trimmed.contains("#{") {
        return true;
    }
    if fstring_prefix.is_match(trimmed) {
        return true;
    }
    !trimmed.contains('"') && !trimmed.contains('\'') && !trimmed.contains('`')
}

/// `CYBER-CMD-INJECT.1` — command-injection sink detector (T1 per-sink
/// matcher over source lines).
pub struct CommandInjectionValidator {
    rule_id: RuleId,
    sinks: Vec<SinkPattern>,
    fstring_prefix: Regex,
}

impl CommandInjectionValidator {
    pub fn new() -> Result<Self, DecodeError> {
        fn compile(slug: &'static str, pattern: &str) -> Result<Regex, DecodeError> {
            Regex::new(pattern).map_err(|err| DecodeError::new(slug, err.to_string()))
        }

        let sinks = vec![
            SinkPattern {
                label: "Python subprocess(..., shell=True)",
                regex: compile(
                    "cyberskillsCmdInjectSubprocessShellTrue",
                    r"subprocess\.\w+\([^\n]*shell\s*=\s*True\b",
                )?,
                check: SinkCheck::AlwaysFlag,
            },
            SinkPattern {
                label: "Python os.system(...)",
                regex: compile("cyberskillsCmdInjectOsSystem", r"os\.system\s*\((.*?)\)")?,
                check: SinkCheck::DynamicArgument,
            },
            SinkPattern {
                label: "Node child_process.exec()/execSync()",
                regex: compile(
                    "cyberskillsCmdInjectChildProcessExec",
                    r"child_process\.(?:exec|execSync)\s*\((.*?)\)",
                )?,
                check: SinkCheck::DynamicArgument,
            },
            SinkPattern {
                label: "eval()/exec() of a non-literal expression",
                regex: compile(
                    "cyberskillsCmdInjectEvalExec",
                    r"(?:^|[\s(;=,])(?:eval|exec)\s*\((.*?)\)",
                )?,
                check: SinkCheck::DynamicArgument,
            },
            SinkPattern {
                label: "Ruby backtick command substitution with #{} interpolation",
                regex: compile("cyberskillsCmdInjectRubyBacktick", r"`[^`\n]*#\{[^`\n]*`")?,
                check: SinkCheck::AlwaysFlag,
            },
            SinkPattern {
                label: "Ruby system() with #{} interpolation",
                regex: compile(
                    "cyberskillsCmdInjectRubySystem",
                    r"(?:^|[^.\w])system\s*\((.*?)\)",
                )?,
                check: SinkCheck::DynamicArgument,
            },
            SinkPattern {
                label: "Java Runtime.getRuntime().exec(...)",
                regex: compile(
                    "cyberskillsCmdInjectJavaRuntimeExec",
                    r"Runtime\s*\.\s*getRuntime\s*\(\s*\)\s*\.\s*exec\s*\((.*?)\)",
                )?,
                check: SinkCheck::DynamicArgument,
            },
        ];

        Ok(Self {
            rule_id: "CYBER-CMD-INJECT.1".parse()?,
            sinks,
            fstring_prefix: compile("cyberskillsCmdInjectFstringPrefix", r#"^[fF]['"]"#)?,
        })
    }
}

impl Validator for CommandInjectionValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let mut matched_labels: Vec<&str> = Vec::new();

            for sink in &self.sinks {
                match (&sink.check, sink.regex.captures(line)) {
                    (SinkCheck::AlwaysFlag, Some(_)) => matched_labels.push(sink.label),
                    (SinkCheck::DynamicArgument, Some(captures)) => {
                        let argument = captures.get(1).map(|m| m.as_str()).unwrap_or("");
                        if is_dynamic_argument(argument, &self.fstring_prefix) {
                            matched_labels.push(sink.label);
                        }
                    }
                    (_, None) => {}
                }
            }

            if matched_labels.is_empty() {
                continue;
            }

            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "Command executed with a non-literal, attacker-influenceable argument"
                    .to_owned(),
                detail: format!(
                    "Sink(s) matched: {}. A command/code-execution sink is invoked with a \
                     concatenated, interpolated, or variable argument (or, for \
                     `subprocess(..., shell=True)`, with the shell enabled at all), so an \
                     attacker who controls part of that input can inject arbitrary shell/OS \
                     commands. Fix: avoid the shell entirely (pass an argv list with \
                     `shell=False`/`execFile`, or the target language's non-shell exec API), and \
                     never build a shell command string from untrusted input.",
                    matched_labels.join(", ")
                ),
                file: input.file.clone(),
                line: line_number,
                snippet: Some(line.to_owned()),
            });
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::CommandInjectionValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_cmd_injection() -> Result<(), Box<dyn std::error::Error>> {
        let validator = CommandInjectionValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/web.command-injection/bad/inject.py",
            "tests/fixtures/cyberskills/web.command-injection/good/safe.py",
        )?;
        Ok(())
    }
}
