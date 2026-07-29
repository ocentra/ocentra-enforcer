//! Half A — run-adapter PARSING.
//!
//! Ported from `src/harness-parsers.mjs`, `src/harness-parsers-json-lines.mjs`,
//! `src/harness-parsers-json-payload.mjs`, and
//! `src/harness-parsers-json-diagnostics.mjs`.
//!
//! Parses native-tool stdout/stderr into diagnostics: rustc/cargo
//! `--message-format=json` compiler messages, ESLint/bandit/pyright/SARIF
//! JSON payloads, `tsc` human-readable text, and `pytest` `FAILED` lines.
//! Unparseable JSON becomes a graceful `HAR-2.4` skip diagnostic rather than
//! a hard failure — mirrors `parserDiagnostic` in the legacy `.mjs`.

use enforcer_domain::harness_types::{
    HarnessCapturedOutput, HarnessDiagnosticFingerprint, HarnessDiagnosticMessage,
    HarnessDiagnosticPath, HarnessExternalRuleId, HarnessLanguage, HarnessRunId, HarnessSourceLine,
    HarnessToolName,
};
use enforcer_domain::severity::Severity;
use regex::Regex;
use serde_json::Value;

/// One parsed diagnostic occurrence. Deliberately NOT `enforcer_domain::Finding`
/// (that type owns validator-report shape and requires strict `RuleId`
/// formatting); this is the harness's own wire-adjacent record, matching
/// the legacy `.mjs` diagnostic object shape field-for-field so `[REDACTED]`
/// text carries straight through into `diagnostics.ndjson`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessDiagnostic {
    pub run_id: HarnessRunId,
    pub tool: HarnessToolName,
    pub language: HarnessLanguage,
    pub severity: Severity,
    pub rule_id: HarnessExternalRuleId,
    pub file: HarnessDiagnosticPath,
    pub line: HarnessSourceLine,
    pub message: HarnessDiagnosticMessage,
    pub source: Option<HarnessCapturedOutput>,
    pub fingerprint: Option<HarnessDiagnosticFingerprint>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct HarnessDiagnosticDto {
    run_id: String,
    tool: String,
    language: String,
    severity: String,
    rule_id: String,
    file: String,
    line: u64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fingerprint: Option<String>,
}

impl From<&HarnessDiagnostic> for HarnessDiagnosticDto {
    fn from(diagnostic: &HarnessDiagnostic) -> Self {
        Self {
            run_id: diagnostic.run_id.as_str().to_owned(),
            tool: diagnostic.tool.as_str().to_owned(),
            language: diagnostic.language.as_str().to_owned(),
            severity: severity_label(diagnostic.severity).to_owned(),
            rule_id: diagnostic.rule_id.as_str().to_owned(),
            file: diagnostic.file.as_str().to_owned(),
            line: diagnostic.line.get(),
            message: diagnostic.message.as_str().to_owned(),
            source: diagnostic
                .source
                .as_ref()
                .map(|value| value.as_str().to_owned()),
            fingerprint: diagnostic
                .fingerprint
                .as_ref()
                .map(|value| value.as_str().to_owned()),
        }
    }
}

impl From<HarnessDiagnosticDto> for HarnessDiagnostic {
    fn from(diagnostic: HarnessDiagnosticDto) -> Self {
        Self {
            run_id: HarnessRunId::from_adapter(&diagnostic.run_id),
            tool: HarnessToolName::from_adapter(&diagnostic.tool),
            language: language_from_label(&diagnostic.language),
            severity: severity_from_label(&diagnostic.severity),
            rule_id: HarnessExternalRuleId::from_adapter(&diagnostic.rule_id),
            file: HarnessDiagnosticPath::from_adapter(&diagnostic.file),
            line: HarnessSourceLine::from_external(diagnostic.line),
            message: HarnessDiagnosticMessage::from_adapter(&diagnostic.message),
            source: diagnostic.source.map(HarnessCapturedOutput::from_owned),
            fingerprint: diagnostic
                .fingerprint
                .and_then(|value| HarnessDiagnosticFingerprint::try_new(value).ok()),
        }
    }
}

