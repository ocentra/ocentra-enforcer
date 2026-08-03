//! Identity-preserving language detection for the scan router.
//!
//! The syntax registry is the only source of parser identity and matcher
//! metadata. This module projects that metadata into scan-owned route values;
//! native-tool values remain typed projections only and do not claim execution
//! or scan success.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::config_types::NativeTool;
use enforcer_domain::language_types::{
    DetectionMatcher, DetectionMatcherKind, LanguageId, LiteralDisposition, LiteralProjection,
    LiteralProjectionDisposition, LiteralReference, MatcherWinner, ScanFamilyDisposition,
    StructuralLanguageSupport,
};
use enforcer_domain::paths::RelPath;
use enforcer_domain::scan_types::{LanguageFamily, RulePack};
use enforcer_syntax::registry::{
    collision_resolutions, detection_precedence, language_registry, literal_projections,
    CanonicalLanguageName,
};

/// Capability disposition attached to a recognized identity by P1B.
///
/// P1B does not add validator bindings. A structural identity therefore stays
/// explicitly unsupported until a later consumer packet proves a binding,
/// while a no-parse identity is not applicable to structural routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteCapabilityDisposition {
    /// The identity is recognized but no validator binding is claimed here.
    Unsupported,
    /// The canonical parser identity intentionally has no structural parser.
    NotApplicable,
}

/// State of a consumer projection for one canonical identity.
///
/// `Unsupported` means this packet has no mechanically proved canonical
/// identity projection for that consumer. It does not claim that the legacy
/// coarse route lacks the capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumerCapabilityState {
    /// No canonical-identity projection is proved by this packet.
    Unsupported,
    /// The consumer question does not apply to this identity projection.
    NotApplicable,
}

/// Typed canonical-identity projection onto the existing CLI language route.
///
/// `Mapped` identifies only an existing CLI route-language value. It does not
/// prove that the CLI command executes a validator or that its architecture
/// checks cover the canonical identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliLanguageProjection {
    /// An existing CLI route-language value is mechanically mapped.
    Mapped(CliLanguage),
    /// No canonical-identity CLI projection is proved.
    Unsupported,
    /// The CLI projection question does not apply to this identity.
    NotApplicable,
}

/// Closed CLI route-language values already accepted by the architecture
/// command boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliLanguage {
    /// The existing `rust` architecture route value.
    Rust,
    /// The existing `typescript` architecture route value.
    TypeScript,
}

/// Typed native-tool identity projection for one canonical language.
///
/// `Mapped` means only that the canonical matchers mechanically resolve to an
/// existing scan family with an existing typed native-tool mapping. It does
/// not prove tool execution, configuration, or scan success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeToolProjection {
    /// An existing native-tool identity is mechanically mapped.
    Mapped(NativeTool),
    /// No canonical-identity native-tool mapping is proved.
    Unsupported,
    /// The native-tool question does not apply to this identity projection.
    NotApplicable,
}

/// Typed identity-specific rule-pack projection.
///
/// `Mapped` means only that the existing scan-family route selects these
/// packs for the identity. It does not prove rule coverage, fact availability,
/// execution, or finding correctness. Route-level universal packs are not
/// included in this identity-specific value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulePackProjection {
    /// Existing identity-specific packs selected by the scan-family route.
    Mapped(&'static [RulePack]),
    /// No identity-specific rule-pack mapping is mechanically proved.
    Unsupported,
    /// The rule-pack question does not apply to this identity projection.
    NotApplicable,
}

/// Typed consumer-capability projection attached to a canonical route.
///
/// Native-tool and rule-pack values reuse only exact scan-family mapping
/// seams proved by the current consumers. CLI remains an independent
/// unsupported disposition, while UI is not applicable to an identity-specific
/// projection because no such capability seam exists. Legacy coarse mappings
/// remain unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalConsumerCapabilities {
    native_scan: ScanFamilyDisposition,
    native_tool: NativeToolProjection,
    rule_packs: RulePackProjection,
    cli: CliLanguageProjection,
    ui: ConsumerCapabilityState,
}

impl CanonicalConsumerCapabilities {
    /// Return the native-scan family projection proved by the existing matcher contract.
    #[must_use]
    pub const fn native_scan(&self) -> ScanFamilyDisposition {
        self.native_scan
    }

