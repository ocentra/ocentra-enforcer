//! The rule scaffolder: given a minimal spec for a NEW rule, emit a
//! well-formed `enforcer_rules::registry::RuleRecord` plus the source text
//! for a bare `Validator` skeleton and two fixture-slot file contents
//! (fail/pass).
//!
//! This is the Rust replacement for hand-writing a new rule's JSON record
//! and validator boilerplate by hand. It does not write anything to disk —
//! callers (a CLI command, a test, a future `enforcer coordination`
//! integration) decide where the generated text lands. Emitting well-formed
//! DATA is this module's job; the fail-closed acceptance gate lives in
//! [`crate::oracle`].

use enforcer_domain::config_types::CrateName;
use enforcer_domain::ids::RuleId;
use enforcer_domain::mechanization_types::{FixtureSlotContent, GeneratedValidatorSource};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::{
    RuleDocAnchor, RuleParameters, RuleTag, RuleTitle, RuleVersion, ValidatorPath,
};
use enforcer_domain::severity::Tier;
use enforcer_rules::registry::{FixtureRef, RuleRecord, ValidatorRef};

use crate::error::{MechanizationError, MechanizationResult};

/// Minimal caller-supplied spec for a new rule. Every field here maps
/// directly onto a [`RuleRecord`] field; the scaffolder's job is to reject a
/// malformed spec before a half-formed record is ever constructed, and to
/// derive the boilerplate (validator skeleton source, fixture slot
/// contents) a human would otherwise write by hand.
#[derive(Debug, Clone)]
pub struct ScaffoldSpec {
    /// Branded rule id for the new rule, e.g. `RR-42.1`.
    pub rule_id: RuleId,
    /// Short human title.
    pub title: RuleTitle,
    /// Mechanical-enforcement tier.
    pub tier: Tier,
    /// Crate that will own the `Validator` implementation.
    pub validator_crate: CrateName,
    /// Type/function path within that crate, e.g. `no_foo::NoFooValidator`.
    pub validator_path: ValidatorPath,
    /// Repo-relative path the fail fixture will live at.
    pub fail_fixture_path: RelPath,
    /// Repo-relative path the pass fixture will live at.
    pub pass_fixture_path: RelPath,
    /// Repo-relative doc anchor for the human-canonical rule doc.
    pub doc_anchor: RuleDocAnchor,
    /// Free-form family tags.
    pub tags: Vec<RuleTag>,
}

/// The scaffolder's output: a well-formed [`RuleRecord`] plus generated
/// boilerplate text for the validator skeleton and both fixture slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldOutput {
    /// The rule record, ready to be appended to a catalog and loaded by
    /// `enforcer_rules::loader`.
    pub record: RuleRecord,
    /// Generated Rust source for a bare `Validator` implementation. Always
    /// returns zero findings (`todo!`-free per workspace lint policy) — an
    /// intentionally inert skeleton that WILL fail the parity oracle until
    /// a human fills in real detection logic, which is the point:
    /// scaffolding never silently produces an already-passing rule.
    pub validator_skeleton_source: GeneratedValidatorSource,
    /// Starter content for the fail fixture slot (a file that SHOULD trip
    /// the eventual validator once implemented).
    pub fail_fixture_slot: FixtureSlotContent,
    /// Starter content for the pass fixture slot (a file that must stay
    /// clean).
    pub pass_fixture_slot: FixtureSlotContent,
}

