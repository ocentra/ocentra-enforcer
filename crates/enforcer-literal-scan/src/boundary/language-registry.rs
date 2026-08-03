//! Canonical literal routing adapter.
//!
//! The syntax registry owns language identities, matcher values, collision
//! winners, and matcher precedence. This module owns only the literal scanner
//! lexical-family and string-syntax overlay.
//! BOUNDARY-INVARIANT: detection consumes only typed canonical matchers and
//! returns an explicit unknown fallback when the caller enables it.
//! Negative invalid-input coverage rejects unmatched paths when that fallback
//! is disabled and preserves known matches ahead of fallback routing.

use super::{LanguageFamily, LanguageSpec};
use enforcer_domain::language_types::{
    DetectionMatcher, DetectionMatcherKind, LiteralProjection, LiteralProjectionDisposition,
    LiteralReference, MatcherWinner,
};
use enforcer_domain::scan_types::{
    LiteralBasenameSet, LiteralExtensionSet, LiteralLanguageName, LiteralStringSyntaxProfile,
};
use enforcer_syntax::registry::{collision_resolutions, detection_precedence, literal_projections};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LexicalProfile {
    name: &'static str,
    family: LanguageFamily,
    syntax: LiteralStringSyntaxProfile,
}

macro_rules! profile {
    ($name:literal, $family:expr, $single:expr, $backtick:expr, $triple:expr $(,)?) => {
        LexicalProfile {
            name: $name,
            family: $family,
            syntax: LiteralStringSyntaxProfile::from_bits(
                (if $single {
                    LiteralStringSyntaxProfile::SINGLE_QUOTE
                } else {
                    0
                }) | (if $backtick {
                    LiteralStringSyntaxProfile::BACKTICK
                } else {
                    0
                }) | (if $triple {
                    LiteralStringSyntaxProfile::TRIPLE_DOUBLE
                } else {
                    0
                }),
            ),
        }
    };
}

// This is deliberately limited to consumer lexical behavior. All names are
// keys into the canonical literalProjection rows; no matcher or identity data
// is repeated here.
const LEXICAL_PROFILES: &[LexicalProfile] = &[
    profile!("rust", LanguageFamily::Rust, false, false, false),
    profile!("typescript", LanguageFamily::TypeScript, true, true, false),
    profile!("javascript", LanguageFamily::TypeScript, true, true, false),
    profile!("python", LanguageFamily::Python, true, false, true),
    profile!("c", LanguageFamily::CLike, false, false, false),
    profile!("cpp", LanguageFamily::CLike, false, false, false),
    profile!("csharp", LanguageFamily::CLike, true, false, true),
    profile!("objective-c", LanguageFamily::CLike, false, false, false),
    profile!("zig", LanguageFamily::CLike, false, false, false),
    profile!("go", LanguageFamily::CLike, false, true, false),
    profile!("d", LanguageFamily::CLike, false, true, false),
    profile!("v", LanguageFamily::CLike, false, true, false),
    profile!("nim", LanguageFamily::HashComment, true, true, true),
    profile!("java", LanguageFamily::CLike, false, false, true),
    profile!("kotlin", LanguageFamily::CLike, true, false, true),
    profile!("scala", LanguageFamily::CLike, true, false, true),
    profile!("groovy", LanguageFamily::CLike, true, true, true),
    profile!("swift", LanguageFamily::CLike, false, false, true),
    profile!("dart", LanguageFamily::CLike, true, true, true),
    profile!("php", LanguageFamily::CLike, true, true, false),
    profile!("ruby", LanguageFamily::HashComment, true, true, true),
    profile!("perl", LanguageFamily::HashComment, true, true, false),
    profile!("lua", LanguageFamily::HashComment, true, false, true),
    profile!("r", LanguageFamily::HashComment, true, true, false),
    profile!("julia", LanguageFamily::HashComment, true, false, true),
    profile!("shell", LanguageFamily::Shell, true, true, false),
    profile!("powershell", LanguageFamily::Shell, true, true, true),
    profile!("batch", LanguageFamily::Shell, true, false, false),
    profile!("make", LanguageFamily::Shell, true, true, false),
    profile!("dockerfile", LanguageFamily::Shell, true, true, false),
    profile!("haskell", LanguageFamily::CLike, false, false, false),
    profile!("ocaml", LanguageFamily::CLike, true, false, false),
    profile!("fsharp", LanguageFamily::CLike, true, true, true),
    profile!("elm", LanguageFamily::CLike, false, false, false),
    profile!("purescript", LanguageFamily::CLike, false, false, false),
    profile!("elixir", LanguageFamily::HashComment, true, true, true),
    profile!("erlang", LanguageFamily::CLike, true, false, false),
    profile!("clojure", LanguageFamily::Lisp, false, false, false),
    profile!("lisp", LanguageFamily::Lisp, false, false, false),
    profile!("sql", LanguageFamily::Sql, true, false, false),
    profile!("graphql", LanguageFamily::CommonText, true, false, false),
    profile!("terraform", LanguageFamily::CommonText, true, false, false),
    profile!("nix", LanguageFamily::CLike, true, false, true),
    profile!("starlark", LanguageFamily::HashComment, true, false, true),
    profile!("protobuf", LanguageFamily::CLike, true, false, false),
    profile!("thrift", LanguageFamily::CLike, true, false, false),
    profile!("solidity", LanguageFamily::CLike, true, false, false),
    profile!("move", LanguageFamily::CLike, true, false, false),
    profile!("apex", LanguageFamily::CLike, true, false, false),
    profile!("qml", LanguageFamily::CLike, true, false, false),
    profile!("cuda", LanguageFamily::CLike, false, false, false),
    profile!("shader", LanguageFamily::CLike, true, false, false),
    profile!("raku", LanguageFamily::HashComment, true, true, true),
    profile!("reason", LanguageFamily::CLike, true, false, false),
    profile!("rescript", LanguageFamily::CLike, true, false, false),
    profile!("sml", LanguageFamily::CLike, true, false, false),
    profile!("avro", LanguageFamily::CommonText, true, false, false),
    profile!("html", LanguageFamily::Markup, true, true, false),
    profile!("css", LanguageFamily::CommonText, true, false, false),
    profile!("json", LanguageFamily::CommonText, true, false, false),
    profile!("yaml", LanguageFamily::CommonText, true, false, false),
    profile!("toml", LanguageFamily::CommonText, true, false, false),
    profile!("env", LanguageFamily::CommonText, true, false, false),
    profile!("markdown", LanguageFamily::CommonText, false, false, false),
    profile!("xml", LanguageFamily::CommonText, true, false, false),
    profile!("csv", LanguageFamily::CommonText, false, false, false),
    profile!("coldfusion", LanguageFamily::Markup, true, false, false),
    profile!("unknown", LanguageFamily::Fallback, true, true, true),
];

