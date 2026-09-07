//! BOUNDARY-INVARIANT: CP05 accepts only validated typed component evidence
//! and emits an in-memory clerical packet; it never writes a repository file.
//! NEGATIVE-TEST: missing source identity, unapproved components, duplicate
//! fixtures, protected source paths, and incomplete packet paths fail closed.
//! SERIALIZATION-DOC: the JSON view is a supplied-input skeleton and carries
//! no generated severity, citation, predicate, or security outcome.
//!
//! CP05 is intentionally a factory for wiring, not a security-rule generator.
//! It removes repetitive path/evidence clerical work after a component has
//! already been approved by the graph. The semantic predicate and `notProved`
//! text remain caller-supplied from that approved input and are never inferred here.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::RuleId;
use serde_json::{json, Value};

const SOURCE_ROOT: &str = "vendor/anthropic-cybersecurity-skills/skills/";
const FIXTURE_ROOT: &str = "crates/enforcer-lang-security/tests/fixtures/";
const PROTECTED_SOURCE: &str = "detecting-fileless-malware-techniques";

/// Owned transport input for a component identity before domain validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentIdInput {
    /// BRAND-INVARIANT: this owned transport value is validated by `ComponentId::try_new`.
    value: String,
}

impl From<String> for ComponentIdInput {
    fn from(value: String) -> Self {
        Self { value }
    }
}

/// Owned transport input for a source digest before domain validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSha256Input {
    /// BRAND-INVARIANT: this owned transport value is validated by `SourceSha256::try_new`.
    value: String,
}

impl From<String> for SourceSha256Input {
    fn from(value: String) -> Self {
        Self { value }
    }
}

/// Owned transport input for a source path before domain validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePathInput {
    /// BRAND-INVARIANT: this owned transport value is validated by `SourcePath::try_new`.
    value: String,
}

impl From<String> for SourcePathInput {
    fn from(value: String) -> Self {
        Self { value }
    }
}

/// Owned transport input for a source anchor before domain validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAnchorInput {
    /// BRAND-INVARIANT: this owned transport value is validated by `SourceAnchor::try_new`.
    value: String,
}

impl From<String> for SourceAnchorInput {
    fn from(value: String) -> Self {
        Self { value }
    }
}

/// Owned transport input for a license label before domain validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseNameInput {
    /// BRAND-INVARIANT: this owned transport value is validated by `LicenseName::try_new`.
    value: String,
}

impl From<String> for LicenseNameInput {
    fn from(value: String) -> Self {
        Self { value }
    }
}

/// Owned transport input for a predicate description before domain validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateTextInput {
    /// BRAND-INVARIANT: this owned transport value is validated by `PredicateText::try_new`.
    value: String,
}

impl From<String> for PredicateTextInput {
    fn from(value: String) -> Self {
        Self { value }
    }
}

/// Owned transport input for a non-proof description before domain validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotProvedTextInput {
    /// BRAND-INVARIANT: this owned transport value is validated by `NotProvedText::try_new`.
    value: String,
}

impl From<String> for NotProvedTextInput {
    fn from(value: String) -> Self {
        Self { value }
    }
}

/// Owned transport input for a fixture path before domain validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixturePathInput {
    /// BRAND-INVARIANT: this owned transport value is validated by `FixturePath::try_new`.
    value: String,
}

impl From<String> for FixturePathInput {
    fn from(value: String) -> Self {
        Self { value }
    }
}

/// Owned transport input for an output path before domain validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketPathInput {
    /// BRAND-INVARIANT: this owned transport value is validated by `PacketPath::try_new`.
    value: String,
}

impl From<String> for PacketPathInput {
    fn from(value: String) -> Self {
        Self { value }
    }
}

/// A validated CP08 component identity.
///
/// BRAND-INVARIANT: component identifiers are non-empty bounded tokens and
/// never cross the factory boundary as an arbitrary public string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComponentId(String);

impl TryFrom<String> for ComponentId {
    type Error = DecodeError;

    /// Validate and retain one component identity.
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let valid = !raw.is_empty()
            && raw.len() <= 128
            && raw
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character));
        valid
            .then_some(Self(raw))
            .ok_or_else(|| DecodeError::new("componentId", "expected a bounded component token"))
    }
}

