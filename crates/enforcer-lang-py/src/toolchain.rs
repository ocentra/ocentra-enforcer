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

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::{Finding, FindingTitle};
use enforcer_domain::ids::{BuiltInPythonRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::{static_title, PythonFindingMessage};
use crate::boundary::source::{diagnostics_count, DiagnosticsArray};

/// A validator that fires when a toolchain's JSON diagnostics payload
/// contains at least one entry in `array_key` (e.g. Ruff's top-level JSON
/// array, or a `{"diagnostics": [...]}`/`{"generalDiagnostics": [...]}`
/// wrapper). Silent on empty/absent arrays and on non-JSON or malformed
/// input (a validator must never panic on adversarial/incomplete input;
/// "cannot prove a violation" is the fail-closed-safe default here, not
/// "assume a violation").
struct StructuredDiagnosticsValidator {
    rule_id: RuleId,
    title: FindingTitle,
    diagnostics_array: DiagnosticsArray,
}

impl Validator for StructuredDiagnosticsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let count = diagnostics_count(input.source, self.diagnostics_array);
        if count == 0 {
            return Vec::new();
        }
        crate::boundary::finding::from_python_source(
            &self.rule_id,
            Severity::Error,
            input.file,
            1,
            PythonFindingMessage::new(
                self.title.as_str(),
                format!("toolchain reported {count} diagnostic(s)"),
                None,
            ),
        )
        .into_iter()
        .collect()
    }
}

/// Build every `python/ruff-json` + `python/typecheck` +
/// `python/toolchain`-keyed validator this crate registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(StructuredDiagnosticsValidator {
            rule_id: BuiltInPythonRule::Py3Rule1.id(),
            title: static_title("Ruff diagnostics must pass")?,
            diagnostics_array: DiagnosticsArray::Root,
        }),
        Box::new(StructuredDiagnosticsValidator {
            rule_id: BuiltInPythonRule::Py3Rule2.id(),
            title: static_title("Python type-check diagnostics must pass")?,
            diagnostics_array: DiagnosticsArray::GeneralDiagnostics,
        }),
        Box::new(StructuredDiagnosticsValidator {
            rule_id: BuiltInPythonRule::Py5Rule5.id(),
            title: static_title("Ruff diagnostics must be structured")?,
            diagnostics_array: DiagnosticsArray::Root,
        }),
        Box::new(StructuredDiagnosticsValidator {
            rule_id: BuiltInPythonRule::Py5Rule6.id(),
            title: static_title("Python type diagnostics must be structured")?,
            diagnostics_array: DiagnosticsArray::GeneralDiagnostics,
        }),
    ])
}