pub(crate) fn severity_from_label(value: &str) -> Severity {
    match value.to_ascii_lowercase().as_str() {
        "error" => Severity::Error,
        "info" | "note" => Severity::Info,
        _ => Severity::Warning,
    }
}

fn severity_label(value: Severity) -> &'static str {
    match value {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

pub(crate) fn language_from_label(value: &str) -> HarnessLanguage {
    match value.to_ascii_lowercase().as_str() {
        "rust" => HarnessLanguage::Rust,
        "typescript" => HarnessLanguage::Typescript,
        "python" => HarnessLanguage::Python,
        _ => HarnessLanguage::Common,
    }
}

#[derive(Clone, Copy)]
struct DiagnosticInput<'a> {
    run_id: &'a str,
    tool: &'a str,
    language: HarnessLanguage,
    severity: Severity,
    rule_id: &'a str,
    file: &'a str,
    line: u64,
    message: &'a str,
}

fn diagnostic(input: DiagnosticInput<'_>) -> HarnessDiagnostic {
    HarnessDiagnostic {
        run_id: HarnessRunId::from_adapter(input.run_id),
        tool: HarnessToolName::from_adapter(input.tool),
        language: input.language,
        severity: input.severity,
        rule_id: HarnessExternalRuleId::from_adapter(input.rule_id),
        file: HarnessDiagnosticPath::from_adapter(input.file),
        line: HarnessSourceLine::from_external(input.line),
        message: HarnessDiagnosticMessage::from_adapter(input.message),
        source: None,
        fingerprint: None,
    }
}

/// Parse combined stdout+stderr text into diagnostics across every known
/// adapter shape. Never panics/hard-fails on malformed input — unparseable
/// JSON becomes a graceful skip diagnostic.
/// PROPERTY-TEST: `tests/parser_properties.rs` supplies arbitrary text twice
/// and asserts deterministic diagnostics without panics.
pub fn parse_diagnostics(
    run_id: &str,
    tool: &str,
    stdout: &str,
    stderr: &str,
) -> Vec<HarnessDiagnostic> {
    let text = [stdout, stderr]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = Vec::new();
    out.extend(parse_json_lines(run_id, tool, &text));
    out.extend(parse_json_payload(run_id, tool, &text));
    out.extend(parse_tsc_text(run_id, tool, &text));
    out.extend(parse_pytest_text(run_id, tool, &text));
    out
}

/// Graceful-skip diagnostic: reports that a tool/parser step was skipped
/// (e.g. tool executable absent, malformed JSON) WITHOUT hard-failing the run.
pub fn skip_diagnostic(run_id: &str, tool: &str, context: &str, detail: &str) -> HarnessDiagnostic {
    let message = format!("Harness parser ignored {context}: {detail}");
    diagnostic(DiagnosticInput {
        run_id,
        tool,
        language: infer_language(tool),
        severity: Severity::Warning,
        rule_id: "HAR-2.4",
        file: ".",
        line: 1,
        message: &message,
    })
}

/// A tool executable was not found on `PATH`. Report skip, do NOT hard-fail —
/// the distribution-doctrine graceful-skip seam.
pub fn missing_tool_skip(run_id: &str, tool: &str) -> HarnessDiagnostic {
    skip_diagnostic(
        run_id,
        tool,
        "missing tool",
        &format!("`{tool}` was not found on PATH"),
    )
}

/// Infer a coarse language tag from a tool executable name (`cargo` -> `rust`,
/// `tsc`/`eslint` -> `typescript`, `ruff`/`pytest` -> `python`, else
/// `common`). Mirrors `inferLanguage` in `src/harness.mjs`.
pub fn infer_language(tool: &str) -> HarnessLanguage {
    let normalized = tool.to_ascii_lowercase();
    if normalized.contains("cargo") || normalized.contains("rust") {
        HarnessLanguage::Rust
    } else if normalized.contains("ts")
        || normalized.contains("eslint")
        || normalized.contains("vite")
    {
        HarnessLanguage::Typescript
    } else if normalized.contains("py")
        || normalized.contains("ruff")
        || normalized.contains("pytest")
    {
        HarnessLanguage::Python
    } else {
        HarnessLanguage::Common
    }
}

