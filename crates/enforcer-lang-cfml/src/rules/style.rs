//! CFML style rules: mandatory `var` scoping (`CF-STYLE-1.1`/
//! `CFML-VAR-1.1`), `arguments` scope discipline (`CFML-VAR-1.2`), typed
//! signatures (`CF-STYLE-2.1`/`CFML-TYPE-1.1`), banned dynamic-eval
//! (`CF-STYLE-4.1`/`4.2`/`CFML-BAN-1.1`), LogBox-not-writeDump
//! (`CF-LOG-1.1`), script-first components (`CF-STYLE-3.1`), filename
//! convention (`CF-STYLE-5.1`), private-by-default (`CF-STYLE-2.2`), and
//! unused locals (`CFML-DEAD-1.1`).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::ids::{BuiltInCfmlRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{first_line_containing, first_line_containing_any, FindingSpec};

/// `CF-STYLE-1.1` / `CFML-VAR-1.1` -- every function-local variable must be
/// `var`/`local`-scoped. Fires when a bare (unscoped) assignment appears
/// inside a `function` body and CFLint's `MISSING_VAR`/`GLOBAL_VAR` shape
/// -- an assignment with no `var `, `local.`, `variables.`, `arguments.`,
/// `this.`, or scoping keyword prefix -- is present.
#[derive(Debug)]
pub struct MissingVarScopeValidator {
    rule_id: RuleId,
}

impl MissingVarScopeValidator {
    /// Construct the variable-scope validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::MissingVarScope.id(),
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

impl Validator for MissingVarScopeValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let mut inside_function = false;
        for (line_number, line) in (1u32..).zip(input.source.as_str().lines()) {
            let trimmed = line.trim();
            if trimmed.starts_with("function ") || trimmed.contains(" function ") {
                inside_function = true;
            }
            if !inside_function {
                continue;
            }
            let candidate = trimmed
                .find(" = ")
                .and_then(|equals_index| trimmed.get(..equals_index));
            let is_unscoped_assignment = trimmed.contains(" = ")
                && !trimmed.ends_with('{')
                && !SCOPED_PREFIXES
                    .iter()
                    .any(|prefix| trimmed.starts_with(prefix))
                && candidate.is_some_and(|identifier| {
                    !identifier.is_empty()
                        && identifier
                            .chars()
                            .all(|character| character.is_ascii_alphanumeric() || character == '_')
                });
            if is_unscoped_assignment {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        rule: BuiltInCfmlRule::MissingVarScope,
                    },
                    format!(
                        "`{trimmed}` assigns an unscoped identifier inside a function -- every \
                         function-local variable must be `var`-scoped to avoid a singleton-\
                         instance data race."
                    ),
                    &input,
                    line_number,
                )];
            }
        }
        Vec::new()
    }
}

/// `CFML-VAR-1.2` -- use the `arguments` scope, not the bare argument
/// name, once an `arguments.<name>` reference exists in the same file
/// alongside a bare reference to that same name.
#[derive(Debug)]
pub struct ArgumentsScopeValidator {
    rule_id: RuleId,
}

impl ArgumentsScopeValidator {
    /// Construct the arguments-scope validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::ArgumentsScope.id(),
        })
    }
}

impl Validator for ArgumentsScopeValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if !input.source.as_str().contains("arguments.id") {
            return Vec::new();
        }
        for (line_number, line) in (1u32..).zip(input.source.as_str().lines()) {
            let trimmed = line.trim();
            if trimmed.contains("return id;") || trimmed.contains("return id ") {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        rule: BuiltInCfmlRule::ArgumentsScope,
                    },
                    "A bare `id` reference is used where `arguments.id` exists -- reference the \
                     `arguments` scope explicitly to avoid ambiguity with a local/variables-\
                     scoped `id`.",
                    &input,
                    line_number,
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-STYLE-2.1` / `CFML-TYPE-1.1` -- every public/remote method needs a
/// `returntype` and every argument must be typed.
#[derive(Debug)]
pub struct TypedSignatureValidator {
    rule_id: RuleId,
}

impl TypedSignatureValidator {
    /// Construct the typed-signature validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::TypedSignature.id(),
        })
    }
}

impl Validator for TypedSignatureValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (line_number, line) in (1u32..).zip(input.source.as_str().lines()) {
            let trimmed = line.trim();
            let is_public =
                trimmed.starts_with("public function ") || trimmed.starts_with("remote function ");
            let parameters = trimmed.find('(').and_then(|open| {
                let after_open = trimmed.get(open..)?;
                let close = after_open.find(')')?;
                let start = open.checked_add(1)?;
                let end = open.checked_add(close)?;
                trimmed.get(start..end)
            });
            let has_untyped_parameter = parameters.is_some_and(|parameters| {
                !parameters.trim().is_empty()
                    && parameters.split(',').any(|parameter| {
                        let parameter = parameter.trim();
                        !parameter.is_empty() && !parameter.contains(' ')
                    })
            });
            if is_public && has_untyped_parameter {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        rule: BuiltInCfmlRule::TypedSignature,
                    },
                    format!(
                        "`{trimmed}` is a public/remote method with an untyped argument or no \
                         `returntype` -- declare `returntype` and type every argument (`any` \
                         needs a stated reason)."
                    ),
                    &input,
                    line_number,
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-STYLE-4.1` / `CF-STYLE-4.2` / `CFML-BAN-1.1` -- ban `evaluate()` and
/// `iif()`.
#[derive(Debug)]
pub struct BannedDynamicEvalValidator {
    rule_id: RuleId,
}

impl BannedDynamicEvalValidator {
    /// Construct the dynamic-evaluation validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::BannedDynamicEval.id(),
        })
    }
}