static LANGUAGE_REGISTRY: OnceLock<Vec<LanguageSpec>> = OnceLock::new();

fn lexical_profile(name: &str) -> Option<LexicalProfile> {
    LEXICAL_PROFILES
        .iter()
        .copied()
        .find(|profile| profile.name == name)
}

fn matcher_kind(matcher: DetectionMatcher) -> DetectionMatcherKind {
    match matcher {
        DetectionMatcher::Extension(_) => DetectionMatcherKind::Extension,
        DetectionMatcher::ExactBasename(_) => DetectionMatcherKind::ExactBasename,
        DetectionMatcher::CompoundSuffix(_) => DetectionMatcherKind::CompoundSuffix,
    }
}

fn matcher_value(matcher: DetectionMatcher) -> &'static str {
    match matcher {
        DetectionMatcher::Extension(value)
        | DetectionMatcher::ExactBasename(value)
        | DetectionMatcher::CompoundSuffix(value) => value,
    }
}

fn normalized_matcher_key(matcher: DetectionMatcher) -> String {
    let kind = match matcher_kind(matcher) {
        DetectionMatcherKind::Extension => "extension",
        DetectionMatcherKind::ExactBasename => "exactBasename",
        DetectionMatcherKind::CompoundSuffix => "compoundSuffix",
    };
    format!("{kind}:{}", matcher_value(matcher).to_ascii_lowercase())
}

fn matcher_winner(row: &LiteralProjection, matcher: DetectionMatcher) -> Option<LiteralReference> {
    let key = normalized_matcher_key(matcher);
    let LiteralProjection::Row(_, _, _, _, winners) = row;
    winners.iter().find_map(|winner| match winner {
        MatcherWinner::Key(winner_key, reference) if *winner_key == key => Some(*reference),
        _ => None,
    })
}

fn collision_winner(matcher: DetectionMatcher) -> Option<LiteralReference> {
    let key = normalized_matcher_key(matcher);
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

fn projection_for_reference(reference: LiteralReference) -> Option<&'static LiteralProjection> {
    literal_projections().iter().find(|projection| {
        let LiteralProjection::Row(name, _, parser_ids, _, _) = projection;
        match reference {
            LiteralReference::ParserId(id) => parser_ids.contains(&id),
            LiteralReference::SupplementalLiteralName(literal_name) => *name == literal_name,
            LiteralReference::Fallback => *name == "unknown",
        }
    })
}

fn path_matches(matcher: DetectionMatcher, basename: &str, extension: &str) -> bool {
    match matcher {
        DetectionMatcher::Extension(value) => extension.eq_ignore_ascii_case(value),
        DetectionMatcher::ExactBasename(value) => basename.eq_ignore_ascii_case(value),
        DetectionMatcher::CompoundSuffix(value) => basename
            .to_ascii_lowercase()
            .ends_with(&value.to_ascii_lowercase()),
    }
}