fn normalize_rel(value: &str) -> String {
    value.replace('\\', "/")
}

// ---------------------------------------------------------------------
// JSON-lines (rustc/cargo `--message-format=json`)
// ---------------------------------------------------------------------

fn parse_json_lines(run_id: &str, tool: &str, text: &str) -> Vec<HarnessDiagnostic> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(parsed) => out.extend(maybe_rust_compiler_message(run_id, tool, &parsed)),
            Err(err) => out.push(skip_diagnostic(
                run_id,
                tool,
                "malformed JSON line",
                &err.to_string(),
            )),
        }
    }
    out
}

fn maybe_rust_compiler_message(run_id: &str, tool: &str, parsed: &Value) -> Vec<HarnessDiagnostic> {
    if parsed.get("reason").and_then(Value::as_str) == Some("compiler-message") {
        if let Some(message) = parsed.get("message") {
            return vec![rust_message_to_diagnostic(run_id, tool, message)];
        }
    }
    Vec::new()
}

/// Convert one rustc `compiler-message` JSON payload into a diagnostic.
pub fn rust_message_to_diagnostic(run_id: &str, tool: &str, message: &Value) -> HarnessDiagnostic {
    let span = primary_span(message);
    let level = message
        .get("level")
        .and_then(Value::as_str)
        .unwrap_or("warning");
    let rule_id = message
        .get("code")
        .and_then(|c| c.get("code"))
        .and_then(Value::as_str)
        .unwrap_or("rustc")
        .to_owned();
    let file = span
        .and_then(|s| s.get("file_name"))
        .and_then(Value::as_str)
        .map(normalize_rel)
        .unwrap_or_else(|| ".".to_owned());
    let line = span
        .and_then(|s| s.get("line_start"))
        .and_then(Value::as_u64)
        .unwrap_or(1);
    diagnostic(DiagnosticInput {
        run_id,
        tool,
        language: HarnessLanguage::Rust,
        severity: severity_from_label(level),
        rule_id: &rule_id,
        file: &file,
        line,
        message: message
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    })
}

fn primary_span(message: &Value) -> Option<&Value> {
    let spans = message.get("spans")?.as_array()?;
    spans
        .iter()
        .find(|s| s.get("is_primary").and_then(Value::as_bool) == Some(true))
        .or_else(|| spans.first())
}

// ---------------------------------------------------------------------
// JSON payload (whole-document): ESLint / bandit / pyright / SARIF
// ---------------------------------------------------------------------

fn parse_json_payload(run_id: &str, tool: &str, text: &str) -> Vec<HarnessDiagnostic> {
    let trimmed = text.trim();
    if !(trimmed.starts_with('[') || trimmed.starts_with('{')) {
        return Vec::new();
    }
    let parsed = match serde_json::from_str::<Value>(trimmed) {
        Ok(v) => v,
        Err(err) => {
            return vec![skip_diagnostic(
                run_id,
                tool,
                "malformed JSON payload",
                &err.to_string(),
            )]
        }
    };
    if is_eslint_payload(&parsed) {
        return eslint_diagnostics(run_id, tool, &parsed);
    }
    if is_bandit_payload(&parsed) {
        return bandit_diagnostics(run_id, tool, &parsed);
    }
    if let Some(entries) = parsed.get("generalDiagnostics").and_then(Value::as_array) {
        return pyright_diagnostics(run_id, tool, entries);
    }
    if let Some(runs) = parsed.get("runs").and_then(Value::as_array) {
        return sarif_diagnostics(run_id, tool, runs);
    }
    Vec::new()
}

fn is_eslint_payload(parsed: &Value) -> bool {
    parsed.as_array().is_some_and(|entries| {
        !entries.is_empty()
            && entries.iter().all(|e| {
                e.get("filePath").is_some() && e.get("messages").and_then(Value::as_array).is_some()
            })
    })
}

