//! `CYBER-MASS-ASSIGN.1` (T1) — harvest target: the vendored
//! `exploiting-mass-assignment-in-rest-apis` cyberskill
//! (`vendor/anthropic-cybersecurity-skills/skills/exploiting-mass-assignment-in-rest-apis/SKILL.md`
//! and its `scripts/agent.py`). Like several other h11 web-attack cyberskills,
//! this vendor skill is a live-target attack playbook: `agent.py` fires live
//! HTTP PUT/PATCH/POST requests carrying injected privilege/financial/status
//! fields (its `PRIVILEGE_FIELDS` table: `role`, `is_admin`, `isAdmin`,
//! `verified`, `balance`, `plan`, ...) at a running API to see whether the
//! server accepts them — there is no inline static-analysis routine to port
//! verbatim. This validator instead implements the deterministic,
//! Semgrep-style SOURCE check for the root cause the skill names throughout
//! its "Key Concepts" table (`Mass Assignment`, `Autobinding`) and its
//! "Remediation" list (`Allowlist`/Rails `strong_parameters`/DTO): a WHOLE
//! untrusted request object bound directly into a model/entity with no
//! field allowlist, narrowed to concrete call-site shapes so only genuine
//! unfiltered binds trigger, per the PREVENTION-gate false-positive budget:
//!
//! - Python: a capitalized model constructor, or an `.update(...)` call,
//!   given `**request.json` / `**request.get_json()` / `**request.form` /
//!   `**request.POST` (Django) / `**request.data` (DRF) — the double-splat
//!   unpacks every request key as a keyword argument with no allowlist.
//! - Python: a `setattr(obj, key, value)` call anywhere in a file that also
//!   contains a bare `for key, value in request.json.items():`-shaped
//!   statement over one of the same five whole-request accessors — the
//!   hand-rolled equivalent of the double-splat bind. The loop header must
//!   be a whole statement line (nothing else on the line beside the
//!   trailing `:`), so a filtered dict/set comprehension that merely
//!   contains the same `for k, v in request.json.items()` token sequence
//!   mid-expression (e.g. `{k: v for k, v in request.json.items() if k in
//!   ALLOWED}`) is correctly left clean.
//! - JS/Node: `new Model(req.body)`, `Object.assign(model, req.body)`,
//!   `model.set(req.body)`, `Model.create(req.body)`, `Model.update(req.body)`,
//!   and `findByIdAndUpdate(id, req.body)` — each requires the bound
//!   argument to be the BARE `req.body`/`req.query`/`req.params` token
//!   (immediately followed by `,` or `)`), which is what distinguishes an
//!   unfiltered whole-object bind from an explicit single-field access such
//!   as `req.body.name` or an inline allowlist literal such as
//!   `{ name: req.body.name }`.
//! - Ruby: `Model.new(params)`, `.update(params)`, and
//!   `.update_attributes(params)` given the bare `params` token — Rails'
//!   raw, unpermitted parameters hash, as opposed to
//!   `params.require(:user).permit(:name, :email)` or a permitted result
//!   assigned to its own variable (`user_params`).
//!
//! All findings are `Severity::Error`: every sink here is a concrete,
//! attacker-reachable bind of an entire untrusted object onto a
//! model/entity, matching this rule's PREDICATE. Detection deliberately does
//! not gate on file extension: the discriminator is the unfiltered
//! whole-request bind, the same source-level shape across Python/JS/Ruby.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// One high-confidence unfiltered whole-object-bind call-site pattern.
struct MassAssignPattern {
    regex: &'static str,
    label: &'static str,
}