    /// Return the canonical-identity native-tool projection state.
    #[must_use]
    pub const fn native_tool(&self) -> NativeToolProjection {
        self.native_tool
    }

    /// Return the canonical-identity rule-pack projection state.
    #[must_use]
    pub const fn rule_packs(&self) -> RulePackProjection {
        self.rule_packs
    }

    /// Return the canonical-identity CLI projection state.
    #[must_use]
    pub const fn cli(&self) -> CliLanguageProjection {
        self.cli
    }

    /// Return the canonical-identity UI projection state.
    #[must_use]
    pub const fn ui(&self) -> ConsumerCapabilityState {
        self.ui
    }
}

/// Policy for the explicit unknown-language fallback route.
///
/// This is a routing decision, not a parser or validator capability. The
/// canonical registry remains authoritative for recognized identities even
/// when this policy excludes the supplemental unknown result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownLanguagePolicy {
    /// Do not append an unknown result when no canonical matcher applies.
    Exclude,
    /// Append one unknown result after all canonical matchers are exhausted.
    Include,
}

/// One canonical language identity retained by the scan router.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalLanguageRoute {
    id: LanguageId,
    canonical_name: CanonicalLanguageName,
    structural: StructuralLanguageSupport,
    literal_disposition: LiteralDisposition,
    capability: RouteCapabilityDisposition,
    scan_family_disposition: ScanFamilyDisposition,
    consumer_capabilities: CanonicalConsumerCapabilities,
    matched_by: Option<DetectionMatcher>,
}

impl CanonicalLanguageRoute {
    /// Return the stable canonical parser identity.
    #[must_use]
    pub const fn id(&self) -> LanguageId {
        self.id
    }

    /// Return the canonical name emitted by the reviewed registry.
    #[must_use]
    pub const fn canonical_name(&self) -> CanonicalLanguageName {
        self.canonical_name
    }

    /// Return whether the identity has structural parser support.
    #[must_use]
    pub const fn structural(&self) -> StructuralLanguageSupport {
        self.structural
    }

    /// Return the reviewed registry literal disposition for this identity.
    #[must_use]
    pub const fn literal_disposition(&self) -> LiteralDisposition {
        self.literal_disposition
    }

    /// Return the honest P1B capability disposition.
    #[must_use]
    pub const fn capability(&self) -> RouteCapabilityDisposition {
        self.capability
    }

    /// Return the current extension-based validator-dispatch projection.
    #[must_use]
    pub const fn scan_family_disposition(&self) -> ScanFamilyDisposition {
        self.scan_family_disposition
    }

    /// Return the typed consumer-capability projection.
    #[must_use]
    pub const fn consumer_capabilities(&self) -> CanonicalConsumerCapabilities {
        self.consumer_capabilities
    }

    /// Return the typed matcher that produced this route, when the route came
    /// from path detection. Registry-only projections intentionally have no
    /// path evidence.
    #[must_use]
    pub const fn matched_by(&self) -> Option<DetectionMatcher> {
        self.matched_by
    }
}

/// One identity-preserving result from canonical path detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedLanguageRoute {
    /// A parser identity from the canonical 160-row registry.
    Canonical(CanonicalLanguageRoute),
    /// A named literal projection with no parser identity.
    SupplementalLiteral { name: &'static str },
    /// No canonical or supplemental matcher matched the path.
    Unknown,
}

/// Detect unique language identities from repo-relative paths.
///
/// Matching uses only typed registry matchers and the reviewed precedence
/// projection. Unknown is included at most once and only when
/// [`UnknownLanguagePolicy::Include`] is selected.
pub fn detect_language_identities(
    paths: &[RelPath],
    unknown_policy: UnknownLanguagePolicy,
) -> Vec<DetectedLanguageRoute> {
    let mut canonical = BTreeMap::new();
    let mut supplemental = BTreeSet::new();
    let mut found_unknown = false;

    for path in paths {
        match matched_reference(path) {
            Some((LiteralReference::ParserId(id), matcher)) => {
                canonical
                    .entry(id)
                    .and_modify(|incumbent| {
                        if matches!(
                            compare_evidence_matchers(matcher, *incumbent),
                            MatcherSelection::Replace
                        ) {
                            *incumbent = matcher;
                        }
                    })
                    .or_insert(matcher);
            }
            Some((LiteralReference::SupplementalLiteralName(name), _)) => {
                supplemental.insert(name);
            }
            Some((LiteralReference::Fallback, _)) | None
                if unknown_policy == UnknownLanguagePolicy::Include =>
            {
                found_unknown = true;
            }
            Some((LiteralReference::Fallback, _)) | None => {}
        }
    }

    let mut routes = canonical
        .into_iter()
        .filter_map(|(id, matcher)| canonical_route_with_match(id, Some(matcher)))
        .map(DetectedLanguageRoute::Canonical)
        .collect::<Vec<_>>();
    routes.extend(
        supplemental
            .into_iter()
            .map(|name| DetectedLanguageRoute::SupplementalLiteral { name }),
    );
    if found_unknown {
        routes.push(DetectedLanguageRoute::Unknown);
    }
    routes
}