fn is_bandit_payload(parsed: &Value) -> bool {
    parsed.as_array().is_some_and(|entries| {
        !entries.is_empty()
            && entries
                .iter()
                .all(|e| e.get("filename").is_some() && e.get("code").is_some())
    })
}

fn eslint_diagnostics(run_id: &str, tool: &str, parsed: &Value) -> Vec<HarnessDiagnostic> {
    let mut out = Vec::new();
    for entry in parsed.as_array().into_iter().flatten() {
        let file = entry
            .get("filePath")
            .and_then(Value::as_str)
            .unwrap_or_default();
        for message in entry
            .get("messages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let normalized_file = normalize_rel(file);
            out.push(diagnostic(DiagnosticInput {
                run_id,
                tool,
                language: HarnessLanguage::Typescript,
                severity: if message.get("severity").and_then(Value::as_i64) == Some(2) {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                rule_id: message
                    .get("ruleId")
                    .and_then(Value::as_str)
                    .unwrap_or("eslint"),
                file: &normalized_file,
                line: message.get("line").and_then(Value::as_u64).unwrap_or(1),
                message: message
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            }));
        }
    }
    out
}

fn bandit_diagnostics(run_id: &str, tool: &str, parsed: &Value) -> Vec<HarnessDiagnostic> {
    parsed
        .as_array()
        .into_iter()
        .flatten()
        .map(|entry| {
            let file = normalize_rel(
                entry
                    .get("filename")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            );
            diagnostic(DiagnosticInput {
                run_id,
                tool,
                language: HarnessLanguage::Python,
                severity: Severity::Error,
                rule_id: entry
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("bandit"),
                file: &file,
                line: entry
                    .get("location")
                    .and_then(|l| l.get("row"))
                    .and_then(Value::as_u64)
                    .unwrap_or(1),
                message: entry
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            })
        })
        .collect()
}

fn pyright_diagnostics(run_id: &str, tool: &str, entries: &[Value]) -> Vec<HarnessDiagnostic> {
    entries
        .iter()
        .map(|entry| {
            let file = entry
                .get("file")
                .and_then(Value::as_str)
                .map(normalize_rel)
                .unwrap_or_else(|| ".".to_owned());
            diagnostic(DiagnosticInput {
                run_id,
                tool,
                language: HarnessLanguage::Python,
                severity: severity_from_label(
                    entry
                        .get("severity")
                        .and_then(Value::as_str)
                        .unwrap_or("error"),
                ),
                rule_id: "pyright",
                file: &file,
                line: entry
                    .get("range")
                    .and_then(|r| r.get("start"))
                    .and_then(|s| s.get("line"))
                    .and_then(Value::as_u64)
                    .map(|l| l + 1)
                    .unwrap_or(1),
                message: entry
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            })
        })
        .collect()
}

fn sarif_severity(level: &str) -> Severity {
    match level {
        "error" => Severity::Error,
        "warning" => Severity::Warning,
        "note" | "none" => Severity::Info,
        _ => Severity::Warning,
    }
}

fn sarif_diagnostics(run_id: &str, tool: &str, runs: &[Value]) -> Vec<HarnessDiagnostic> {
    let mut out = Vec::new();
    for run in runs {
        for result in run
            .get("results")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let location = result
                .get("locations")
                .and_then(Value::as_array)
                .and_then(|l| l.first())
                .and_then(|l| l.get("physicalLocation"));
            let region = location.and_then(|l| l.get("region"));
            let uri = location
                .and_then(|l| l.get("artifactLocation"))
                .and_then(|a| a.get("uri"))
                .and_then(Value::as_str)
                .unwrap_or(".");
            let level = result
                .get("level")
                .and_then(Value::as_str)
                .unwrap_or("warning");
            let normalized_uri = normalize_rel(uri);
            out.push(diagnostic(DiagnosticInput {
                run_id,
                tool,
                language: HarnessLanguage::Common,
                severity: sarif_severity(level),
                rule_id: result
                    .get("ruleId")
                    .and_then(Value::as_str)
                    .or_else(|| {
                        result
                            .get("rule")
                            .and_then(|r| r.get("id"))
                            .and_then(Value::as_str)
                    })
                    .unwrap_or("sarif"),
                file: &normalized_uri,
                line: region
                    .and_then(|r| r.get("startLine"))
                    .and_then(Value::as_u64)
                    .unwrap_or(1),
                message: result
                    .get("message")
                    .and_then(|m| m.get("text").or_else(|| m.get("markdown")))
                    .and_then(Value::as_str)
                    .unwrap_or("SARIF result"),
            }));
        }
    }
    out
}

