//! CFML layered architecture + DI rules `CF-ARCH-1.1..5.1` /
//! `CFML-LAYER-1.1` (handlers/services/gateways/views must not cross
//! layers to run SQL directly) and `CF-DI-1.1`/`CFML-DI-1.1` (WireBox DI:
//! no manual `createObject`/`new` construction of collaborators).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, first_line_containing_any, FindingSpec};

const QUERY_MARKERS: &[&str] = &["<cfquery", "queryExecute("];

/// `CF-ARCH-1.1` -- layered architecture: a handler/service/view must not
/// run SQL directly. A `Handler.cfc`/`Service.cfc` (except `*Gateway.cfc`)
/// containing a query marker is a violation; a `.cfm` view containing a
/// query marker or `createObject(` is also a violation.
pub struct LayeredArchitectureValidator {
    rule_id: RuleId,
}

impl LayeredArchitectureValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-ARCH-1.1".parse()?,
        })
    }
}

fn is_gateway_path(path: &str) -> bool {
    path.contains("Gateway.cfc")
}

fn is_view_path(path: &str) -> bool {
    path.ends_with(".cfm")
}

impl Validator for LayeredArchitectureValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let path = input.file.as_str();
        if is_gateway_path(path) {
            return Vec::new();
        }
        if is_view_path(path) {
            let markers: Vec<&str> = QUERY_MARKERS
                .iter()
                .chain(["createObject("].iter())
                .copied()
                .collect();
            let Some(line) = first_line_containing_any(input.source, &markers) else {
                return Vec::new();
            };
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    title: "layered architecture: view runs a query or createObject directly",
                },
                "A `.cfm` view must contain no query execution or `createObject` call -- \
                 delegate to a handler/service."
                    .to_owned(),
                &input,
                line,
            )];
        }
        let Some(line) = first_line_containing_any(input.source, QUERY_MARKERS) else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "layered architecture: handler/service runs SQL directly",
            },
            "A handler/service component must delegate persistence to a `*Gateway.cfc`; it \
             must never run `<cfquery>`/`queryExecute(...)` itself."
                .to_owned(),
            &input,
            line,
        )]
    }
}

const DI_MARKERS: &[&str] = &["createObject(\"component\"", "createObject('component'"];

/// `CF-DI-1.1` -- WireBox DI: collaborators must be injected via `property
/// ... inject="...";`, never manually constructed with `createObject` or a
/// bare `new FooService()`.
pub struct WireBoxDiValidator {
    rule_id: RuleId,
}

impl WireBoxDiValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-DI-1.1".parse()?,
        })
    }
}

fn is_manual_new_construction(line: &str) -> bool {
    let trimmed = line.trim_start();
    (trimmed.contains("= new ") || trimmed.contains("=new ")) && trimmed.trim_end().ends_with("();")
}

impl Validator for WireBoxDiValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if let Some(line) = first_line_containing_any(input.source, DI_MARKERS) {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    title: "DI: collaborator built via createObject instead of WireBox injection",
                },
                "A collaborator is constructed via `createObject(\"component\", ...)` -- use \
                 WireBox injection (`property name=\"x\" inject=\"...\";`) instead."
                    .to_owned(),
                &input,
                line,
            )];
        }
        for (idx, line) in input.source.lines().enumerate() {
            if is_manual_new_construction(line) {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "DI: collaborator built via new() instead of WireBox injection",
                    },
                    "A collaborator is constructed via a bare `new FooService()` -- use WireBox \
                     injection (`property name=\"x\" inject=\"...\";`) instead."
                        .to_owned(),
                    &input,
                    (idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-ARCH-3.1` (scored) -- services must not read `rc`/`form`/`url`
/// scopes directly; the caller should pass a typed argument instead.
pub struct ServiceScopeReadValidator {
    rule_id: RuleId,
}

impl ServiceScopeReadValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-ARCH-3.1".parse()?,
        })
    }
}

const RC_FORM_URL_MARKERS: &[&str] = &["rc.", "form.", "url."];

impl Validator for ServiceScopeReadValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let path = input.file.as_str();
        if !path.contains("Service.cfc") {
            return Vec::new();
        }
        let Some(line) = first_line_containing_any(input.source, RC_FORM_URL_MARKERS) else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "architecture: service reads rc/form/url scope directly (scored)",
            },
            "A `*Service.cfc` reads the `rc`/`form`/`url` request scope directly -- pass the \
             value in as a typed argument instead."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// `CF-DI-1.2` / `CF-DI-1.3` (scored) -- lifecycle scope: no
/// `application`-scope service lookup; services/gateways should be
/// singleton-injected.
pub struct ApplicationScopeLookupValidator {
    rule_id: RuleId,
}

impl ApplicationScopeLookupValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CF-DI-1.2".parse()?,
        })
    }
}

impl Validator for ApplicationScopeLookupValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let Some(line) = first_line_containing_any(
            input.source,
            &[
                "application.orderService",
                "application.wirebox.getInstance",
            ],
        ) else {
            return Vec::new();
        };
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "DI: application-scope service lookup instead of injection (scored)",
            },
            "A service/gateway is looked up via the `application` scope instead of an injected \
             singleton -- inject it via `property ... inject=\"...\";`."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(LayeredArchitectureValidator::new()?),
        Box::new(WireBoxDiValidator::new()?),
        Box::new(ServiceScopeReadValidator::new()?),
        Box::new(ApplicationScopeLookupValidator::new()?),
    ])
}
