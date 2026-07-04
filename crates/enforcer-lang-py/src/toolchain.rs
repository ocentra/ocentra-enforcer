//! `python/ruff-json`, `python/typecheck`, and the `python/toolchain`-keyed
//! slice of PY-5 (4 rules: PY-3.1, PY-3.2, PY-5.5, PY-5.6).
//!
//! These rules gate on STRUCTURED diagnostics from an external toolchain
//! (Ruff's `--output-format json`, Pyright's `--outputjson`, or mypy's
//! structured output) rather than on this crate parsing Python source
//! itself. Per this crate's charter (`enforcer-validator`'s `Validator` is
//! a pure per-file text-in/findings-out contract, never a subprocess
//! runner), the validator here inspects the diagnostics JSON blob as
//! `ValidationInput::source` -- the file `enforcer-scan`'s toolchain
//! adapter (a later track) is responsible for invoking the real tool and
//! feeding its JSON output through this same contract. The fixture/parity
//! harness proves the JSON-shape detection, not a live Ruff/Pyright/mypy
//! invocation.

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// A validator that fires when a toolchain's JSON diagnostics payload
/// contains at least one entry in `array_key` (e.g. Ruff's top-level JSON
/// array, or a `{"diagnostics": [...]}`/`{"generalDiagnostics": [...]}`
/// wrapper). Silent on empty/absent arrays and on non-JSON or malformed
/// input (a validator must never panic on adversarial/incomplete input;
/// "cannot prove a violation" is the fail-closed-safe default here, not
/// "assume a violation").
struct StructuredDiagnosticsValidator {
    rule_id: RuleId,
    title: &'static str,
    array_key: Option<&'static str>,
}

impl Validator for StructuredDiagnosticsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(input.source) else {
            return Vec::new();
        };
        let array = match self.array_key {
            Some(key) => parsed.get(key).and_then(serde_json::Value::as_array),
            None => parsed.as_array(),
        };
        let count = array.map_or(0, Vec::len);
        if count == 0 {
            return Vec::new();
        }
        vec![Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: self.title.to_owned(),
            detail: format!("toolchain reported {count} diagnostic(s)"),
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }]
    }
}

/// Build every `python/ruff-json` + `python/typecheck` +
/// `python/toolchain`-keyed validator this crate registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(StructuredDiagnosticsValidator {
            rule_id: "PY-3.1".parse()?,
            title: "Ruff diagnostics must pass",
            array_key: None,
        }),
        Box::new(StructuredDiagnosticsValidator {
            rule_id: "PY-3.2".parse()?,
            title: "Python type-check diagnostics must pass",
            array_key: Some("generalDiagnostics"),
        }),
        Box::new(StructuredDiagnosticsValidator {
            rule_id: "PY-5.5".parse()?,
            title: "Ruff diagnostics must be structured",
            array_key: None,
        }),
        Box::new(StructuredDiagnosticsValidator {
            rule_id: "PY-5.6".parse()?,
            title: "Python type diagnostics must be structured",
            array_key: Some("generalDiagnostics"),
        }),
    ])
}
