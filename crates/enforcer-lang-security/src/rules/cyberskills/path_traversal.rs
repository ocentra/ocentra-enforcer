//! `CYBER-PATH-TRAVERSAL.1` — harvested from
//! `vendor/anthropic-cybersecurity-skills/skills/performing-directory-traversal-testing`
//! (`SKILL.md` + `scripts/agent.py`). The vendor skill is a DYNAMIC testing
//! playbook: `agent.py` fires a `TRAVERSAL_PAYLOADS` list of `../`-style
//! strings (plain, URL-encoded, double-encoded, overlong-UTF8, PHP-wrapper,
//! etc.) at a *live* URL parameter and inspects the HTTP response body for
//! `LINUX_INDICATORS` / `WINDOWS_INDICATORS` (e.g. `"root:x:"`, `"[fonts]"`).
//! None of that is an inline source-code detection predicate we can port
//! verbatim: there is no static-analysis table in the vendor script, only a
//! payload list and a response-content oracle for a black-box HTTP probe.
//!
//! Per the "well-known standard" fallback: this validator instead implements
//! the standard Semgrep-style static rule for path traversal / LFI (the same
//! shape as `python.lang.security.audit.path-traversal-open` and
//! `javascript.express.security.audit.express-path-join-resolve-traversal`) —
//! a static source-code line-scan for (1) a `../`/`..%2f` traversal literal
//! reaching a file-op sink (`open(`, `readFile(`, `sendFile(`, `fopen(`,
//! `File(`), and (2) a file-op sink whose argument is built from an
//! obviously request-derived variable (name containing `req`/`request`/
//! `params`/`query`/`filename`/`user`). Both conditions are scoped to the
//! SAME line as the sink call to keep false positives low, mirroring the
//! vendor skill's own `file_param_names` allowlist
//! (`file, path, page, include, template, doc, ...`) used to decide which
//! parameters are even worth testing.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// `CYBER-PATH-TRAVERSAL.1` — static line-scan for path traversal / LFI
/// sinks fed a literal `../` sequence or a request-derived variable.
#[derive(Debug)]
pub struct PathTraversalValidator {
    rule_id: RuleId,
    sink_regex: Regex,
    traversal_regex: Regex,
    request_var_regex: Regex,
}

impl PathTraversalValidator {
    pub fn new() -> Result<Self, DecodeError> {
        // File-op sinks named in the workpack: open(, readFile/read_file(,
        // sendFile/send_file(, fopen(, File( (bare or `new File(`).
        let sink_regex = Regex::new(r"(?i)\b(?:open|read_?file|send_?file|fopen|file)\s*\(")
            .map_err(|err| crate::boundary::regex::decode("cyberskillsPathTraversalSink", err))?;
        // `../`, `..\`, or the URL-encoded `..%2f` traversal sequence.
        let traversal_regex = Regex::new(r"(?i)\.\.(?:[/\\]|%2f)").map_err(|err| {
            crate::boundary::regex::decode("cyberskillsPathTraversalLiteral", err)
        })?;
        // A word containing one of the request-derived name fragments
        // called out in the workpack.
        let request_var_regex = Regex::new(
            r"(?i)\b\w*(?:request|req|params|query|filename|user)\w*\b",
        )
        .map_err(|err| crate::boundary::regex::decode("cyberskillsPathTraversalRequestVar", err))?;
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberPathTraversal.id(),
            sink_regex,
            traversal_regex,
            request_var_regex,
        })
    }
}

impl Validator for PathTraversalValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.as_str().lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let Some(sink_match) = self.sink_regex.find(line) else {
                continue;
            };
            let sink_name = sink_match.as_str().trim_end_matches('(').trim();
            let Some(call_tail) = line.get(sink_match.end()..) else {
                continue;
            };

            match (
                self.traversal_regex.is_match(line),
                self.request_var_regex.is_match(call_tail),
            ) {
                (true, _) => {
                    findings.extend(crate::boundary::finding::from_source(
                        (&self.rule_id, Severity::Error),
                        "Path traversal literal passed to a file operation",
                        format!(
                            "A `../`/`..%2f` traversal sequence reaches the file-handling call \
                             `{sink_name}(`. Fix: reject any path containing traversal sequences, \
                             canonicalize the resolved path, and verify it stays within the \
                             intended base directory before opening it (see \
                             performing-directory-traversal-testing)."
                        ),
                        input.file,
                        (line_number, Some(line)),
                    ));
                }
                (false, true) => {
                    findings.extend(crate::boundary::finding::from_source(
                        (&self.rule_id, Severity::Error),
                        "Request-derived path passed to a file operation",
                        format!(
                            "The file-handling call `{sink_name}(` builds its path from a \
                             request-derived variable (name matches req/request/params/query/\
                             filename/user). Fix: validate the value against an allowlist or \
                             canonicalize it and verify it resolves within the intended base \
                             directory before using it in a file operation."
                        ),
                        input.file,
                        (line_number, Some(line)),
                    ));
                }
                (false, false) => {}
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use crate::boundary::fixture::run_manifest_fixture_parity;

    use super::PathTraversalValidator;

    #[test]
    fn cyberskills_path_traversal() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PathTraversalValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/web.path-traversal/bad/traverse.py",
            "tests/fixtures/cyberskills/web.path-traversal/good/safe.py",
        )?;
        Ok(())
    }
}