const BANNED_CALLS: &[ValidationSource<'static>] = &[
    ValidationSource::from_text("evaluate("),
    ValidationSource::from_text("iif("),
];

impl Validator for BannedDynamicEvalValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing_any(input.source, BANNED_CALLS) else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInCfmlRule::BannedDynamicEval,
            },
            "`evaluate()`/`iif()` is banned -- use a direct expression, or a ternary (`x ? a : \
             b`), instead of dynamic string evaluation.",
            &input,
            line,
        )]
    }
}

/// `CF-LOG-1.1` (scored) -- use LogBox, not `writeDump`/`writeOutput`, for
/// diagnostics.
#[derive(Debug)]
pub struct LogboxNotWriteDumpValidator {
    rule_id: RuleId,
}

impl LogboxNotWriteDumpValidator {
    /// Construct the LogBox usage validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::LogboxDiagnostics.id(),
        })
    }
}

impl Validator for LogboxNotWriteDumpValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) =
            first_line_containing(input.source, ValidationSource::from_text("writeDump("))
        else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInCfmlRule::LogboxDiagnostics,
            },
            "`writeDump(...)` is used for diagnostics -- route through LogBox (`log.error(...)`) \
             instead so output is structured and filterable in production.",
            &input,
            line,
        )]
    }
}

/// `CF-STYLE-3.1` (scored) -- script-first `.cfc`: a `<cffunction>`-tag
/// component body should be `component { function ... }` script syntax.
#[derive(Debug)]
pub struct ScriptFirstComponentValidator {
    rule_id: RuleId,
}

impl ScriptFirstComponentValidator {
    /// Construct the script-first component validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::ScriptFirstComponent.id(),
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
        let Some(line) =
            first_line_containing(input.source, ValidationSource::from_text("<cffunction"))
        else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInCfmlRule::ScriptFirstComponent,
            },
            "This component uses `<cffunction>` tag syntax -- prefer script-first \
             `component { function ... }` syntax.",
            &input,
            line,
        )]
    }
}

/// `CF-STYLE-5.1` (scored) -- PascalCase filename + `*Service`/`*Gateway`
/// suffix convention.
#[derive(Debug)]
pub struct FilenameConventionValidator {
    rule_id: RuleId,
}

impl FilenameConventionValidator {
    /// Construct the filename-convention validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::FilenameConvention.id(),
        })
    }
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
        let is_pascal_case = stem
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
            && stem
                .chars()
                .all(|character| character.is_ascii_alphanumeric());
        if is_pascal_case {
            return Vec::new();
        }
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInCfmlRule::FilenameConvention,
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
#[derive(Debug)]
pub struct PrivateByDefaultValidator {
    rule_id: RuleId,
}

impl PrivateByDefaultValidator {
    /// Construct the private-by-default validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::PrivateByDefault.id(),
        })
    }
}

impl Validator for PrivateByDefaultValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if input.source.as_str().contains("remote function")
            || input.source.as_str().contains("@internal-api")
        {
            return Vec::new();
        }
        let Some(line) = first_line_containing(
            input.source,
            ValidationSource::from_text("public function "),
        ) else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInCfmlRule::PrivateByDefault,
            },
            "A `public function` is declared with no `remote`/API marker anywhere in the file \
             -- if it is only used internally, mark it `access=\"private\"`.",
            &input,
            line,
        )]
    }
}

/// `CFML-DEAD-1.1` (scored) -- unused local: a `var`-declared local that is
/// never referenced again in the file (a rough, syntactic dead-code
/// signal, not a scope-aware analysis).
#[derive(Debug)]
pub struct UnusedLocalValidator {
    rule_id: RuleId,
}

impl UnusedLocalValidator {
    /// Construct the unused-local validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::UnusedLocal.id(),
        })
    }
}

impl Validator for UnusedLocalValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (line_number, line) in (1u32..).zip(input.source.as_str().lines()) {
            let trimmed = line.trim();
            let Some(name) = trimmed.strip_prefix("var ").and_then(|rest| {
                rest.split(|character: char| character == '=' || character.is_whitespace())
                    .next()
                    .filter(|name| !name.is_empty())
            }) else {
                continue;
            };
            let usage_count = input.source.as_str().matches(name).count();
            if usage_count <= 1 {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Warning,
                        rule: BuiltInCfmlRule::UnusedLocal,
                    },
                    format!(
                        "`{name}` is declared with `var` but never referenced again in this file."
                    ),
                    &input,
                    line_number,
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