fn canonical_route(id: LanguageId) -> Option<CanonicalLanguageRoute> {
    canonical_route_with_match(id, None)
}

fn canonical_route_with_match(
    id: LanguageId,
    matched_by: Option<DetectionMatcher>,
) -> Option<CanonicalLanguageRoute> {
    let record = language_registry()
        .iter()
        .find(|record| record.id() == id)?;
    let capability = match record.structural() {
        StructuralLanguageSupport::ParseFile => RouteCapabilityDisposition::Unsupported,
        StructuralLanguageSupport::NoParseFile => RouteCapabilityDisposition::NotApplicable,
    };
    Some(CanonicalLanguageRoute {
        id,
        canonical_name: record.canonical_name(),
        structural: record.structural(),
        literal_disposition: record.literal_disposition(),
        capability,
        scan_family_disposition: scan_family_disposition(record.matchers()),
        consumer_capabilities: consumer_capabilities_for_matchers(record.matchers()),
        matched_by,
    })
}

pub(crate) fn canonical_scan_family_disposition(id: LanguageId) -> Option<ScanFamilyDisposition> {
    canonical_route(id).map(|route| route.scan_family_disposition())
}

pub(crate) fn canonical_literal_disposition(id: LanguageId) -> Option<LiteralDisposition> {
    canonical_route(id).map(|route| route.literal_disposition())
}

pub(crate) fn canonical_consumer_capabilities(
    id: LanguageId,
) -> Option<CanonicalConsumerCapabilities> {
    canonical_route(id).map(|route| route.consumer_capabilities())
}

fn scan_family_disposition(matchers: &[DetectionMatcher]) -> ScanFamilyDisposition {
    let mut mapped = None;
    for matcher in matchers.iter().copied() {
        let Ok(Some(family)) = scan_family_for_matcher(matcher) else {
            return ScanFamilyDisposition::Unsupported;
        };
        if mapped.is_some_and(|mapped| mapped != family) {
            return ScanFamilyDisposition::Unsupported;
        }
        mapped = Some(family);
    }
    mapped.map_or(
        ScanFamilyDisposition::Unsupported,
        ScanFamilyDisposition::Mapped,
    )
}

fn consumer_capabilities_for_matchers(
    matchers: &[DetectionMatcher],
) -> CanonicalConsumerCapabilities {
    CanonicalConsumerCapabilities {
        native_scan: scan_family_disposition(matchers),
        native_tool: native_tool_projection(matchers),
        rule_packs: rule_pack_projection(matchers),
        cli: cli_language_projection(matchers),
        ui: ui_capability_projection(),
    }
}

/// Return the honest canonical UI projection for every registry identity.
///
/// The current UI surface is an identity-agnostic report/presentation
/// consumer: no identity-specific UI capability seam exists in `enforcer-ui`.
/// `NotApplicable` therefore describes this projection question only; it does
/// not claim that the UI lacks report functionality, and it must not be
/// inferred from parser, literal, native-tool, rule-pack, or CLI support.
const fn ui_capability_projection() -> ConsumerCapabilityState {
    ConsumerCapabilityState::NotApplicable
}

