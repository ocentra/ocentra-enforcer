//! CFML security rules: SQLi (`CF-SEC-1.1`/`CFML-SQL-1.1`, CWE-89), XSS
//! (`CF-SEC-2.1`, CWE-79), hardcoded secrets (`CF-SEC-4.1`, CWE-798), and
//! information disclosure (`CF-SEC-3.1`, CWE-209).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::ids::{BuiltInCfmlRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{first_line_containing, first_line_containing_any, FindingSpec};

/// `CF-SEC-1.1` / `CFML-SQL-1.1` -- SQLi: a dynamic SQL string built with
/// an interpolated `#...#` value inside `queryExecute(`/`<cfquery>` must
/// use `cfqueryparam`/a param struct instead.
#[derive(Debug)]
pub struct SqlInjectionValidator {
    rule_id: RuleId,
}

impl SqlInjectionValidator {
    /// Construct the SQL-injection validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::SqlInjection.id(),
        })
    }
}

impl Validator for SqlInjectionValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (line_number, line) in (1u32..).zip(input.source.as_str().lines()) {
            let Some(after) = line
                .find("queryExecute(")
                .and_then(|start| line.get(start..))
            else {
                continue;
            };
            let has_interpolation = after.contains('#');
            let has_param_struct =
                after.contains("{ ") || after.contains("{\n") || after.contains("{id");
            if has_interpolation && !has_param_struct {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        rule: BuiltInCfmlRule::SqlInjection,
                    },
                    "`queryExecute(...)` interpolates a `#value#` directly into the SQL string \
                     -- bind it via a param struct (or `<cfqueryparam>`) instead.",
                    &input,
                    line_number,
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-SEC-2.1` -- XSS: raw `<cfoutput>#value#</cfoutput>` of an untrusted
/// value must be encoded via `encodeForHTML(...)`.
#[derive(Debug)]
pub struct XssOutputValidator {
    rule_id: RuleId,
}

impl XssOutputValidator {
    /// Construct the output-encoding validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::XssOutput.id(),
        })
    }
}

impl Validator for XssOutputValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (line_number, line) in (1u32..).zip(input.source.as_str().lines()) {
            if line.contains("<cfoutput>#")
                && line.contains('#')
                && !line.contains("encodeForHTML(")
            {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        rule: BuiltInCfmlRule::XssOutput,
                    },
                    "A `<cfoutput>#value#</cfoutput>` writes an untrusted value without \
                     encoding -- wrap it in `encodeForHTML(...)`.",
                    &input,
                    line_number,
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-SEC-4.1` -- hardcoded secret literal (CWE-798): a `variables.` (or
/// bare local) assignment of an `apiKey`/`secret`/`password` field to a
/// string literal is a violation.
#[derive(Debug)]
pub struct EmbeddedLiteralValidator {
    rule_id: RuleId,
}

impl EmbeddedLiteralValidator {
    /// Construct the embedded-secret validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::HardcodedSecret.id(),
        })
    }
}

const SECRET_FIELD_ASSIGNMENTS: &[ValidationSource<'static>] = &[
    ValidationSource::from_text("variables.apiKey = \""),
    ValidationSource::from_text("variables.apiKey = '"),
    ValidationSource::from_text("variables.secret = \""),
    ValidationSource::from_text("variables.secret = '"),
    ValidationSource::from_text("variables.password = \""),
    ValidationSource::from_text("variables.password = '"),
];

impl Validator for EmbeddedLiteralValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing_any(input.source, SECRET_FIELD_ASSIGNMENTS) else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInCfmlRule::HardcodedSecret,
            },
            "A secret/API-key/password field is assigned a hardcoded string literal -- read it \
             from an environment variable or a config/secret store instead.",
            &input,
            line,
        )]
    }
}

/// `CF-SEC-3.1` -- information disclosure (CWE-209): `cfcatch.detail`/
/// `.tagContext`/`<cfdump>` must never reach the client/view.
#[derive(Debug)]
pub struct InfoDisclosureValidator {
    rule_id: RuleId,
}

impl InfoDisclosureValidator {
    /// Construct the information-disclosure validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::InformationDisclosure.id(),
        })
    }
}

const DISCLOSURE_MARKERS: &[ValidationSource<'static>] = &[
    ValidationSource::from_text("cfcatch.tagContext"),
    ValidationSource::from_text("cfcatch.detail"),
    ValidationSource::from_text("<cfdump"),
];

impl Validator for InfoDisclosureValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(matched_marker) = DISCLOSURE_MARKERS
            .iter()
            .find(|marker| input.source.as_str().contains(marker.as_str()))
        else {
            return Vec::new();
        };
        let Some(line) = first_line_containing(input.source, *matched_marker) else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInCfmlRule::InformationDisclosure,
            },
            "`cfcatch.detail`/`cfcatch.tagContext`/`<cfdump>` output is returned to the \
             caller/rendered in a view -- log the detail server-side and return a generic \
             client-facing message.",
            &input,
            line,
        )]
    }
}

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(SqlInjectionValidator::new()?),
        Box::new(XssOutputValidator::new()?),
        Box::new(EmbeddedLiteralValidator::new()?),
        Box::new(InfoDisclosureValidator::new()?),
    ])
}