impl ComponentId {
    /// Validate one component identity from an owned boundary value.
    pub fn try_new(value: ComponentIdInput) -> Result<Self, DecodeError> {
        Self::try_from(value.value)
    }
}

/// A validated lowercase source SHA-256 value without a self-referential
/// artifact wrapper.
///
/// BRAND-INVARIANT: exactly 64 lowercase hexadecimal characters are stored.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceSha256(String);

impl TryFrom<String> for SourceSha256 {
    type Error = DecodeError;

    /// Validate one vendor source digest.
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let valid = raw.len() == 64
            && raw
                .chars()
                .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character));
        valid
            .then_some(Self(raw))
            .ok_or_else(|| DecodeError::new("sourceSha256", "expected 64 lowercase hex characters"))
    }
}

impl SourceSha256 {
    /// Validate one vendor source digest from an owned boundary value.
    pub fn try_new(value: SourceSha256Input) -> Result<Self, DecodeError> {
        Self::try_from(value.value)
    }
}

/// A vendor skill source path accepted by the packet factory.
///
/// BRAND-INVARIANT: the path is a relative Apache CyberSkills source path,
/// ends in `SKILL.md`, and excludes the protected unresolved deletion.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourcePath(String);

impl TryFrom<String> for SourcePath {
    type Error = DecodeError;

    /// Validate one source path without reading it.
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let valid = raw.starts_with(SOURCE_ROOT)
            && raw.ends_with("/SKILL.md")
            && !raw.contains("..")
            && !raw.contains('\\')
            && !raw.contains(PROTECTED_SOURCE);
        valid.then_some(Self(raw)).ok_or_else(|| {
            DecodeError::new("sourcePath", "expected an eligible relative skill path")
        })
    }
}

impl SourcePath {
    /// Validate one source path from an owned boundary value.
    pub fn try_new(value: SourcePathInput) -> Result<Self, DecodeError> {
        Self::try_from(value.value)
    }
}

/// A source heading/line anchor retained as source evidence.
///
/// BRAND-INVARIANT: anchors are non-empty text ending in a positive `:L<n>`
/// line suffix; they are not artifact-anchor values and are never conflated.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceAnchor(String);

impl TryFrom<String> for SourceAnchor {
    type Error = DecodeError;

    /// Validate one source anchor without assuming a heading-only format.
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let valid = raw.rsplit_once(":L").is_some_and(|(heading, line)| {
            !heading.trim().is_empty() && line.parse::<u32>().is_ok_and(|number| number > 0)
        });
        valid.then_some(Self(raw)).ok_or_else(|| {
            DecodeError::new(
                "sourceAnchor",
                "expected non-empty text with a positive line",
            )
        })
    }
}

impl SourceAnchor {
    /// Validate one source anchor from an owned boundary value.
    pub fn try_new(value: SourceAnchorInput) -> Result<Self, DecodeError> {
        Self::try_from(value.value)
    }
}

/// The source license accepted by this CP05 packet.
///
/// BRAND-INVARIANT: CP05 currently accepts only the catalog's Apache-2.0
/// source license label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LicenseName {
    /// Apache License 2.0 source label.
    Apache2,
}

impl TryFrom<String> for LicenseName {
    type Error = DecodeError;

    /// Validate the source license label.
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        (raw == "Apache-2.0")
            .then_some(Self::Apache2)
            .ok_or_else(|| DecodeError::new("license", "CP05 requires Apache-2.0 source evidence"))
    }
}

impl LicenseName {
    /// Validate the stable source license label.
    pub fn try_new(value: LicenseNameInput) -> Result<Self, DecodeError> {
        Self::try_from(value.value)
    }
}

/// Validated source predicate text supplied by the approved component.
///
/// BRAND-INVARIANT: bounded non-empty text is retained; no predicate is
/// inferred from a rule name or fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateText(String);

impl TryFrom<String> for PredicateText {
    type Error = DecodeError;

    /// Validate one supplied predicate description.
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        (!raw.trim().is_empty() && raw.len() <= 4096 && !raw.contains('\0'))
            .then_some(Self(raw))
            .ok_or_else(|| DecodeError::new("predicate", "expected bounded non-empty text"))
    }
}

