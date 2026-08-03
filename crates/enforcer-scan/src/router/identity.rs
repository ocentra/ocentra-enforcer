//! Identity-preserving language detection for the scan router.
//!
//! The syntax registry is the only source of parser identity and matcher
//! metadata. This module projects that metadata into scan-owned route values;
//! it does not claim parser, validator, or native-tool capability.

use std::collections::BTreeSet;
use std::str::FromStr;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::language_types::{
    DetectionMatcher, DetectionMatcherKind, LanguageId, LiteralProjection,
    LiteralProjectionDisposition, LiteralReference, MatcherWinner, ScanFamilyDisposition,
    StructuralLanguageSupport,
};
use enforcer_domain::paths::RelPath;
use enforcer_domain::scan_types::LanguageFamily;
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
    capability: RouteCapabilityDisposition,
    scan_family_disposition: ScanFamilyDisposition,
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
    let mut canonical = BTreeSet::new();
    let mut supplemental = BTreeSet::new();
    let mut found_unknown = false;

    for path in paths {
        match matched_reference(path) {
            Some(LiteralReference::ParserId(id)) => {
                canonical.insert(id);
            }
            Some(LiteralReference::SupplementalLiteralName(name)) => {
                supplemental.insert(name);
            }
            Some(LiteralReference::Fallback) | None
                if unknown_policy == UnknownLanguagePolicy::Include =>
            {
                found_unknown = true;
            }
            Some(LiteralReference::Fallback) | None => {}
        }
    }

    let mut routes = canonical
        .into_iter()
        .filter_map(canonical_route)
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
        capability,
        scan_family_disposition: scan_family_disposition(record.matchers()),
    })
}

pub(crate) fn canonical_scan_family_disposition(id: LanguageId) -> Option<ScanFamilyDisposition> {
    canonical_route(id).map(|route| route.scan_family_disposition())
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

fn matched_reference(path: &RelPath) -> Option<LiteralReference> {
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

        if let Some((_, reference)) = best {
            return Some(reference);
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
mod tests {
    use super::{canonical_route, RouteCapabilityDisposition};
    use enforcer_domain::language_types::StructuralLanguageSupport;
    use enforcer_syntax::registry::language_registry;

    #[test]
    fn every_registry_identity_has_a_typed_route_disposition() {
        assert_eq!(language_registry().len(), 160);
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
        }
    }
}