fn cli_language_projection(matchers: &[DetectionMatcher]) -> CliLanguageProjection {
    match scan_family_disposition(matchers) {
        ScanFamilyDisposition::Mapped(LanguageFamily::Rust) => {
            CliLanguageProjection::Mapped(CliLanguage::Rust)
        }
        ScanFamilyDisposition::Mapped(LanguageFamily::TypeScript) => {
            CliLanguageProjection::Mapped(CliLanguage::TypeScript)
        }
        ScanFamilyDisposition::Mapped(_) | ScanFamilyDisposition::Unsupported => {
            CliLanguageProjection::Unsupported
        }
        ScanFamilyDisposition::NotApplicable => CliLanguageProjection::NotApplicable,
    }
}

fn rule_pack_projection(matchers: &[DetectionMatcher]) -> RulePackProjection {
    match scan_family_disposition(matchers) {
        ScanFamilyDisposition::Mapped(family) => {
            let packs = super::plan::rule_packs_for_scan_family(family);
            if packs.is_empty() {
                RulePackProjection::Unsupported
            } else {
                RulePackProjection::Mapped(packs)
            }
        }
        ScanFamilyDisposition::Unsupported | ScanFamilyDisposition::NotApplicable => {
            RulePackProjection::Unsupported
        }
    }
}

fn native_tool_projection(matchers: &[DetectionMatcher]) -> NativeToolProjection {
    match scan_family_disposition(matchers) {
        ScanFamilyDisposition::Mapped(family) => {
            super::native_tie::native_tool_for_scan_family(family).map_or(
                NativeToolProjection::Unsupported,
                NativeToolProjection::Mapped,
            )
        }
        ScanFamilyDisposition::Unsupported | ScanFamilyDisposition::NotApplicable => {
            NativeToolProjection::Unsupported
        }
    }
}

fn scan_family_for_matcher(
    matcher: DetectionMatcher,
) -> Result<Option<LanguageFamily>, DecodeError> {
    let DetectionMatcher::Extension(extension) = matcher else {
        return Ok(None);
    };
    let path = RelPath::from_str(&format!("__ul06_scan_family.{extension}"))?;
    match super::classify(&path) {
        LanguageFamily::Unknown => Ok(None),
        family => Ok(Some(family)),
    }
}

fn matched_reference(path: &RelPath) -> Option<(LiteralReference, DetectionMatcher)> {
    for kind in detection_precedence().ordered_kinds() {
        let mut best: Option<(DetectionMatcher, LiteralReference)> = None;

        for record in language_registry() {
            for matcher in record.matchers().iter().copied() {
                if matcher_kind(matcher) != *kind {
                    continue;
                }
                let Some(matched_matcher) = matching_matcher(matcher, path) else {
                    continue;
                };
                let reference =
                    collision_winner(matcher).unwrap_or(LiteralReference::ParserId(record.id()));
                if best.is_none_or(|(best_matcher, _)| {
                    matches!(
                        compare_matchers(matched_matcher, best_matcher),
                        MatcherSelection::Replace
                    )
                }) {
                    best = Some((matched_matcher, reference));
                }
            }
        }

        for projection in literal_projections() {
            let LiteralProjection::Row(_, disposition, parser_ids, matchers, winners) = projection;
            if *disposition == LiteralProjectionDisposition::Fallback {
                continue;
            }
            for matcher in matchers.iter().copied() {
                if matcher_kind(matcher) != *kind {
                    continue;
                }
                let Some(matched_matcher) = matching_matcher(matcher, path) else {
                    continue;
                };
                let reference = collision_winner(matcher)
                    .or_else(|| matcher_winner(matcher, winners))
                    .or_else(|| match parser_ids {
                        [id] => Some(LiteralReference::ParserId(*id)),
                        _ => None,
                    });
                let Some(reference) = reference else {
                    continue;
                };
                if best.is_none_or(|(best_matcher, _)| {
                    matches!(
                        compare_matchers(matched_matcher, best_matcher),
                        MatcherSelection::Replace
                    )
                }) {
                    best = Some((matched_matcher, reference));
                }
            }
        }

        if let Some((matcher, reference)) = best {
            return Some((reference, matcher));
        }
    }
    None
}

fn matcher_kind(matcher: DetectionMatcher) -> DetectionMatcherKind {
    match matcher {
        DetectionMatcher::Extension(_) => DetectionMatcherKind::Extension,
        DetectionMatcher::ExactBasename(_) => DetectionMatcherKind::ExactBasename,
        DetectionMatcher::CompoundSuffix(_) => DetectionMatcherKind::CompoundSuffix,
    }
}