impl PredicateText {
    /// Validate one supplied predicate from an owned boundary value.
    pub fn try_new(value: PredicateTextInput) -> Result<Self, DecodeError> {
        Self::try_from(value.value)
    }
}

/// Validated non-proof text supplied by the approved component.
///
/// BRAND-INVARIANT: bounded non-empty text is retained so packet generation
/// cannot silently erase uncertainty or claim broader behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotProvedText(String);

impl TryFrom<String> for NotProvedText {
    type Error = DecodeError;

    /// Validate one supplied non-proof description.
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        (!raw.trim().is_empty() && raw.len() <= 4096 && !raw.contains('\0'))
            .then_some(Self(raw))
            .ok_or_else(|| DecodeError::new("notProved", "expected bounded non-empty text"))
    }
}

impl NotProvedText {
    /// Validate one supplied non-proof description from an owned boundary value.
    pub fn try_new(value: NotProvedTextInput) -> Result<Self, DecodeError> {
        Self::try_from(value.value)
    }
}

/// A validated fixture path under the existing security fixture root.
///
/// BRAND-INVARIANT: fixture paths remain relative, fixture-rooted, and free
/// from traversal or Windows separator ambiguity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FixturePath(String);

impl TryFrom<String> for FixturePath {
    type Error = DecodeError;

    /// Validate one fail/pass fixture path.
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let valid = raw.starts_with(FIXTURE_ROOT)
            && !raw.contains("..")
            && !raw.contains('\\')
            && !raw.ends_with('/');
        valid.then_some(Self(raw)).ok_or_else(|| {
            DecodeError::new("fixturePath", "expected a relative fixture-rooted path")
        })
    }
}

impl FixturePath {
    /// Validate one fixture path from an owned boundary value.
    pub fn try_new(value: FixturePathInput) -> Result<Self, DecodeError> {
        Self::try_from(value.value)
    }
}

/// A safe repository-relative output path supplied by the approved packet.
///
/// BRAND-INVARIANT: output paths cannot be absolute, traverse parents, or
/// contain Windows separators; the factory never writes them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PacketPath(String);

impl TryFrom<String> for PacketPath {
    type Error = DecodeError;

    /// Validate one generated-output path.
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let valid = !raw.is_empty()
            && !raw.starts_with('/')
            && !raw.contains("..")
            && !raw.contains('\\')
            && !raw.ends_with('/');
        valid.then_some(Self(raw)).ok_or_else(|| {
            DecodeError::new("packetPath", "expected a safe repository-relative path")
        })
    }
}

impl PacketPath {
    /// Validate one output path from an owned boundary value.
    pub fn try_new(value: PacketPathInput) -> Result<Self, DecodeError> {
        Self::try_from(value.value)
    }
}

/// The source provenance attached to an approved component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceIdentity {
    path: SourcePath,
    sha256: SourceSha256,
    anchor: SourceAnchor,
    license: LicenseName,
}

impl SourceIdentity {
    /// Combine independently validated source evidence roles.
    pub fn new(
        path: SourcePath,
        sha256: SourceSha256,
        anchor: SourceAnchor,
        license: LicenseName,
    ) -> Self {
        Self {
            path,
            sha256,
            anchor,
            license,
        }
    }
}

/// The existing fail/pass fixture pair for a component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSet {
    fail: FixturePath,
    pass: FixturePath,
}

impl FixtureSet {
    /// Combine a distinct fail fixture and pass fixture.
    pub fn new(fail: FixturePath, pass: FixturePath) -> Result<Self, DecodeError> {
        (fail.0 != pass.0)
            .then_some(Self { fail, pass })
            .ok_or_else(|| DecodeError::new("fixtures", "fail and pass fixtures must differ"))
    }
}

/// Output paths that the approved component already owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketPaths {
    registry: PacketPath,
    implementation: PacketPath,
    evidence: PacketPath,
}

impl PacketPaths {
    /// Combine the clerical registry, implementation, and evidence paths.
    pub fn new(registry: PacketPath, implementation: PacketPath, evidence: PacketPath) -> Self {
        Self {
            registry,
            implementation,
            evidence,
        }
    }
}

