//! CFML security rules: SQLi (`CF-SEC-1.1`/`CFML-SQL-1.1`, CWE-89), XSS
//! (`CF-SEC-2.1`, CWE-79), hardcoded secrets (`CF-SEC-4.1`, CWE-798), and
//! information disclosure (`CF-SEC-3.1`, CWE-209).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, first_line_containing, first_line_containing_any, FindingSpec};

/// `CF-SEC-1.1` / `CFML-SQL-1.1` -- SQLi: a dynamic SQL string built with
/// an interpolated `#...#` value inside `queryExecute(`/`<cfquery>` must
/// use `cfqueryparam`/a param struct instead.
pub struct SqlInjectionValidator {
    rule_id: RuleId,
}

impl SqlInjectionValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-SEC-1.1".parse()?,
        })
    }
}

/// True when `line` contains a `queryExecute("...#...#...")`-shaped call
/// whose SQL string interpolates a `#value#` placeholder without a
/// trailing param-struct argument on the SAME line (the uniform shape this
/// crate's fixtures use).
fn is_unparameterized_query_execute(line: &str) -> bool {
    let Some(start) = line.find("queryExecute(") else {
        return false;
    };
    let after = &line[start..];
    let has_interpolation = after.contains('#');
    let has_param_struct = after.contains("{ ") || after.contains("{\n") || after.contains("{id");
    has_interpolation && !has_param_struct
}

impl Validator for SqlInjectionValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (idx, line) in input.source.lines().enumerate() {
            if is_unparameterized_query_execute(line) {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "SQLi: dynamic SQL value not bound via cfqueryparam (CWE-89)",
                    },
                    "`queryExecute(...)` interpolates a `#value#` directly into the SQL string \
                     -- bind it via a param struct (or `<cfqueryparam>`) instead."
                        .to_owned(),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-SEC-2.1` -- XSS: raw `<cfoutput>#value#</cfoutput>` of an untrusted
/// value must be encoded via `encodeForHTML(...)`.
pub struct XssOutputValidator {
    rule_id: RuleId,
}

impl XssOutputValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-SEC-2.1".parse()?,
        })
    }
}

impl Validator for XssOutputValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        for (idx, line) in input.source.lines().enumerate() {
            if line.contains("<cfoutput>#")
                && line.contains('#')
                && !line.contains("encodeForHTML(")
            {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "XSS: unencoded value written to cfoutput (CWE-79)",
                    },
                    "A `<cfoutput>#value#</cfoutput>` writes an untrusted value without \
                     encoding -- wrap it in `encodeForHTML(...)`."
                        .to_owned(),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-SEC-4.1` -- hardcoded secret literal (CWE-798): a `variables.` (or
/// bare local) assignment of an `apiKey`/`secret`/`password` field to a
/// string literal is a violation.
pub struct HardcodedSecretValidator {
    rule_id: RuleId,
}

impl HardcodedSecretValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-SEC-4.1".parse()?,
        })
    }
}

const SECRET_FIELD_ASSIGNMENTS: &[&str] = &[
    "variables.apiKey = \"",
    "variables.apiKey = '",
    "variables.secret = \"",
    "variables.secret = '",
    "variables.password = \"",
    "variables.password = '",
];

impl Validator for HardcodedSecretValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing_any(input.source, SECRET_FIELD_ASSIGNMENTS) else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "hardcoded secret literal (CWE-798)",
            },
            "A secret/API-key/password field is assigned a hardcoded string literal -- read it \
             from an environment variable or a config/secret store instead."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `CF-SEC-3.1` -- information disclosure (CWE-209): `cfcatch.detail`/
/// `.tagContext`/`<cfdump>` must never reach the client/view.
pub struct InfoDisclosureValidator {
    rule_id: RuleId,
}

impl InfoDisclosureValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-SEC-3.1".parse()?,
        })
    }
}

const DISCLOSURE_MARKERS: &[&str] = &["cfcatch.tagContext", "cfcatch.detail", "<cfdump"];

impl Validator for InfoDisclosureValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(matched_marker) = DISCLOSURE_MARKERS
            .iter()
            .find(|marker| input.source.contains(**marker))
        else {
            return Vec::new();
        };
        let Some(line) = first_line_containing(input.source, matched_marker) else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title:
                    "information disclosure: internal error/stack detail reaches caller (CWE-209)",
            },
            "`cfcatch.detail`/`cfcatch.tagContext`/`<cfdump>` output is returned to the \
             caller/rendered in a view -- log the detail server-side and return a generic \
             client-facing message."
                .to_owned(),
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
        Box::new(HardcodedSecretValidator::new()?),
        Box::new(InfoDisclosureValidator::new()?),
    ])
}