/// Call-site patterns for a WHOLE untrusted request object bound directly
/// into a model/entity with no field allowlist (SKILL.md's "Mass
/// Assignment"/"Autobinding" concepts; see the module doc for the full
/// mapping from each entry back to its named sink).
const MASS_ASSIGN_PATTERNS_SRC: &[MassAssignPattern] = &[
    // Python: capitalized model constructor given **request.<whole-body>.
    MassAssignPattern {
        regex: r"\b[A-Z][A-Za-z0-9_]*\s*\(\s*\*\*\s*request\.(?:json\b|get_json\s*\(\s*\)|form\b|POST\b|data\b)",
        label: "Python model constructor bound from **request.json/get_json()/form/POST/data",
    },
    // Python: .update(**request.<whole-body>) with no allowlist.
    MassAssignPattern {
        regex: r"\.update\s*\(\s*\*\*\s*request\.(?:json\b|get_json\s*\(\s*\)|form\b|POST\b|data\b)",
        label: "Python .update(**request...) binds the whole request body with no field allowlist",
    },
    // JS: new Model(req.body/query/params) — bare whole-object argument.
    MassAssignPattern {
        regex: r"\bnew\s+[A-Z][A-Za-z0-9_]*\s*\(\s*req\.(?:body|query|params)\s*\)",
        label: "new Model(req.body/query/params) binds the whole request object with no field allowlist",
    },
    // JS: Object.assign(model, req.body/query/params) — bare second arg.
    MassAssignPattern {
        regex: r"Object\.assign\s*\(\s*[A-Za-z0-9_$.]+\s*,\s*req\.(?:body|query|params)\s*\)",
        label: "Object.assign(model, req.body/query/params) copies every request key with no field allowlist",
    },
    // JS: model.set(req.body/query/params) — bare whole-object argument.
    MassAssignPattern {
        regex: r"\.set\s*\(\s*req\.(?:body|query|params)\s*\)",
        label: "model.set(req.body/query/params) binds the whole request object with no field allowlist",
    },
    // JS: Model.create(req.body/query/params).
    MassAssignPattern {
        regex: r"\b[A-Z][A-Za-z0-9_]*\.create\s*\(\s*req\.(?:body|query|params)\s*\)",
        label: "Model.create(req.body/query/params) persists the whole request object with no field allowlist",
    },
    // JS: Model.update(req.body/query/params).
    MassAssignPattern {
        regex: r"\b[A-Z][A-Za-z0-9_]*\.update\s*\(\s*req\.(?:body|query|params)\s*\)",
        label: "Model.update(req.body/query/params) writes the whole request object with no field allowlist",
    },
    // JS: findByIdAndUpdate(id, req.body/query/params, ...).
    MassAssignPattern {
        regex: r"findByIdAndUpdate\s*\(\s*[^,()]+,\s*req\.(?:body|query|params)\s*[,)]",
        label: "findByIdAndUpdate(id, req.body/query/params) writes the whole request object with no field allowlist",
    },
    // Ruby: Model.new(params) — raw, unpermitted params.
    MassAssignPattern {
        regex: r"\b[A-Z][A-Za-z0-9_]*\.new\s*\(\s*params\s*\)",
        label: "Model.new(params) binds raw, unpermitted Rails params with no field allowlist",
    },
    // Ruby: .update(params) — raw, unpermitted params.
    MassAssignPattern {
        regex: r"\.update\s*\(\s*params\s*\)",
        label: "Model#update(params) binds raw, unpermitted Rails params with no field allowlist",
    },
    // Ruby: .update_attributes(params) — raw, unpermitted params.
    MassAssignPattern {
        regex: r"\.update_attributes\s*\(\s*params\s*\)",
        label: "Model#update_attributes(params) binds raw, unpermitted Rails params with no field allowlist",
    },
];