fn matching_matcher(matcher: DetectionMatcher, path: &RelPath) -> Option<DetectionMatcher> {
    let path_text = path.as_str();
    let basename = path_text.rsplit('/').next().unwrap_or(path_text);
    let extension = basename.rsplit('.').next().unwrap_or("");
    match matcher {
        DetectionMatcher::Extension(value) => {
            extension.eq_ignore_ascii_case(value).then_some(matcher)
        }
        DetectionMatcher::ExactBasename(value) => {
            basename.eq_ignore_ascii_case(value).then_some(matcher)
        }
        DetectionMatcher::CompoundSuffix(value) => basename
            .to_ascii_lowercase()
            .ends_with(&value.to_ascii_lowercase())
            .then_some(matcher),
    }
}

enum MatcherSelection {
    Keep,
    Replace,
}

fn compare_matchers(candidate: DetectionMatcher, incumbent: DetectionMatcher) -> MatcherSelection {
    match (candidate, incumbent) {
        (
            DetectionMatcher::CompoundSuffix(candidate),
            DetectionMatcher::CompoundSuffix(incumbent),
        ) => {
            if candidate.len() > incumbent.len() {
                MatcherSelection::Replace
            } else {
                MatcherSelection::Keep
            }
        }
        _ => MatcherSelection::Keep,
    }
}

fn compare_evidence_matchers(
    candidate: DetectionMatcher,
    incumbent: DetectionMatcher,
) -> MatcherSelection {
    let precedence = detection_precedence().ordered_kinds();
    let candidate_kind = matcher_kind(candidate);
    let incumbent_kind = matcher_kind(incumbent);
    let candidate_rank = precedence
        .iter()
        .position(|kind| *kind == candidate_kind)
        .unwrap_or(precedence.len());
    let incumbent_rank = precedence
        .iter()
        .position(|kind| *kind == incumbent_kind)
        .unwrap_or(precedence.len());
    if candidate_rank != incumbent_rank {
        return if candidate_rank < incumbent_rank {
            MatcherSelection::Replace
        } else {
            MatcherSelection::Keep
        };
    }
    if let (
        DetectionMatcher::CompoundSuffix(candidate),
        DetectionMatcher::CompoundSuffix(incumbent),
    ) = (candidate, incumbent)
    {
        return if candidate.len() != incumbent.len() {
            if candidate.len() > incumbent.len() {
                MatcherSelection::Replace
            } else {
                MatcherSelection::Keep
            }
        } else if candidate < incumbent {
            MatcherSelection::Replace
        } else {
            MatcherSelection::Keep
        };
    }
    let candidate_key = matcher_value(candidate);
    let incumbent_key = matcher_value(incumbent);
    if candidate_key < incumbent_key {
        MatcherSelection::Replace
    } else {
        MatcherSelection::Keep
    }
}

fn matcher_value(matcher: DetectionMatcher) -> &'static str {
    match matcher {
        DetectionMatcher::Extension(value)
        | DetectionMatcher::ExactBasename(value)
        | DetectionMatcher::CompoundSuffix(value) => value,
    }
}

fn collision_winner(matcher: DetectionMatcher) -> Option<LiteralReference> {
    let key = match matcher {
        DetectionMatcher::Extension(value)
        | DetectionMatcher::ExactBasename(value)
        | DetectionMatcher::CompoundSuffix(value) => value.to_ascii_lowercase(),
    };
    let kind = matcher_kind(matcher);
    collision_resolutions()
        .iter()
        .find_map(|resolution| match resolution {
            enforcer_domain::language_types::CollisionResolution::Group(
                resolution_kind,
                resolution_key,
                _,
                winner,
            ) if *resolution_kind == kind && *resolution_key == key => Some(*winner),
            _ => None,
        })
}

fn matcher_winner(
    matcher: DetectionMatcher,
    winners: &[MatcherWinner],
) -> Option<LiteralReference> {
    let key = match matcher {
        DetectionMatcher::Extension(value) => format!("extension:{}", value.to_ascii_lowercase()),
        DetectionMatcher::ExactBasename(value) => {
            format!("exactBasename:{}", value.to_ascii_lowercase())
        }
        DetectionMatcher::CompoundSuffix(value) => {
            format!("compoundSuffix:{}", value.to_ascii_lowercase())
        }
    };
    winners.iter().find_map(|winner| match winner {
        MatcherWinner::Key(winner_key, reference) if *winner_key == key => Some(*reference),
        _ => None,
    })
}

