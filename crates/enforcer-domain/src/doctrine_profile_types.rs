//! Closed domain values for shape-driven doctrine profiles.
//!
//! This module owns validated doctrine identities and resolution semantics.
//! JSON and other wire representations remain in `enforcer-config`, so the
//! domain layer never decodes an untrusted profile document.

use std::collections::BTreeMap;

use crate::boundary::decode_error::DecodeError;
use crate::config_types::{ConfigProfileName, PolicyOwner, PolicyReason, RuleEnabled};
use crate::ids::RuleId;
use crate::severity::Severity;

/// A language identity for which the doctrine contract can describe families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Closed language identity used by a doctrine profile."]
pub enum DoctrineLanguage {
    /// TypeScript and JavaScript boundary-shape families.
    Typescript,
    /// Python boundary-shape families.
    Python,
    /// Rust boundary-shape families.
    Rust,
}

const TYPESCRIPT_FAMILIES: &[DoctrineFrameworkFamily] = &[
    DoctrineFrameworkFamily::Effect,
    DoctrineFrameworkFamily::Zod,
    DoctrineFrameworkFamily::Valibot,
];
const PYTHON_FAMILIES: &[DoctrineFrameworkFamily] = &[
    DoctrineFrameworkFamily::Pydantic,
    DoctrineFrameworkFamily::AttrsValidators,
];
const RUST_FAMILIES: &[DoctrineFrameworkFamily] = &[DoctrineFrameworkFamily::SerdeNewtypes];

impl DoctrineLanguage {
    /// Return the stable profile spelling for this language.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Typescript => "typescript",
            Self::Python => "python",
            Self::Rust => "rust",
        }
    }

    /// Decode a closed language spelling at the configuration boundary.
    pub fn from_wire(raw: &str) -> Result<Self, DecodeError> {
        match raw {
            "typescript" => Ok(Self::Typescript),
            "python" => Ok(Self::Python),
            "rust" => Ok(Self::Rust),
            _ => Err(DecodeError::new(
                "language",
                format!("unsupported doctrine language `{raw}`"),
            )),
        }
    }

    /// Return the only framework families valid for this language.
    #[must_use]
    pub const fn valid_families(self) -> &'static [DoctrineFrameworkFamily] {
        match self {
            Self::Typescript => TYPESCRIPT_FAMILIES,
            Self::Python => PYTHON_FAMILIES,
            Self::Rust => RUST_FAMILIES,
        }
    }
}

/// A framework family that can produce a validated boundary shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Closed framework-family identity selected by a doctrine profile."]
pub enum DoctrineFrameworkFamily {
    /// Effect Schema boundary validation for TypeScript.
    Effect,
    /// Zod boundary validation for TypeScript.
    Zod,
    /// Valibot boundary validation for TypeScript.
    Valibot,
    /// Pydantic boundary validation for Python.
    Pydantic,
    /// attrs plus validators boundary validation for Python.
    AttrsValidators,
    /// serde plus validated newtypes for Rust.
    SerdeNewtypes,
}

impl DoctrineFrameworkFamily {
    /// Return the stable profile spelling for this family.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Effect => "effect",
            Self::Zod => "zod",
            Self::Valibot => "valibot",
            Self::Pydantic => "pydantic",
            Self::AttrsValidators => "attrs-validators",
            Self::SerdeNewtypes => "serde-newtypes",
        }
    }

    /// Decode a closed framework-family spelling at the configuration boundary.
    pub fn from_wire(raw: &str) -> Result<Self, DecodeError> {
        match raw {
            "effect" => Ok(Self::Effect),
            "zod" => Ok(Self::Zod),
            "valibot" => Ok(Self::Valibot),
            "pydantic" => Ok(Self::Pydantic),
            "attrs-validators" => Ok(Self::AttrsValidators),
            "serde-newtypes" => Ok(Self::SerdeNewtypes),
            _ => Err(DecodeError::new(
                "family",
                format!("unsupported doctrine framework family `{raw}`"),
            )),
        }
    }

    /// Return whether this family is valid for the selected language.
    #[must_use]
    pub fn is_valid_for(self, language: DoctrineLanguage) -> bool {
        language.valid_families().contains(&self)
    }
}