/// `CYBER-MASS-ASSIGN.1` — flags a WHOLE untrusted request object bound
/// directly into a model/entity with no field allowlist: capitalized
/// constructor/`.update()` calls fed `**request...` (Python), a
/// `setattr()` loop over a whole-request `.items()` accessor (Python),
/// `new Model`/`Object.assign`/`.set`/`.create`/`.update`/
/// `findByIdAndUpdate` fed a bare `req.body`/`req.query`/`req.params`
/// (JS/Node), and `Model.new`/`.update`/`.update_attributes` fed raw,
/// unpermitted `params` (Ruby).
pub struct MassAssignmentValidator {
    rule_id: RuleId,
    patterns: Vec<(Regex, &'static str)>,
    request_items_loop: Regex,
    setattr_call: Regex,
}

impl MassAssignmentValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut patterns = Vec::with_capacity(MASS_ASSIGN_PATTERNS_SRC.len());
        for entry in MASS_ASSIGN_PATTERNS_SRC {
            let regex = Regex::new(entry.regex)
                .map_err(|err| DecodeError::new("cyberskillsMassAssignPattern", err.to_string()))?;
            patterns.push((regex, entry.label));
        }
        // Python: a bare for-loop STATEMENT (the whole line, up to the
        // trailing `:`) that destructures every key/value out of one of the
        // five whole-request accessors — the hand-rolled equivalent of the
        // double-splat bind (SKILL.md's "Autobinding" concept). The `(?m)`
        // per-line `^...$` anchor is what keeps this from matching the same
        // token sequence embedded mid-line inside a filtered dict/set
        // comprehension, which always has trailing content (a trailing `if`
        // clause or closing brace) after `.items()` before end of line.
        let request_items_loop = Regex::new(
            r"(?m)^\s*for\s+\w+\s*,\s*\w+\s+in\s+request\.(?:json\b|get_json\s*\(\s*\)|form\b|POST\b|data\b)\.items\s*\(\s*\)\s*:\s*$",
        )
        .map_err(|err| DecodeError::new("cyberskillsMassAssignItemsLoop", err.to_string()))?;
        let setattr_call = Regex::new(r"\bsetattr\s*\(")
            .map_err(|err| DecodeError::new("cyberskillsMassAssignSetattrCall", err.to_string()))?;
        Ok(Self {
            rule_id: "CYBER-MASS-ASSIGN.1".parse()?,
            patterns,
            request_items_loop,
            setattr_call,
        })
    }
}

impl Validator for MassAssignmentValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        // Whole-file context: a setattr() call is only the mass-assignment
        // sink when paired with a for-loop statement that destructures a
        // whole-request accessor's .items() somewhere in the same scanned
        // unit (mirrors the `proto_pollution`/`weak_crypto` same-source
        // correlated-context idiom).
        let has_request_items_loop = self.request_items_loop.is_match(input.source);

        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);

            let mut matched_labels: Vec<&str> = Vec::new();
            for (regex, label) in &self.patterns {
                if regex.is_match(line) && !matched_labels.contains(label) {
                    matched_labels.push(*label);
                }
            }
            if !matched_labels.is_empty() {
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "Mass assignment: whole request object bound with no field allowlist"
                        .to_owned(),
                    detail: format!(
                        "Line binds an entire untrusted request object directly onto a \
                         model/entity with no field allowlist: {}. Fix: allowlist explicit \
                         fields (Rails `params.require(:model).permit(:field, ...)`, an \
                         explicit `{{ field: req.body.field }}` pick, or a validated DTO/schema \
                         object) instead of binding the whole request object.",
                        matched_labels.join(", ")
                    ),
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }

            if self.setattr_call.is_match(line) && has_request_items_loop {
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title:
                        "Mass assignment: setattr loop binds every request field with no allowlist"
                            .to_owned(),
                    detail: "Line calls setattr() in a file that also has a for-loop statement \
                             destructuring every key/value out of a whole-request accessor's \
                             .items() (request.json / request.get_json() / request.form / \
                             request.POST / request.data). This is the hand-rolled equivalent of \
                             Model(**request.json): every request key is written onto the object \
                             with no field allowlist. Fix: iterate only over an explicit \
                             allowlist of field names (or a pre-filtered dict built from one) \
                             before calling setattr()."
                        .to_owned(),
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::MassAssignmentValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_mass_assignment() -> Result<(), Box<dyn std::error::Error>> {
        let validator = MassAssignmentValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/web.mass-assignment/bad/vuln.py",
            "tests/fixtures/cyberskills/web.mass-assignment/good/safe.py",
        )?;
        Ok(())
    }
}