#[cfg(test)]
mod scan_family_tests {
    use super::{scan_family_disposition, ScanFamilyDisposition};
    use enforcer_domain::language_types::DetectionMatcher;
    use enforcer_domain::scan_types::LanguageFamily;
    use enforcer_syntax::parsers::Language;
    use enforcer_syntax::registry::language_registry;

    #[test]
    fn scan_family_projection_preserves_the_160_row_denominator() {
        let records = language_registry();
        assert_eq!(records.len(), 160);

        let mut rust = 0;
        let mut typescript = 0;
        let mut python = 0;
        let mut terraform = 0;
        let mut yaml_or_config = 0;
        let mut unknown = 0;
        let mut unsupported = 0;
        let mut not_applicable = 0;

        for record in records {
            match scan_family_disposition(record.matchers()) {
                ScanFamilyDisposition::Mapped(LanguageFamily::Rust) => rust += 1,
                ScanFamilyDisposition::Mapped(LanguageFamily::TypeScript) => typescript += 1,
                ScanFamilyDisposition::Mapped(LanguageFamily::Python) => python += 1,
                ScanFamilyDisposition::Mapped(LanguageFamily::Terraform) => terraform += 1,
                ScanFamilyDisposition::Mapped(LanguageFamily::YamlOrConfig) => yaml_or_config += 1,
                ScanFamilyDisposition::Mapped(LanguageFamily::Unknown) => unknown += 1,
                ScanFamilyDisposition::Unsupported => unsupported += 1,
                ScanFamilyDisposition::NotApplicable => not_applicable += 1,
            }
        }

        assert_eq!(
            (rust, typescript, python, terraform, yaml_or_config),
            (1, 1, 0, 0, 1)
        );
        assert_eq!(unsupported, 157);
        assert_eq!(not_applicable, 0);
        assert_eq!(unknown, 0);
    }

    #[test]
    fn scan_family_projection_fails_closed_for_ambiguous_matchers() {
        assert_eq!(
            scan_family_disposition(&[]),
            ScanFamilyDisposition::Unsupported
        );
        assert_eq!(
            scan_family_disposition(&[DetectionMatcher::ExactBasename("Dockerfile")]),
            ScanFamilyDisposition::Unsupported
        );
        assert_eq!(
            scan_family_disposition(&[DetectionMatcher::CompoundSuffix(".env.local")]),
            ScanFamilyDisposition::Unsupported
        );
        assert_eq!(
            scan_family_disposition(&[DetectionMatcher::Extension("unknown")]),
            ScanFamilyDisposition::Unsupported
        );
        assert_eq!(
            scan_family_disposition(&[
                DetectionMatcher::Extension("rs"),
                DetectionMatcher::Extension("py"),
            ]),
            ScanFamilyDisposition::Unsupported
        );
    }

    #[test]
    fn config_json_and_yaml_remain_unsupported_without_canonical_matchers() {
        for parser in [Language::ConfigJson, Language::ConfigYaml] {
            let record = language_registry()
                .iter()
                .find(|record| record.parser() == parser);
            assert_eq!(
                record.map(|record| record.parser()),
                Some(parser),
                "missing reviewed parser identity"
            );
            let Some(record) = record else {
                continue;
            };
            assert!(record.matchers().is_empty());
            assert_eq!(
                scan_family_disposition(record.matchers()),
                ScanFamilyDisposition::Unsupported
            );
        }
    }
}

#[cfg(test)]
mod native_tool_tests {
    use super::{native_tool_projection, NativeToolProjection};
    use enforcer_domain::config_types::NativeTool;
    use enforcer_domain::language_types::DetectionMatcher;
    use enforcer_syntax::registry::language_registry;

    #[test]
    fn native_tool_projection_preserves_the_160_row_denominator() {
        let records = language_registry();
        assert_eq!(records.len(), 160);

        let mut cargo = 0;
        let mut tsc = 0;
        let mut unsupported = 0;
        let mut not_applicable = 0;

        for record in records {
            match native_tool_projection(record.matchers()) {
                NativeToolProjection::Mapped(NativeTool::Cargo) => cargo += 1,
                NativeToolProjection::Mapped(NativeTool::Tsc) => tsc += 1,
                NativeToolProjection::Mapped(_) => {
                    assert!(false, "only Cargo and Tsc are mapped in this projection")
                }
                NativeToolProjection::Unsupported => unsupported += 1,
                NativeToolProjection::NotApplicable => not_applicable += 1,
            }
        }

        assert_eq!((cargo, tsc, unsupported, not_applicable), (1, 1, 158, 0));
    }

