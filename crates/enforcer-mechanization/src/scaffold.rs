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

use enforcer_domain::ids::RuleId;
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
    pub title: String,
    /// Mechanical-enforcement tier.
    pub tier: Tier,
    /// Crate that will own the `Validator` implementation.
    pub validator_crate: String,
    /// Type/function path within that crate, e.g. `no_foo::NoFooValidator`.
    pub validator_path: String,
    /// Repo-relative path the fail fixture will live at.
    pub fail_fixture_path: String,
    /// Repo-relative path the pass fixture will live at.
    pub pass_fixture_path: String,
    /// Repo-relative doc anchor for the human-canonical rule doc.
    pub doc_anchor: String,
    /// Free-form family tags.
    pub tags: Vec<String>,
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
    pub validator_skeleton_source: String,
    /// Starter content for the fail fixture slot (a file that SHOULD trip
    /// the eventual validator once implemented).
    pub fail_fixture_slot: String,
    /// Starter content for the pass fixture slot (a file that must stay
    /// clean).
    pub pass_fixture_slot: String,
}

/// Scaffold a new rule from `spec`. Fails closed (returns
/// [`MechanizationError::InvalidSpec`]) on any structurally empty field —
/// same discipline `enforcer_rules::registry` applies at load time, applied
/// here at GENERATION time so a malformed spec never even reaches a
/// catalog file.
pub fn scaffold_rule(spec: &ScaffoldSpec) -> MechanizationResult<ScaffoldOutput> {
    require_non_empty("title", &spec.title)?;
    require_non_empty("validatorCrate", &spec.validator_crate)?;
    require_non_empty("validatorPath", &spec.validator_path)?;
    require_non_empty("failFixturePath", &spec.fail_fixture_path)?;
    require_non_empty("passFixturePath", &spec.pass_fixture_path)?;
    require_non_empty("docAnchor", &spec.doc_anchor)?;
    if spec.fail_fixture_path == spec.pass_fixture_path {
        return Err(MechanizationError::InvalidSpec {
            reason: "failFixturePath and passFixturePath must differ".to_owned(),
        });
    }

    let record = RuleRecord {
        rule_id: spec.rule_id.clone(),
        version: 1,
        title: spec.title.clone(),
        tier: spec.tier,
        validator: ValidatorRef {
            crate_name: spec.validator_crate.clone(),
            path: spec.validator_path.clone(),
        },
        fixtures: FixtureRef {
            fail: spec.fail_fixture_path.clone(),
            pass: spec.pass_fixture_path.clone(),
        },
        doc_anchor: spec.doc_anchor.clone(),
        tags: spec.tags.clone(),
        params: serde_json::Value::Null,
    };

    Ok(ScaffoldOutput {
        validator_skeleton_source: render_validator_skeleton(spec),
        fail_fixture_slot: render_fixture_slot(spec, true),
        pass_fixture_slot: render_fixture_slot(spec, false),
        record,
    })
}

fn require_non_empty(field: &'static str, value: &str) -> MechanizationResult<()> {
    if value.trim().is_empty() {
        Err(MechanizationError::InvalidSpec {
            reason: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

/// Type-name fragment derived from the last segment of `validator_path`
/// (e.g. `no_foo::NoFooValidator` -> `NoFooValidator`), used only to make
/// the generated skeleton source read naturally; not itself validated as
/// an identifier since it is human-edited before compiling.
fn validator_type_name(spec: &ScaffoldSpec) -> &str {
    spec.validator_path
        .rsplit("::")
        .next()
        .unwrap_or(spec.validator_path.as_str())
}

fn render_validator_skeleton(spec: &ScaffoldSpec) -> String {
    let type_name = validator_type_name(spec);
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
}

fn render_fixture_slot(spec: &ScaffoldSpec, is_fail: bool) -> String {
    if is_fail {
        format!(
            "// FAIL fixture slot for {rule_id}.\n\
             // Fill this slot with source text that MUST trip the rule once\n\
             // its validator is implemented. An empty/unfilled slot will\n\
             // correctly be rejected by the parity oracle (the validator\n\
             // skeleton never fires, so this slot will not pass parity until\n\
             // BOTH the validator and this fixture carry real content).\n",
            rule_id = spec.rule_id.as_str(),
        )
    } else {
        format!(
            "// PASS fixture slot for {rule_id}.\n\
             // Fill this slot with clean source text that must NOT trip the\n\
             // rule.\n",
            rule_id = spec.rule_id.as_str(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{scaffold_rule, ScaffoldSpec};
    use enforcer_domain::severity::Tier;

    fn sample_spec() -> Result<ScaffoldSpec, enforcer_core::error::DecodeError> {
        Ok(ScaffoldSpec {
            rule_id: "RR-42.1".parse()?,
            title: "No frobnicating".to_owned(),
            tier: Tier::T1,
            validator_crate: "enforcer-lang-rust".to_owned(),
            validator_path: "no_frobnicate::NoFrobnicateValidator".to_owned(),
            fail_fixture_path: "crates/enforcer-lang-rust/fixtures/no_frobnicate/fail.rs"
                .to_owned(),
            pass_fixture_path: "crates/enforcer-lang-rust/fixtures/no_frobnicate/pass.rs"
                .to_owned(),
            doc_anchor: "docs/rules/FROB.md#FROB-1".to_owned(),
            tags: vec!["rust".to_owned()],
        })
    }

    #[test]
    fn scaffolds_a_loadable_record() -> Result<(), Box<dyn std::error::Error>> {
        let spec = sample_spec()?;
        let output = scaffold_rule(&spec)?;
        assert_eq!(output.record.rule_id.as_str(), "RR-42.1");
        assert_eq!(output.record.version, 1);

        // The scaffolder's own output must be independently loadable by the
        // registry it targets — the whole point of emitting a well-formed
        // record.
        let registry = enforcer_rules::registry::RuleRegistry::from_records(vec![output.record])?;
        assert_eq!(registry.len(), 1);
        Ok(())
    }

    #[test]
    fn validator_skeleton_names_the_type_and_rule() -> Result<(), Box<dyn std::error::Error>> {
        let spec = sample_spec()?;
        let output = scaffold_rule(&spec)?;
        assert!(output
            .validator_skeleton_source
            .contains("struct NoFrobnicateValidator"));
        assert!(output.validator_skeleton_source.contains("RR-42.1"));
        Ok(())
    }

    #[test]
    fn rejects_empty_title() -> Result<(), Box<dyn std::error::Error>> {
        let mut spec = sample_spec()?;
        spec.title = "   ".to_owned();
        assert!(scaffold_rule(&spec).is_err());
        Ok(())
    }

    #[test]
    fn rejects_identical_fail_and_pass_fixture_paths() -> Result<(), Box<dyn std::error::Error>> {
        let mut spec = sample_spec()?;
        spec.pass_fixture_path = spec.fail_fixture_path.clone();
        assert!(scaffold_rule(&spec).is_err());
        Ok(())
    }

    #[test]
    fn rejects_empty_validator_path() -> Result<(), Box<dyn std::error::Error>> {
        let mut spec = sample_spec()?;
        spec.validator_path = String::new();
        assert!(scaffold_rule(&spec).is_err());
        Ok(())
    }
}