/// A universal boundary requirement independent of any library family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Closed shape-driven doctrine requirement."]
pub enum DoctrineRequirement {
    /// Input is decoded and validated at a declared boundary.
    ParseAtBoundary,
    /// A reusable schema or equivalent typed shape is present at a boundary.
    SchemaRequired,
    /// Raw strings do not cross a boundary where a typed value is required.
    NoRawBoundaryStrings,
    /// Domain identity is represented by validated values rather than primitives.
    BrandDomainValues,
}

const ALL_REQUIREMENTS: &[DoctrineRequirement] = &[
    DoctrineRequirement::ParseAtBoundary,
    DoctrineRequirement::SchemaRequired,
    DoctrineRequirement::NoRawBoundaryStrings,
    DoctrineRequirement::BrandDomainValues,
];

impl DoctrineRequirement {
    /// Return the stable profile spelling for this requirement.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::ParseAtBoundary => "parse-at-boundary",
            Self::SchemaRequired => "schema-required",
            Self::NoRawBoundaryStrings => "no-raw-boundary-strings",
            Self::BrandDomainValues => "brand-domain-values",
        }
    }

    /// Decode a closed requirement spelling at the configuration boundary.
    pub fn from_wire(raw: &str) -> Result<Self, DecodeError> {
        match raw {
            "parse-at-boundary" => Ok(Self::ParseAtBoundary),
            "schema-required" => Ok(Self::SchemaRequired),
            "no-raw-boundary-strings" => Ok(Self::NoRawBoundaryStrings),
            "brand-domain-values" => Ok(Self::BrandDomainValues),
            _ => Err(DecodeError::new(
                "requirement",
                format!("unsupported doctrine requirement `{raw}`"),
            )),
        }
    }

    /// Return every requirement that a complete profile must declare.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        ALL_REQUIREMENTS
    }
}

/// The result of resolving one requirement and one detected framework family.
///
/// The state is intentionally opaque: callers can inspect the verdict, but
/// only the profile resolver can create one. This prevents rule code from
/// manufacturing an accepted or disabled outcome without selected profile
/// data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Visible profile verdict; disabled requirements never become clean passes."]
pub struct DoctrineVerdict {
    state: DoctrineVerdictState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctrineVerdictState {
    Accepted,
    Rejected,
    RequirementDisabled,
}

impl DoctrineVerdict {
    const fn accepted() -> Self {
        Self {
            state: DoctrineVerdictState::Accepted,
        }
    }

    const fn rejected() -> Self {
        Self {
            state: DoctrineVerdictState::Rejected,
        }
    }

    const fn requirement_disabled() -> Self {
        Self {
            state: DoctrineVerdictState::RequirementDisabled,
        }
    }

    /// Return whether the selected family satisfied the active requirement.
    #[must_use]
    pub const fn is_accepted(self) -> bool {
        matches!(self.state, DoctrineVerdictState::Accepted)
    }

    /// Return whether the selected family was rejected by the profile.
    #[must_use]
    pub const fn is_rejected(self) -> bool {
        matches!(self.state, DoctrineVerdictState::Rejected)
    }

    /// Return whether the profile explicitly disabled the requirement.
    #[must_use]
    pub const fn is_requirement_disabled(self) -> bool {
        matches!(self.state, DoctrineVerdictState::RequirementDisabled)
    }
}

/// One explicit family toggle inside a requirement policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Closed enabled/disabled state for one framework family."]
pub struct DoctrineFamilyPolicy {
    enabled: RuleEnabled,
}

impl DoctrineFamilyPolicy {
    /// Construct a family toggle from the canonical enabled state.
    #[must_use]
    pub const fn from_state(enabled: RuleEnabled) -> Self {
        Self { enabled }
    }

    /// Return the canonical state stored for this family.
    #[must_use]
    pub const fn state(self) -> RuleEnabled {
        self.enabled
    }

    /// Return whether this family is accepted by the toggle.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        matches!(self.enabled, RuleEnabled::Enabled)
    }
}

/// One requirement's active state, severity, family toggles, and weakening explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Validated policy for one universal doctrine requirement."]
pub struct DoctrineRequirementPolicy {
    enabled: RuleEnabled,
    severity: Severity,
    families: BTreeMap<DoctrineFrameworkFamily, DoctrineFamilyPolicy>,
    owner: Option<PolicyOwner>,
    reason: Option<PolicyReason>,
}