    #[test]
    fn native_tool_projection_rejects_ambiguous_or_unmapped_matchers() {
        for matchers in [
            vec![],
            vec![DetectionMatcher::ExactBasename("Dockerfile")],
            vec![DetectionMatcher::CompoundSuffix(".env.local")],
            vec![DetectionMatcher::Extension("unknown")],
            vec![
                DetectionMatcher::Extension("rs"),
                DetectionMatcher::Extension("py"),
            ],
        ] {
            assert_eq!(
                native_tool_projection(&matchers),
                NativeToolProjection::Unsupported
            );
        }
    }
}

#[cfg(test)]
mod rule_pack_tests {
    use super::{rule_pack_projection, RulePackProjection};
    use enforcer_domain::language_types::DetectionMatcher;
    use enforcer_domain::scan_types::RulePack;
    use enforcer_syntax::registry::language_registry;

    #[test]
    fn rule_pack_projection_preserves_the_160_row_denominator() {
        let records = language_registry();
        assert_eq!(records.len(), 160);

        let mut mapped = 0;
        let mut unsupported = 0;
        let mut not_applicable = 0;
        for record in records {
            match rule_pack_projection(record.matchers()) {
                RulePackProjection::Mapped(packs) => {
                    mapped += 1;
                    assert!(matches!(
                        packs,
                        [RulePack::Rust, RulePack::Security]
                            | [RulePack::TypeScript, RulePack::Security]
                    ));
                }
                RulePackProjection::Unsupported => unsupported += 1,
                RulePackProjection::NotApplicable => not_applicable += 1,
            }
        }

        assert_eq!((mapped, unsupported, not_applicable), (2, 158, 0));
    }

    #[test]
    fn rule_pack_projection_rejects_ambiguous_or_unmapped_matchers() {
        for matchers in [
            vec![],
            vec![DetectionMatcher::ExactBasename("Dockerfile")],
            vec![DetectionMatcher::CompoundSuffix(".env.local")],
            vec![DetectionMatcher::Extension("yaml")],
            vec![DetectionMatcher::Extension("unknown")],
            vec![
                DetectionMatcher::Extension("rs"),
                DetectionMatcher::Extension("py"),
            ],
        ] {
            assert_eq!(
                rule_pack_projection(&matchers),
                RulePackProjection::Unsupported
            );
        }
    }
}

#[cfg(test)]
mod cli_projection_tests {
    use super::{cli_language_projection, CliLanguage, CliLanguageProjection};
    use enforcer_domain::language_types::DetectionMatcher;
    use enforcer_syntax::registry::language_registry;

    #[test]
    fn cli_projection_preserves_the_160_row_denominator() {
        let records = language_registry();
        assert_eq!(records.len(), 160);

        let mut rust = 0;
        let mut type_script = 0;
        let mut unsupported = 0;
        let mut not_applicable = 0;
        for record in records {
            match cli_language_projection(record.matchers()) {
                CliLanguageProjection::Mapped(CliLanguage::Rust) => rust += 1,
                CliLanguageProjection::Mapped(CliLanguage::TypeScript) => type_script += 1,
                CliLanguageProjection::Unsupported => unsupported += 1,
                CliLanguageProjection::NotApplicable => not_applicable += 1,
            }
        }

        assert_eq!(
            (rust, type_script, unsupported, not_applicable),
            (1, 1, 158, 0)
        );
    }