pub(crate) fn matching_length(
    matcher: DetectionMatcher,
    basename: &str,
    extension: &str,
) -> Option<usize> {
    if !path_matches(matcher, basename, extension) {
        return None;
    }
    Some(
        if matcher_kind(matcher) == DetectionMatcherKind::CompoundSuffix {
            matcher_value(matcher).len()
        } else {
            0
        },
    )
}

pub(crate) fn matched_projection(path: &Path) -> Option<&'static LiteralProjection> {
    let basename = path.file_name().and_then(OsStr::to_str).unwrap_or("");
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or("");
    for kind in detection_precedence().ordered_kinds() {
        let mut best: Option<(usize, &'static LiteralProjection)> = None;
        for projection in literal_projections() {
            let LiteralProjection::Row(_, _, _, matchers, _) = projection;
            for matcher in matchers.iter().copied() {
                if matcher_kind(matcher) != *kind {
                    continue;
                }
                let Some(length) = matching_length(matcher, basename, extension) else {
                    continue;
                };
                let reference =
                    collision_winner(matcher).or_else(|| matcher_winner(projection, matcher));
                let Some(reference) = reference else { continue };
                let target = match (projection, reference) {
                    (
                        LiteralProjection::Row(_, _, parser_ids, _, _),
                        LiteralReference::ParserId(id),
                    ) if parser_ids.contains(&id) => projection,
                    _ => {
                        let Some(target) = projection_for_reference(reference) else {
                            continue;
                        };
                        target
                    }
                };
                if best.map_or(true, |(best_length, _)| length > best_length) {
                    best = Some((length, target));
                }
            }
        }
        if let Some((_, projection)) = best {
            return Some(projection);
        }
    }
    None
}

fn build_registry() -> Vec<LanguageSpec> {
    assert!(
        profile_overlay_is_exhaustive(),
        "lexical profile overlay must cover every canonical literal row"
    );
    let mut registry = Vec::with_capacity(LEXICAL_PROFILES.len() - 1);
    for projection in literal_projections() {
        let LiteralProjection::Row(name, disposition, _, matchers, _) = projection;
        if *disposition == LiteralProjectionDisposition::Fallback {
            continue;
        }
        let profile = match lexical_profile(name) {
            Some(profile) => profile,
            None => std::process::abort(),
        };
        let mut extensions = Vec::new();
        let mut basenames = Vec::new();
        for matcher in matchers.iter().copied() {
            match matcher {
                DetectionMatcher::Extension(value) => extensions.push(value),
                DetectionMatcher::ExactBasename(value) => basenames.push(value),
                DetectionMatcher::CompoundSuffix(_) => {}
            }
        }
        // LEAK-JUSTIFICATION: the public scanner spec requires static slices;
        // this allocates each canonical projection once inside OnceLock.
        let extensions: &'static [&'static str] = Box::leak(extensions.into_boxed_slice());
        let basenames: &'static [&'static str] = Box::leak(basenames.into_boxed_slice());
        registry.push(LanguageSpec {
            id: LiteralLanguageName::from_static(name),
            family: profile.family,
            extensions: LiteralExtensionSet::from_static(extensions),
            basenames: LiteralBasenameSet::from_static(basenames),
            syntax: profile.syntax,
        });
    }
    registry
}

/// Return the 67 named canonical literal projections as scanner specs.
pub fn language_registry() -> Vec<LanguageSpec> {
    LANGUAGE_REGISTRY.get_or_init(build_registry).clone()
}

pub(crate) fn profile_overlay_is_exhaustive() -> bool {
    let names = LEXICAL_PROFILES
        .iter()
        .map(|profile| profile.name)
        .collect::<HashSet<_>>();
    names.len() == LEXICAL_PROFILES.len()
        && LEXICAL_PROFILES.len() == literal_projections().len()
        && literal_projections().iter().all(|projection| {
            let LiteralProjection::Row(name, _, _, _, _) = projection;
            names.contains(name)
        })
}

fn unknown_spec() -> LanguageSpec {
    let profile = match lexical_profile("unknown") {
        Some(profile) => profile,
        None => std::process::abort(),
    };
    LanguageSpec {
        id: LiteralLanguageName::from_static("unknown"),
        family: profile.family,
        extensions: LiteralExtensionSet::from_static(&[]),
        basenames: LiteralBasenameSet::from_static(&[]),
        syntax: profile.syntax,
    }
}

pub(crate) fn detect_language(path: &Path, include_unknown: bool) -> Option<LanguageSpec> {
    let projection = matched_projection(path);
    if let Some(projection) = projection {
        let LiteralProjection::Row(name, _, _, _, _) = projection;
        if let Some(spec) = language_registry()
            .into_iter()
            .find(|spec| spec.id.as_str() == *name)
        {
            return Some(spec);
        }
    }
    include_unknown.then(unknown_spec)
}