impl DoctrineRequirementPolicy {
    /// Construct a requirement policy after validating its explanation invariants.
    pub fn try_from_parts(
        enabled: RuleEnabled,
        severity: Severity,
        families: BTreeMap<DoctrineFrameworkFamily, DoctrineFamilyPolicy>,
        owner: Option<PolicyOwner>,
        reason: Option<PolicyReason>,
    ) -> Result<Self, DecodeError> {
        validate_explanation(
            enabled,
            owner.as_ref(),
            reason.as_ref(),
            DoctrineErrorContext::Requirement,
        )?;
        if matches!(enabled, RuleEnabled::Enabled) && families.is_empty() {
            return Err(DecodeError::new(
                "families",
                "an enabled doctrine requirement must declare family toggles",
            ));
        }
        Ok(Self {
            enabled,
            severity,
            families,
            owner,
            reason,
        })
    }

    /// Return the requirement's enabled state.
    #[must_use]
    pub const fn state(&self) -> RuleEnabled {
        self.enabled
    }

    /// Return the severity selected for findings from this requirement.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Return the optional owner explaining an explicit weakening.
    #[must_use]
    pub fn owner(&self) -> Option<&PolicyOwner> {
        self.owner.as_ref()
    }

    /// Return the optional reason explaining an explicit weakening.
    #[must_use]
    pub fn reason(&self) -> Option<&PolicyReason> {
        self.reason.as_ref()
    }

    /// Iterate over every explicit family toggle in deterministic order.
    pub fn family_policies(
        &self,
    ) -> impl Iterator<Item = (&DoctrineFrameworkFamily, &DoctrineFamilyPolicy)> {
        self.families.iter()
    }

    /// Resolve one family against this requirement's state.
    #[must_use]
    pub fn resolve(&self, family: DoctrineFrameworkFamily) -> DoctrineVerdict {
        if matches!(self.enabled, RuleEnabled::Disabled) {
            return DoctrineVerdict::requirement_disabled();
        }
        match self.families.get(&family) {
            Some(policy) if policy.is_enabled() => DoctrineVerdict::accepted(),
            _ => DoctrineVerdict::rejected(),
        }
    }
}

/// Typed row passed from a wire boundary into the complete profile builder.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Serialization-free family row used to assemble a requirement policy."]
pub struct DoctrineFamilyRow {
    family: DoctrineFrameworkFamily,
    policy: DoctrineFamilyPolicy,
}

impl DoctrineFamilyRow {
    /// Construct one typed family row without accepting raw wire text.
    #[must_use]
    pub const fn from_parts(family: DoctrineFrameworkFamily, policy: DoctrineFamilyPolicy) -> Self {
        Self { family, policy }
    }
}

/// Typed row passed from a wire boundary into the complete profile builder.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Serialization-free requirement row used to assemble a doctrine profile."]
pub struct DoctrineRequirementRow {
    requirement: DoctrineRequirement,
    policy: DoctrineRequirementPolicy,
}

/// Typed inputs used to construct one validated requirement policy.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Serialization-free requirement-policy inputs collected before validation."]
pub struct DoctrineRequirementPolicyParts {
    enabled: RuleEnabled,
    severity: Severity,
    families: Vec<DoctrineFamilyRow>,
    owner: Option<PolicyOwner>,
    reason: Option<PolicyReason>,
}

impl DoctrineRequirementPolicyParts {
    /// Collect typed requirement-policy inputs without accepting wire text.
    #[must_use]
    pub fn from_parts(
        enabled: RuleEnabled,
        severity: Severity,
        families: Vec<DoctrineFamilyRow>,
        owner: Option<PolicyOwner>,
        reason: Option<PolicyReason>,
    ) -> Self {
        Self {
            enabled,
            severity,
            families,
            owner,
            reason,
        }
    }

    fn into_policy(self) -> Result<DoctrineRequirementPolicy, DecodeError> {
        let mut family_map = BTreeMap::new();
        for row in self.families {
            let family = row.family;
            let policy = row.policy;
            if family_map.insert(family, policy).is_some() {
                return Err(DecodeError::new(
                    "families",
                    format!("duplicate family `{}`", family.wire_name()),
                ));
            }
        }
        DoctrineRequirementPolicy::try_from_parts(
            self.enabled,
            self.severity,
            family_map,
            self.owner,
            self.reason,
        )
    }
}

