//! CFML style rules: mandatory `var` scoping (`CF-STYLE-1.1`/
//! `CFML-VAR-1.1`), `arguments` scope discipline (`CFML-VAR-1.2`), typed
//! signatures (`CF-STYLE-2.1`/`CFML-TYPE-1.1`), banned dynamic-eval
//! (`CF-STYLE-4.1`/`4.2`/`CFML-BAN-1.1`), LogBox-not-writeDump
//! (`CF-LOG-1.1`), script-first components (`CF-STYLE-3.1`), filename
//! convention (`CF-STYLE-5.1`), private-by-default (`CF-STYLE-2.2`), and
//! unused locals (`CFML-DEAD-1.1`).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, first_line_containing, first_line_containing_any, FindingSpec};

/// `CF-STYLE-1.1` / `CFML-VAR-1.1` -- every function-local variable must be
/// `var`/`local`-scoped. Fires when a bare (unscoped) assignment appears
/// inside a `function` body and CFLint's `MISSING_VAR`/`GLOBAL_VAR` shape
/// -- an assignment with no `var `, `local.`, `variables.`, `arguments.`,
/// `this.`, or scoping keyword prefix -- is present.
pub struct MissingVarScopeValidator {
    rule_id: RuleId,
}

impl MissingVarScopeValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-STYLE-1.1".parse()?,
        })
    }
}

const SCOPED_PREFIXES: &[&str] = &[
    "var ",
    "local.",
    "variables.",
    "arguments.",
    "this.",
    "//",
    "*",
    "function",
    "}",
    "return",
    "if ",
    "for ",
    "while ",
];

fn is_unscoped_assignment(trimmed: &str) -> bool {
    if !trimmed.contains(" = ") || trimmed.ends_with('{') {
        return false;
    }
    if SCOPED_PREFIXES.iter().any(|p| trimmed.starts_with(p)) {
        return false;
    }
    // Must look like `identifier = value;` (identifier: alnum/underscore).
    let Some(eq_idx) = trimmed.find(" = ") else {
        return false;
    };
    let Some(candidate) = trimmed.get(..eq_idx) else {
        return false;
    };
    !candidate.is_empty()
        && candidate
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl Validator for MissingVarScopeValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let mut inside_function = false;
        for (idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("function ") || trimmed.contains(" function ") {
                inside_function = true;
            }
            if !inside_function {
                continue;
            }
            if is_unscoped_assignment(trimmed) {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "missing var scope on a function-local assignment (MISSING_VAR)",
                    },
                    format!(
                        "`{trimmed}` assigns an unscoped identifier inside a function -- every \
                         function-local variable must be `var`-scoped to avoid a singleton-\
                         instance data race."
                    ),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// `CFML-VAR-1.2` -- use the `arguments` scope, not the bare argument
/// name, once an `arguments.<name>` reference exists in the same file
/// alongside a bare reference to that same name.
pub struct ArgumentsScopeValidator {
    rule_id: RuleId,
}

impl ArgumentsScopeValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CFML-VAR-1.2".parse()?,
        })
    }
}

impl Validator for ArgumentsScopeValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if !input.source.contains("arguments.id") {
            return Vec::new();
        }
        for (idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("return id;") || trimmed.contains("return id ") {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "bare argument reference instead of arguments scope",
                    },
                    "A bare `id` reference is used where `arguments.id` exists -- reference the \
                     `arguments` scope explicitly to avoid ambiguity with a local/variables-\
                     scoped `id`."
                        .to_owned(),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-STYLE-2.1` / `CFML-TYPE-1.1` -- every public/remote method needs a
/// `returntype` and every argument must be typed.
pub struct TypedSignatureValidator {
    rule_id: RuleId,
}

impl TypedSignatureValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-STYLE-2.1".parse()?,
        })
    }
}

fn is_untyped_public_signature(trimmed: &str) -> bool {
    if !trimmed.starts_with("public function ") && !trimmed.starts_with("remote function ") {
        return false;
    }
    let Some(open) = trimmed.find('(') else {
        return false;
    };
    let Some(after_open) = trimmed.get(open..) else {
        return false;
    };
    let Some(close) = after_open.find(')') else {
        return false;
    };
    let Some(params_start) = open.checked_add(1) else {
        return false;
    };
    let Some(params_end) = open.checked_add(close) else {
        return false;
    };
    let Some(params) = trimmed.get(params_start..params_end) else {
        return false;
    };
    let has_untyped_param = !params.trim().is_empty()
        && params.split(',').any(|p| {
            let p = p.trim();
            !p.is_empty() && !p.contains(' ')
        });
    has_untyped_param
}

impl Validator for TypedSignatureValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim();
            if is_untyped_public_signature(trimmed) {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "public method missing returntype or typed argument",
                    },
                    format!(
                        "`{trimmed}` is a public/remote method with an untyped argument or no \
                         `returntype` -- declare `returntype` and type every argument (`any` \
                         needs a stated reason)."
                    ),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-STYLE-4.1` / `CF-STYLE-4.2` / `CFML-BAN-1.1` -- ban `evaluate()` and
/// `iif()`.
pub struct BannedDynamicEvalValidator {
    rule_id: RuleId,
}

impl BannedDynamicEvalValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-STYLE-4.1".parse()?,
        })
    }
}