/// Whether a component has passed the graph's approval boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComponentApproval {
    /// The component is not approved for packet generation.
    #[default]
    Unapproved,
    /// The component has an accepted source/predicate decision.
    Approved,
}

impl ComponentApproval {
    fn require(self) -> Result<(), DecodeError> {
        matches!(self, Self::Approved)
            .then_some(())
            .ok_or_else(|| DecodeError::new("approval", "component is not approved"))
    }
}

/// A typed, possibly incomplete input draft for the packet factory.
///
/// Missing fields remain explicit so the factory can reject clerical gaps
/// mechanically instead of generating placeholders that look complete.
#[derive(Debug, Clone, Default)]
pub struct ComponentDraft {
    /// Graph approval state for this component.
    pub approval: ComponentApproval,
    /// Approved catalog component identity, when supplied.
    pub component_id: Option<ComponentId>,
    /// Approved source provenance, when supplied.
    pub source: Option<SourceIdentity>,
    /// Existing rule identity, when supplied.
    pub rule_id: Option<RuleId>,
    /// Existing precise predicate, when supplied.
    pub predicate: Option<PredicateText>,
    /// Existing explicit non-proof boundary, when supplied.
    pub not_proved: Option<NotProvedText>,
    /// Existing fail/pass fixtures, when supplied.
    pub fixtures: Option<FixtureSet>,
    /// Existing output ownership paths, when supplied.
    pub paths: Option<PacketPaths>,
}

/// The pure output of CP05's clerical packet factory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePacketBlueprint {
    component_id: ComponentId,
    source: SourceIdentity,
    rule_id: RuleId,
    predicate: PredicateText,
    not_proved: NotProvedText,
    fixtures: FixtureSet,
    paths: PacketPaths,
}

impl NativePacketBlueprint {
    /// Render a stable JSON skeleton without writing any file.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "schemaVersion": 1,
            "kind": "native-cyberskill-packet-skeleton",
            "componentId": self.component_id.0,
            "ruleId": self.rule_id.as_str(),
            "source": {
                "path": self.source.path.0,
                "sha256": self.source.sha256.0,
                "anchor": self.source.anchor.0,
                "license": match self.source.license {
                    LicenseName::Apache2 => "Apache-2.0",
                },
            },
            "predicate": self.predicate.0,
            "notProved": self.not_proved.0,
            "fixtures": {
                "fail": self.fixtures.fail.0,
                "pass": self.fixtures.pass.0,
            },
            "paths": {
                "registry": self.paths.registry.0,
                "implementation": self.paths.implementation.0,
                "evidence": self.paths.evidence.0,
            },
            "generated": {
                "writesFiles": false,
                "securityMeaning": "supplied-input-only",
            },
        })
    }
}

/// Pure CP05 factory that validates clerical completeness and emits a
/// non-writing native packet blueprint.
#[derive(Debug)]
pub struct PacketFactory;

impl PacketFactory {
    /// Build a blueprint only when approval and every required evidence link
    /// are present. No file, registry entry, predicate, or outcome is created.
    pub fn build(draft: ComponentDraft) -> Result<NativePacketBlueprint, DecodeError> {
        draft.approval.require()?;
        Ok(NativePacketBlueprint {
            component_id: draft.component_id.ok_or_else(|| {
                DecodeError::new("componentId", "required approved packet field is missing")
            })?,
            source: draft.source.ok_or_else(|| {
                DecodeError::new("source", "required approved packet field is missing")
            })?,
            rule_id: draft.rule_id.ok_or_else(|| {
                DecodeError::new("ruleId", "required approved packet field is missing")
            })?,
            predicate: draft.predicate.ok_or_else(|| {
                DecodeError::new("predicate", "required approved packet field is missing")
            })?,
            not_proved: draft.not_proved.ok_or_else(|| {
                DecodeError::new("notProved", "required approved packet field is missing")
            })?,
            fixtures: draft.fixtures.ok_or_else(|| {
                DecodeError::new("fixtures", "required approved packet field is missing")
            })?,
            paths: draft.paths.ok_or_else(|| {
                DecodeError::new("paths", "required approved packet field is missing")
            })?,
        })
    }
}