    #[test]
    fn cli_projection_rejects_unmapped_or_ambiguous_matchers() {
        for matchers in [
            vec![],
            vec![DetectionMatcher::ExactBasename("Dockerfile")],
            vec![DetectionMatcher::CompoundSuffix(".env.local")],
            vec![DetectionMatcher::Extension("yaml")],
            vec![DetectionMatcher::Extension("unknown")],
            vec![
                DetectionMatcher::Extension("rs"),
                DetectionMatcher::Extension("py"),
            ],
        ] {
            assert_eq!(
                cli_language_projection(&matchers),
                CliLanguageProjection::Unsupported
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{canonical_route, RouteCapabilityDisposition};
    use enforcer_domain::language_types::{LiteralDisposition, StructuralLanguageSupport};
    use enforcer_syntax::parsers::Language;
    use enforcer_syntax::registry::language_registry;

    #[test]
    fn every_registry_identity_has_a_typed_route_disposition() {
        assert_eq!(language_registry().len(), 160);
        let ui_mapped = 0;
        let mut ui_unsupported = 0;
        let mut ui_not_applicable = 0;
        for record in language_registry() {
            let Some(route) = canonical_route(record.id()) else {
                assert!(false, "registry id must resolve to a route");
                return;
            };
            let expected = match record.structural() {
                StructuralLanguageSupport::ParseFile => RouteCapabilityDisposition::Unsupported,
                StructuralLanguageSupport::NoParseFile => RouteCapabilityDisposition::NotApplicable,
            };
            assert_eq!(route.capability(), expected);
            assert_eq!(route.canonical_name(), record.canonical_name());
            assert_eq!(
                route.consumer_capabilities().native_tool(),
                match record.id().as_nonzero_u16().get() {
                    1 => super::NativeToolProjection::Mapped(
                        enforcer_domain::config_types::NativeTool::Cargo,
                    ),
                    3 => super::NativeToolProjection::Mapped(
                        enforcer_domain::config_types::NativeTool::Tsc,
                    ),
                    _ => super::NativeToolProjection::Unsupported,
                }
            );
            assert_eq!(
                route.consumer_capabilities().rule_packs(),
                match record.id().as_nonzero_u16().get() {
                    1 => super::RulePackProjection::Mapped(&[
                        enforcer_domain::scan_types::RulePack::Rust,
                        enforcer_domain::scan_types::RulePack::Security,
                    ]),
                    3 => super::RulePackProjection::Mapped(&[
                        enforcer_domain::scan_types::RulePack::TypeScript,
                        enforcer_domain::scan_types::RulePack::Security,
                    ]),
                    _ => super::RulePackProjection::Unsupported,
                }
            );
            assert_eq!(
                route.consumer_capabilities().cli(),
                match record.id().as_nonzero_u16().get() {
                    1 => super::CliLanguageProjection::Mapped(super::CliLanguage::Rust),
                    3 => super::CliLanguageProjection::Mapped(super::CliLanguage::TypeScript),
                    _ => super::CliLanguageProjection::Unsupported,
                }
            );
            assert_eq!(
                route.consumer_capabilities().ui(),
                super::ConsumerCapabilityState::NotApplicable
            );
            match route.consumer_capabilities().ui() {
                super::ConsumerCapabilityState::Unsupported => ui_unsupported += 1,
                super::ConsumerCapabilityState::NotApplicable => ui_not_applicable += 1,
            }
        }
        assert_eq!((ui_mapped, ui_unsupported, ui_not_applicable), (0, 0, 160));
    }

    #[test]
    fn ui_projection_is_not_applicable_across_identity_and_literal_shapes() -> Result<(), String> {
        for parser in [
            Language::Rust,
            Language::JavaScript,
            Language::Python,
            Language::Yaml,
            Language::ConfigJson,
            Language::ConfigYaml,
        ] {
            let record = language_registry()
                .iter()
                .find(|record| record.parser() == parser)
                .ok_or_else(|| "reviewed parser identity must exist".to_owned())?;
            let route = canonical_route(record.id())
                .ok_or_else(|| "registry identity must route".to_owned())?;
            assert_eq!(
                route.consumer_capabilities().ui(),
                super::ConsumerCapabilityState::NotApplicable
            );
        }

        let unmatched = language_registry()
            .iter()
            .find(|record| {
                record.structural() == StructuralLanguageSupport::ParseFile
                    && record.literal_disposition() == LiteralDisposition::Unsupported
            })
            .ok_or_else(|| {
                "an unmatched structural identity must remain in the registry".to_owned()
            })?;
        let route = canonical_route(unmatched.id())
            .ok_or_else(|| "unmatched identity must route".to_owned())?;
        assert_eq!(
            route.consumer_capabilities().ui(),
            super::ConsumerCapabilityState::NotApplicable
        );
        Ok(())
    }
}