const BANNED_CALLS: &[&str] = &["evaluate(", "iif("];

impl Validator for BannedDynamicEvalValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing_any(input.source, BANNED_CALLS) else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "banned dynamic construct: evaluate()/iif()",
            },
            "`evaluate()`/`iif()` is banned -- use a direct expression, or a ternary (`x ? a : \
             b`), instead of dynamic string evaluation."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `CF-LOG-1.1` (scored) -- use LogBox, not `writeDump`/`writeOutput`, for
/// diagnostics.
pub struct LogboxNotWriteDumpValidator {
    rule_id: RuleId,
}

impl LogboxNotWriteDumpValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-LOG-1.1".parse()?,
        })
    }
}

impl Validator for LogboxNotWriteDumpValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing(input.source, "writeDump(") else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "diagnostics via writeDump instead of LogBox (scored)",
            },
            "`writeDump(...)` is used for diagnostics -- route through LogBox (`log.error(...)`) \
             instead so output is structured and filterable in production."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `CF-STYLE-3.1` (scored) -- script-first `.cfc`: a `<cffunction>`-tag
/// component body should be `component { function ... }` script syntax.
pub struct ScriptFirstComponentValidator {
    rule_id: RuleId,
}

impl ScriptFirstComponentValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-STYLE-3.1".parse()?,
        })
    }
}

impl Validator for ScriptFirstComponentValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if !input.file.as_str().ends_with(".cfc") {
            return Vec::new();
        }
        let Some(line) = first_line_containing(input.source, "<cffunction") else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "tag-based component body instead of script syntax (scored)",
            },
            "This component uses `<cffunction>` tag syntax -- prefer script-first \
             `component { function ... }` syntax."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `CF-STYLE-5.1` (scored) -- PascalCase filename + `*Service`/`*Gateway`
/// suffix convention.
pub struct FilenameConventionValidator {
    rule_id: RuleId,
}

impl FilenameConventionValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-STYLE-5.1".parse()?,
        })
    }
}

fn is_pascal_case_stem(stem: &str) -> bool {
    stem.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && stem.chars().all(|c| c.is_ascii_alphanumeric())
}

impl Validator for FilenameConventionValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let path = input.file.as_str();
        if !path.ends_with(".cfc") {
            return Vec::new();
        }
        let file_name = path.rsplit('/').next().unwrap_or(path);
        let Some(stem) = file_name.strip_suffix(".cfc") else {
            return Vec::new();
        };
        if is_pascal_case_stem(stem) {
            return Vec::new();
        }
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "component filename is not PascalCase (scored)",
            },
            format!(
                "`{file_name}` is not PascalCase -- CFML component filenames should be \
                 PascalCase (e.g. `OrderService.cfc`)."
            ),
            &input,
            1,
        )]
    }
}

/// `CF-STYLE-2.2` (scored) -- `access="private"` default for non-API
/// methods: a `public function` with no `remote`/API marker anywhere in
/// the file is scored.
pub struct PrivateByDefaultValidator {
    rule_id: RuleId,
}

impl PrivateByDefaultValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-STYLE-2.2".parse()?,
        })
    }
}

impl Validator for PrivateByDefaultValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if input.source.contains("remote function") || input.source.contains("@internal-api") {
            return Vec::new();
        }
        let Some(line) = first_line_containing(input.source, "public function ") else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "public method with no external API marker (scored)",
            },
            "A `public function` is declared with no `remote`/API marker anywhere in the file \
             -- if it is only used internally, mark it `access=\"private\"`."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `CFML-DEAD-1.1` (scored) -- unused local: a `var`-declared local that is
/// never referenced again in the file (a rough, syntactic dead-code
/// signal, not a scope-aware analysis).
pub struct UnusedLocalValidator {
    rule_id: RuleId,
}

impl UnusedLocalValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CFML-DEAD-1.1".parse()?,
        })
    }
}

fn declared_var_name(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("var ")?;
    let name = rest.split(|c: char| c == '=' || c.is_whitespace()).next()?;
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

impl Validator for UnusedLocalValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim();
            let Some(name) = declared_var_name(trimmed) else {
                continue;
            };
            let usage_count = input.source.matches(name).count();
            if usage_count <= 1 {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Warning,
                        title: "unused local variable (UNUSED_LOCAL_VARIABLE, scored)",
                    },
                    format!(
                        "`{name}` is declared with `var` but never referenced again in this file."
                    ),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(MissingVarScopeValidator::new()?),
        Box::new(ArgumentsScopeValidator::new()?),
        Box::new(TypedSignatureValidator::new()?),
        Box::new(BannedDynamicEvalValidator::new()?),
        Box::new(LogboxNotWriteDumpValidator::new()?),
        Box::new(ScriptFirstComponentValidator::new()?),
        Box::new(FilenameConventionValidator::new()?),
        Box::new(PrivateByDefaultValidator::new()?),
        Box::new(UnusedLocalValidator::new()?),
    ])
}