/// Scaffold a new rule from `spec`. Fails closed (returns
/// [`MechanizationError::InvalidSpec`]) on any structurally empty field —
/// same discipline `enforcer_rules::registry` applies at load time, applied
/// here at GENERATION time so a malformed spec never even reaches a
/// catalog file.
pub fn scaffold_rule(spec: &ScaffoldSpec) -> MechanizationResult<ScaffoldOutput> {
    // Every textual specification field is already a validated canonical
    // domain value. The registry below is the raw catalog boundary.
    if spec.fail_fixture_path == spec.pass_fixture_path {
        return Err(MechanizationError::InvalidSpec {
            reason: "failFixturePath and passFixturePath must differ".parse()?,
        });
    }

    let record = RuleRecord {
        // CLONE-JUSTIFICATION: `record` must own its rule id while `spec` remains borrowed for generated output below.
        rule_id: spec.rule_id.clone(),
        version: RuleVersion::try_new(std::num::NonZeroU32::MIN),
        // CLONE-JUSTIFICATION: `record` retains its title independently of the caller-owned scaffold specification.
        title: spec.title.clone(),
        tier: spec.tier,
        validator: ValidatorRef {
            // CLONE-JUSTIFICATION: the emitted record owns validator metadata after this borrowed specification is returned to its caller.
            crate_name: spec.validator_crate.clone(),
            // CLONE-JUSTIFICATION: the emitted record owns validator metadata after this borrowed specification is returned to its caller.
            path: spec.validator_path.clone(),
        },
        fixtures: FixtureRef {
            // CLONE-JUSTIFICATION: fixture locations belong to the durable rule record while `spec` remains available for source generation.
            fail: spec.fail_fixture_path.clone(),
            // CLONE-JUSTIFICATION: fixture locations belong to the durable rule record while `spec` remains available for source generation.
            pass: spec.pass_fixture_path.clone(),
        },
        // CLONE-JUSTIFICATION: the catalog record must own its documentation location beyond this borrowed input.
        doc_anchor: spec.doc_anchor.clone(),
        // CLONE-JUSTIFICATION: the catalog record must own its tags beyond this borrowed input.
        tags: spec.tags.clone(),
        params: RuleParameters::default(),
    };

    Ok(ScaffoldOutput {
        validator_skeleton_source: render_validator_skeleton(spec)?,
        fail_fixture_slot: render_fail_fixture_slot(spec)?,
        pass_fixture_slot: render_pass_fixture_slot(spec)?,
        record,
    })
}

fn render_validator_skeleton(spec: &ScaffoldSpec) -> MechanizationResult<GeneratedValidatorSource> {
    let type_name = spec
        .validator_path
        .as_str()
        .rsplit("::")
        .next()
        .unwrap_or(spec.validator_path.as_str());
    format!(
        "//! Freshly scaffolded validator for `{rule_id}` — {title}.\n\
         //! This validator intentionally never fires yet: fill in real\n\
         //! detection logic, then re-run the fail-closed parity oracle\n\
         //! (`enforcer_mechanization::oracle::accept_rule`) against the\n\
         //! fail/pass fixture slots before this rule is accepted.\n\
         \n\
         use enforcer_domain::findings::Finding;\n\
         use enforcer_domain::ids::RuleId;\n\
         use enforcer_validator::validator::{{ValidationInput, Validator}};\n\
         \n\
         /// Scaffolded validator for `{rule_id}`.\n\
         pub struct {type_name} {{\n\
         \x20\x20\x20\x20rule_id: RuleId,\n\
         }}\n\
         \n\
         impl Validator for {type_name} {{\n\
         \x20\x20\x20\x20fn rule_id(&self) -> &RuleId {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20&self.rule_id\n\
         \x20\x20\x20\x20}}\n\
         \n\
         \x20\x20\x20\x20fn validate(&self, _input: ValidationInput<'_>) -> Vec<Finding> {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20// NEXT STEP: implement detection for {rule_id} here.\n\
         \x20\x20\x20\x20\x20\x20\x20\x20Vec::new()\n\
         \x20\x20\x20\x20}}\n\
         }}\n",
        rule_id = spec.rule_id.as_str(),
        title = spec.title,
        type_name = type_name,
    )
    .try_into()
    .map_err(Into::into)
}

fn render_fail_fixture_slot(spec: &ScaffoldSpec) -> MechanizationResult<FixtureSlotContent> {
    format!(
        "// FAIL fixture slot for {rule_id}.\n+         // Fill this slot with source text that MUST trip the rule once\n+         // its validator is implemented. An empty/unfilled slot will\n+         // correctly be rejected by the parity oracle (the validator\n+         // skeleton never fires, so this slot will not pass parity until\n+         // BOTH the validator and this fixture carry real content).\n",
        rule_id = spec.rule_id.as_str(),
    )
    .try_into()
    .map_err(Into::into)
}

fn render_pass_fixture_slot(spec: &ScaffoldSpec) -> MechanizationResult<FixtureSlotContent> {
    format!(
        "// PASS fixture slot for {rule_id}.\n+         // Fill this slot with clean source text that must NOT trip the\n+         // rule.\n",
        rule_id = spec.rule_id.as_str(),
    )
    .try_into()
    .map_err(Into::into)
}