// ---------------------------------------------------------------------
// Text-format adapters: tsc, pytest
// ---------------------------------------------------------------------

fn parse_tsc_text(run_id: &str, tool: &str, text: &str) -> Vec<HarnessDiagnostic> {
    let re = match Regex::new(r"(?m)^(.+?)\((\d+),(\d+)\):\s+(error|warning)\s+(TS\d+):\s+(.+)$") {
        Ok(regex) => regex,
        Err(error) => {
            return vec![skip_diagnostic(
                run_id,
                tool,
                "tsc parser",
                &error.to_string(),
            )]
        }
    };
    re.captures_iter(text)
        .filter_map(|cap| {
            let file = cap.get(1)?.as_str();
            let line = cap.get(2)?.as_str().parse().unwrap_or(1);
            let severity = cap.get(4)?.as_str();
            let rule_id = cap.get(5)?.as_str();
            let message = cap.get(6)?.as_str();
            let normalized_file = normalize_rel(file);
            Some(diagnostic(DiagnosticInput {
                run_id,
                tool,
                language: HarnessLanguage::Typescript,
                severity: severity_from_label(severity),
                rule_id,
                file: &normalized_file,
                line,
                message,
            }))
        })
        .collect()
}

fn parse_pytest_text(run_id: &str, tool: &str, text: &str) -> Vec<HarnessDiagnostic> {
    let re = match Regex::new(r"(?m)^FAILED\s+([^:\s]+(?:::[^\s]+)*)\s+-\s+(.+)$") {
        Ok(regex) => regex,
        Err(error) => {
            return vec![skip_diagnostic(
                run_id,
                tool,
                "pytest parser",
                &error.to_string(),
            )]
        }
    };
    re.captures_iter(text)
        .filter_map(|cap| {
            let path = cap.get(1)?.as_str().split("::").next().unwrap_or_default();
            let message = cap.get(2)?.as_str();
            Some(diagnostic(DiagnosticInput {
                run_id,
                tool,
                language: HarnessLanguage::Python,
                severity: Severity::Error,
                rule_id: "pytest",
                file: path,
                line: 1,
                message,
            }))
        })
        .collect()
}

// ---------------------------------------------------------------------
// Compact diagnostics: dedupe (with fingerprint) + sort
// ---------------------------------------------------------------------

/// Deduplicate diagnostics by `(tool, ruleId, file, line, message)`,
/// stamping each survivor with a stable base64url fingerprint of that key —
/// mirrors `dedupeDiagnostics` in `src/harness-parsers.mjs`.
pub fn dedupe_diagnostics(diagnostics: Vec<HarnessDiagnostic>) -> Vec<HarnessDiagnostic> {
    use std::collections::HashSet;
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for mut diagnostic in diagnostics {
        let key = format!(
            "{}|{}|{}|{}|{}",
            diagnostic.tool,
            diagnostic.rule_id,
            diagnostic.file,
            diagnostic.line,
            diagnostic.message
        );
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key.clone());
        diagnostic.fingerprint = Some(fingerprint(&key));
        out.push(diagnostic);
    }
    out
}

fn fingerprint(key: &str) -> HarnessDiagnosticFingerprint {
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.as_bytes());
    let compact: String = encoded.chars().take(24).collect();
    HarnessDiagnosticFingerprint::from_digest(compact)
}