impl DoctrineRequirementRow {
    /// Convert typed row parts into one validated requirement row.
    pub fn try_from_parts(
        requirement: DoctrineRequirement,
        policy_parts: DoctrineRequirementPolicyParts,
    ) -> Result<Self, DecodeError> {
        let policy = policy_parts.into_policy()?;
        Ok(Self {
            requirement,
            policy,
        })
    }
}

/// One rule-specific toggle and severity override carried by a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Validated per-rule doctrine toggle used by future rule adapters."]
pub struct DoctrineRulePolicy {
    enabled: RuleEnabled,
    severity: Severity,
    owner: Option<PolicyOwner>,
    reason: Option<PolicyReason>,
}

impl DoctrineRulePolicy {
    /// Construct a rule toggle after validating its explanation invariants.
    pub fn try_from_parts(
        enabled: RuleEnabled,
        severity: Severity,
        owner: Option<PolicyOwner>,
        reason: Option<PolicyReason>,
    ) -> Result<Self, DecodeError> {
        validate_explanation(
            enabled,
            owner.as_ref(),
            reason.as_ref(),
            DoctrineErrorContext::RuleToggle,
        )?;
        Ok(Self {
            enabled,
            severity,
            owner,
            reason,
        })
    }

    /// Return the rule's enabled state.
    #[must_use]
    pub const fn state(&self) -> RuleEnabled {
        self.enabled
    }

    /// Return the rule's selected severity.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Return the optional owner for an explicit rule weakening.
    #[must_use]
    pub fn owner(&self) -> Option<&PolicyOwner> {
        self.owner.as_ref()
    }

    /// Return the optional reason for an explicit rule weakening.
    #[must_use]
    pub fn reason(&self) -> Option<&PolicyReason> {
        self.reason.as_ref()
    }
}

/// Typed row passed from a wire boundary into the complete profile builder.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Serialization-free rule-toggle row used to assemble a doctrine profile."]
pub struct DoctrineRuleRow {
    rule_id: RuleId,
    policy: DoctrineRulePolicy,
}

impl DoctrineRuleRow {
    /// Convert typed row parts into one validated rule-toggle row.
    pub fn try_from_parts(
        rule_id: RuleId,
        enabled: RuleEnabled,
        severity: Severity,
        owner: Option<PolicyOwner>,
        reason: Option<PolicyReason>,
    ) -> Result<Self, DecodeError> {
        let policy = DoctrineRulePolicy::try_from_parts(enabled, severity, owner, reason)?;
        Ok(Self { rule_id, policy })
    }
}

/// A complete language-aware shape-driven doctrine profile.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Closed doctrine profile separating requirements from framework families."]
pub struct DoctrineProfile {
    profile_name: ConfigProfileName,
    language: DoctrineLanguage,
    requirements: BTreeMap<DoctrineRequirement, DoctrineRequirementPolicy>,
    rule_toggles: BTreeMap<RuleId, DoctrineRulePolicy>,
}

impl DoctrineProfile {
    /// Assemble a profile from typed rows while rejecting duplicate identities.
    pub fn try_from_rows(
        profile_name: ConfigProfileName,
        language: DoctrineLanguage,
        requirements: Vec<DoctrineRequirementRow>,
        rule_toggles: Vec<DoctrineRuleRow>,
    ) -> Result<Self, DecodeError> {
        let mut requirement_map = BTreeMap::new();
        for row in requirements {
            let requirement = row.requirement;
            let policy = row.policy;
            if requirement_map.insert(requirement, policy).is_some() {
                return Err(DecodeError::new(
                    "requirements",
                    format!("duplicate requirement `{}`", requirement.wire_name()),
                ));
            }
        }
        let mut rule_map = BTreeMap::new();
        for row in rule_toggles {
            let rule_id = row.rule_id;
            if rule_map.contains_key(&rule_id) {
                return Err(DecodeError::new(
                    "ruleToggles",
                    format!("duplicate rule id `{}`", rule_id.as_str()),
                ));
            }
            rule_map.insert(rule_id, row.policy);
        }
        Self::try_from_parts(profile_name, language, requirement_map, rule_map)
    }

