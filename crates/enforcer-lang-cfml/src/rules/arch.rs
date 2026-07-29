//! CFML layered architecture + DI rules `CF-ARCH-1.1..5.1` /
//! `CFML-LAYER-1.1` (handlers/services/gateways/views must not cross
//! layers to run SQL directly) and `CF-DI-1.1`/`CFML-DI-1.1` (WireBox DI:
//! no manual `createObject`/`new` construction of collaborators).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::ids::{BuiltInCfmlRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{first_line_containing_any, FindingSpec};

const QUERY_MARKERS: &[ValidationSource<'static>] = &[
    ValidationSource::from_text("<cfquery"),
    ValidationSource::from_text("queryExecute("),
];
const VIEW_MARKERS: &[ValidationSource<'static>] = &[
    ValidationSource::from_text("<cfquery"),
    ValidationSource::from_text("queryExecute("),
    ValidationSource::from_text("createObject("),
];

/// `CF-ARCH-1.1` -- layered architecture: a handler/service/view must not
/// run SQL directly. A `Handler.cfc`/`Service.cfc` (except `*Gateway.cfc`)
/// containing a query marker is a violation; a `.cfm` view containing a
/// query marker or `createObject(` is also a violation.
#[derive(Debug)]
pub struct LayeredArchitectureValidator {
    rule_id: RuleId,
}

impl LayeredArchitectureValidator {
    /// Construct the layered-architecture validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::LayeredArchitecture.id(),
        })
    }
}

impl Validator for LayeredArchitectureValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if input.file.as_str().contains("Gateway.cfc") {
            return Vec::new();
        }
        if input.file.as_str().ends_with(".cfm") {
            let Some(line) = first_line_containing_any(input.source, VIEW_MARKERS) else {
                return Vec::new();
            };
            return vec![finding!(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    rule: BuiltInCfmlRule::LayeredArchitecture,
                },
                "A `.cfm` view must contain no query execution or `createObject` call -- \
                 delegate to a handler/service.",
                &input,
                line,
            )];
        }
        let Some(line) = first_line_containing_any(input.source, QUERY_MARKERS) else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                rule: BuiltInCfmlRule::LayeredArchitecture,
            },
            "A handler/service component must delegate persistence to a `*Gateway.cfc`; it \
             must never run `<cfquery>`/`queryExecute(...)` itself.",
            &input,
            line,
        )]
    }
}

const DI_MARKERS: &[ValidationSource<'static>] = &[
    ValidationSource::from_text("createObject(\"component\""),
    ValidationSource::from_text("createObject('component'"),
];

/// `CF-DI-1.1` -- WireBox DI: collaborators must be injected via `property
/// ... inject="...";`, never manually constructed with `createObject` or a
/// bare `new FooService()`.
#[derive(Debug)]
pub struct WireBoxDiValidator {
    rule_id: RuleId,
}

impl WireBoxDiValidator {
    /// Construct the WireBox dependency-injection validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::WireBoxDi.id(),
        })
    }
}

impl Validator for WireBoxDiValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        if let Some(line) = first_line_containing_any(input.source, DI_MARKERS) {
            return vec![finding!(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    rule: BuiltInCfmlRule::WireBoxDi,
                },
                "A collaborator is constructed via `createObject(\"component\", ...)` -- use \
                 WireBox injection (`property name=\"x\" inject=\"...\";`) instead.",
                &input,
                line,
            )];
        }
        for (line_number, line) in (1u32..).zip(input.source.as_str().lines()) {
            let trimmed = line.trim_start();
            let is_manual_new = (trimmed.contains("= new ") || trimmed.contains("=new "))
                && trimmed.trim_end().ends_with("();");
            if is_manual_new {
                return vec![finding!(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        rule: BuiltInCfmlRule::WireBoxDi,
                    },
                    "A collaborator is constructed via a bare `new FooService()` -- use WireBox \
                     injection (`property name=\"x\" inject=\"...\";`) instead.",
                    &input,
                    line_number,
                )];
            }
        }
        Vec::new()
    }
}

/// `CF-ARCH-3.1` (scored) -- services must not read `rc`/`form`/`url`
/// scopes directly; the caller should pass a typed argument instead.
#[derive(Debug)]
pub struct ServiceScopeReadValidator {
    rule_id: RuleId,
}

impl ServiceScopeReadValidator {
    /// Construct the service-scope read validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::ServiceScopeRead.id(),
        })
    }
}

const RC_FORM_URL_MARKERS: &[ValidationSource<'static>] = &[
    ValidationSource::from_text("rc."),
    ValidationSource::from_text("form."),
    ValidationSource::from_text("url."),
];

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
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInCfmlRule::ServiceScopeRead,
            },
            "A `*Service.cfc` reads the `rc`/`form`/`url` request scope directly -- pass the \
             value in as a typed argument instead.",
            &input,
            line,
        )]
    }
}

/// `CF-DI-1.2` / `CF-DI-1.3` (scored) -- lifecycle scope: no
/// `application`-scope service lookup; services/gateways should be
/// singleton-injected.
#[derive(Debug)]
pub struct ApplicationScopeLookupValidator {
    rule_id: RuleId,
}

impl ApplicationScopeLookupValidator {
    /// Construct the application-scope lookup validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::ApplicationScopeLookup.id(),
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
                ValidationSource::from_text("application.orderService"),
                ValidationSource::from_text("application.wirebox.getInstance"),
            ],
        ) else {
            return Vec::new();
        };
        vec![finding!(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInCfmlRule::ApplicationScopeLookup,
            },
            "A service/gateway is looked up via the `application` scope instead of an injected \
             singleton -- inject it via `property ... inject=\"...\";`.",
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