/// Sort diagnostics by `(file, line, ruleId, message)` — mirrors
/// `sortDiagnostics` in `src/harness-parsers.mjs`.
pub fn sort_diagnostics(mut diagnostics: Vec<HarnessDiagnostic>) -> Vec<HarnessDiagnostic> {
    diagnostics.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.rule_id.cmp(&b.rule_id))
            .then(a.message.cmp(&b.message))
    });
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::{
        dedupe_diagnostics, missing_tool_skip, parse_diagnostics, sort_diagnostics,
        HarnessDiagnostic, HarnessDiagnosticDto,
    };

    #[test]
    fn harness_diagnostic_wire_roundtrip_preserves_domain_record() -> Result<(), serde_json::Error>
    {
        let mut diagnostic = super::diagnostic(super::DiagnosticInput {
            run_id: "roundtrip-run",
            tool: "cargo",
            language: enforcer_domain::harness_types::HarnessLanguage::Rust,
            severity: enforcer_domain::severity::Severity::Error,
            rule_id: "E0308",
            file: "src/lib.rs",
            line: 7,
            message: "mismatched types",
        });
        diagnostic.fingerprint = Some(
            enforcer_domain::harness_types::HarnessDiagnosticFingerprint::from_digest(
                "fingerprint".to_owned(),
            ),
        );
        let wire = serde_json::to_string(&HarnessDiagnosticDto::from(&diagnostic))?;
        let decoded: HarnessDiagnosticDto = serde_json::from_str(&wire)?;
        assert_eq!(HarnessDiagnostic::from(decoded), diagnostic);
        Ok(())
    }

    #[test]
    fn harness_diagnostic_dto_rejects_incomplete_wire_shape() {
        let invalid = r#"{"run_id":"run","tool":"cargo"}"#;
        let decoded = serde_json::from_str::<HarnessDiagnosticDto>(invalid);
        assert!(
            decoded.is_err(),
            "missing diagnostic fields must be rejected"
        );
    }

    #[test]
    fn rustc_compiler_message_fail_fixture_produces_error_finding() {
        let line = serde_json::json!({
            "reason": "compiler-message",
            "message": {
                "level": "error",
                "code": { "code": "E0308" },
                "message": "mismatched types",
                "spans": [{ "is_primary": true, "file_name": "src/lib.rs", "line_start": 12 }]
            }
        })
        .to_string();
        let diagnostics = parse_diagnostics("run-1", "cargo", &line, "");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].severity,
            enforcer_domain::severity::Severity::Error
        );
        assert_eq!(diagnostics[0].rule_id, "E0308");
        assert_eq!(diagnostics[0].file, "src/lib.rs");
        assert_eq!(diagnostics[0].line, 12);
    }

    #[test]
    fn rustc_clean_output_pass_fixture_produces_no_findings() {
        let line = serde_json::json!({ "reason": "build-finished", "success": true }).to_string();
        let diagnostics = parse_diagnostics("run-1", "cargo", &line, "");
        assert!(
            diagnostics.is_empty(),
            "clean output must not emit findings"
        );
    }

    #[test]
    fn missing_tool_is_a_graceful_skip_not_a_hard_fail() {
        let skip = missing_tool_skip("run-1", "cflint");
        assert_eq!(skip.rule_id, "HAR-2.4");
        assert_eq!(skip.severity, enforcer_domain::severity::Severity::Warning);
        assert_eq!(
            skip.message,
            "Harness parser ignored missing tool: `cflint` was not found on PATH"
        );
    }

    #[test]
    fn malformed_json_line_is_graceful_skip_not_panic() {
        // The line-scanner and whole-payload adapter both attempt to parse
        // the same malformed blob (matches the legacy `.mjs` behavior of
        // running every adapter over the combined text) so a single bad
        // line yields a skip diagnostic from each; neither path panics or
        // hard-fails, which is the property under test.
        let diagnostics = parse_diagnostics("run-1", "cargo", "{not json", "");
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().all(|d| d.rule_id == "HAR-2.4"
            && d.severity == enforcer_domain::severity::Severity::Warning));
    }

    #[test]
    fn tsc_text_fail_fixture_parses_error_line() {
        let text = "src/app.ts(10,5): error TS2322: Type mismatch.\nOK.";
        let diagnostics = parse_diagnostics("run-1", "tsc", text, "");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "TS2322");
        assert_eq!(diagnostics[0].file, "src/app.ts");
        assert_eq!(diagnostics[0].line, 10);
    }

    #[test]
    fn tsc_clean_output_pass_fixture_produces_no_findings() {
        let diagnostics = parse_diagnostics(
            "run-1",
            "tsc",
            "Compilation complete. Watching for file changes.",
            "",
        );
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn pytest_failed_line_fail_fixture_parses() {
        let text = "FAILED tests/test_x.py::test_thing - AssertionError: boom";
        let diagnostics = parse_diagnostics("run-1", "pytest", text, "");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "pytest");
        assert_eq!(diagnostics[0].file, "tests/test_x.py");
    }

    #[test]
    fn eslint_json_payload_parses_error_and_warning_severity() {
        let payload = serde_json::json!([
            {
                "filePath": "src/x.ts",
                "messages": [
                    { "ruleId": "no-unused-vars", "severity": 2, "line": 3, "message": "unused" },
                    { "ruleId": "prefer-const", "severity": 1, "line": 4, "message": "use const" }
                ]
            }
        ])
        .to_string();
        let diagnostics = parse_diagnostics("run-1", "eslint", &payload, "");
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[0].severity,
            enforcer_domain::severity::Severity::Error
        );
        assert_eq!(
            diagnostics[1].severity,
            enforcer_domain::severity::Severity::Warning
        );
    }

    #[test]
    fn sarif_payload_maps_level_to_severity() {
        let payload = serde_json::json!({
            "runs": [{
                "results": [{
                    "ruleId": "CFL-1",
                    "level": "error",
                    "message": { "text": "boom" },
                    "locations": [{ "physicalLocation": { "artifactLocation": { "uri": "a.cfc" }, "region": { "startLine": 5 } } }]
                }]
            }]
        })
        .to_string();
        let diagnostics = parse_diagnostics("run-1", "cflint", &payload, "");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].severity,
            enforcer_domain::severity::Severity::Error
        );
        assert_eq!(diagnostics[0].file, "a.cfc");
        assert_eq!(diagnostics[0].line, 5);
    }

    #[test]
    fn dedupe_collapses_identical_diagnostics_and_stamps_fingerprint() {
        let d = super::diagnostic(super::DiagnosticInput {
            run_id: "r",
            tool: "cargo",
            language: enforcer_domain::harness_types::HarnessLanguage::Rust,
            severity: enforcer_domain::severity::Severity::Error,
            rule_id: "E0308",
            file: "a.rs",
            line: 1,
            message: "boom",
        });
        let out = dedupe_diagnostics(vec![d.clone(), d]);
        assert_eq!(out.len(), 1);
        assert_eq!(
            out.first()
                .and_then(|diagnostic| diagnostic.fingerprint.as_ref())
                .map(|value| value.as_str())
                .map(str::len),
            Some(24)
        );
    }

    #[test]
    fn sort_orders_by_file_then_line_then_rule_then_message() {
        let mk = |file: &str, line: u64, rule: &str| {
            super::diagnostic(super::DiagnosticInput {
                run_id: "r",
                tool: "t",
                language: enforcer_domain::harness_types::HarnessLanguage::Common,
                severity: enforcer_domain::severity::Severity::Error,
                rule_id: rule,
                file,
                line,
                message: "m",
            })
        };
        let sorted = sort_diagnostics(vec![
            mk("b.rs", 1, "Z"),
            mk("a.rs", 2, "A"),
            mk("a.rs", 1, "B"),
        ]);
        assert_eq!(sorted[0].file, "a.rs");
        assert_eq!(sorted[0].line, 1);
        assert_eq!(sorted[1].line, 2);
        assert_eq!(sorted[2].file, "b.rs");
    }
}