    /// Construct a complete profile and reject missing or language-incompatible rows.
    pub fn try_from_parts(
        profile_name: ConfigProfileName,
        language: DoctrineLanguage,
        requirements: BTreeMap<DoctrineRequirement, DoctrineRequirementPolicy>,
        rule_toggles: BTreeMap<RuleId, DoctrineRulePolicy>,
    ) -> Result<Self, DecodeError> {
        if requirements.len() != DoctrineRequirement::all().len() {
            return Err(DecodeError::new(
                "requirements",
                "a doctrine profile must declare each closed requirement exactly once",
            ));
        }
        for requirement in DoctrineRequirement::all() {
            let Some(policy) = requirements.get(requirement) else {
                return Err(DecodeError::new(
                    "requirements",
                    format!("missing requirement `{}`", requirement.wire_name()),
                ));
            };
            let expected_families = language.valid_families();
            for (family, _) in policy.family_policies() {
                if !family.is_valid_for(language) {
                    return Err(DecodeError::new(
                        "family",
                        format!(
                            "family `{}` is not valid for language `{}`",
                            family.wire_name(),
                            language.wire_name()
                        ),
                    ));
                }
            }
            if policy.family_policies().count() != expected_families.len() {
                return Err(DecodeError::new(
                    "families",
                    format!(
                        "requirement `{}` must declare every family valid for `{}`",
                        requirement.wire_name(),
                        language.wire_name()
                    ),
                ));
            }
            for family in expected_families {
                if policy
                    .family_policies()
                    .all(|(candidate, _)| candidate != family)
                {
                    return Err(DecodeError::new(
                        "families",
                        format!(
                            "requirement `{}` is missing family `{}`",
                            requirement.wire_name(),
                            family.wire_name()
                        ),
                    ));
                }
            }
        }
        Ok(Self {
            profile_name,
            language,
            requirements,
            rule_toggles,
        })
    }

    /// Return the profile's validated name.
    #[must_use]
    pub fn profile_name(&self) -> &ConfigProfileName {
        &self.profile_name
    }

    /// Return the language selected by this profile.
    #[must_use]
    pub const fn language(&self) -> DoctrineLanguage {
        self.language
    }

    /// Iterate over all requirement policies in stable order.
    pub fn requirements(
        &self,
    ) -> impl Iterator<Item = (&DoctrineRequirement, &DoctrineRequirementPolicy)> {
        self.requirements.iter()
    }

    /// Iterate over all explicitly configured rule toggles in stable order.
    pub fn rule_toggles(&self) -> impl Iterator<Item = (&RuleId, &DoctrineRulePolicy)> {
        self.rule_toggles.iter()
    }

    /// Resolve a language/requirement/family tuple through profile data.
    #[must_use]
    pub fn resolve(
        &self,
        language: DoctrineLanguage,
        requirement: DoctrineRequirement,
        family: DoctrineFrameworkFamily,
    ) -> DoctrineVerdict {
        if language != self.language || !family.is_valid_for(language) {
            return DoctrineVerdict::rejected();
        }
        self.requirements
            .get(&requirement)
            .map_or(DoctrineVerdict::rejected(), |policy| policy.resolve(family))
    }

    /// Look up a rule toggle without inventing an implicit default.
    #[must_use]
    pub fn rule_policy(&self, rule_id: &RuleId) -> Option<&DoctrineRulePolicy> {
        self.rule_toggles.get(rule_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctrineErrorContext {
    Requirement,
    RuleToggle,
}

impl DoctrineErrorContext {
    const fn path(self) -> &'static str {
        match self {
            Self::Requirement => "requirement",
            Self::RuleToggle => "ruleToggle",
        }
    }
}

fn validate_explanation(
    enabled: RuleEnabled,
    owner: Option<&PolicyOwner>,
    reason: Option<&PolicyReason>,
    context: DoctrineErrorContext,
) -> Result<(), DecodeError> {
    if owner.is_some() != reason.is_some() {
        return Err(DecodeError::new(
            context.path(),
            "owner and reason must be supplied together",
        ));
    }
    if matches!(enabled, RuleEnabled::Disabled) && owner.is_none() {
        return Err(DecodeError::new(
            context.path(),
            "disabled doctrine requirements and rules require owner and reason",
        ));
    }
    Ok(())
}
