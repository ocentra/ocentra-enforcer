//! UL06 P1B identity-preserving routing contracts.

use enforcer_domain::config_types::NativeTool;
use enforcer_domain::language_types::{
    DetectionMatcher, DetectionMatcherKind, LiteralDisposition, LiteralProjection,
    LiteralProjectionDisposition, LiteralReference, ScanFamilyDisposition,
    StructuralLanguageSupport,
};
use enforcer_domain::paths::RelPath;
use enforcer_domain::scan_types::{LanguageFamily, RouteScope};
use enforcer_scan::boundary::router::{
    CanonicalConsumerCapabilityProjectionResponse, CanonicalLanguageRouteResponse,
};
use enforcer_scan::router::identity::{
    detect_language_identities, ConsumerCapabilityState, DetectedLanguageRoute,
    NativeToolProjection, RouteCapabilityDisposition, UnknownLanguagePolicy,
};
use enforcer_scan::router::plan::build_canonical_route_plan;
use enforcer_syntax::registry::{
    collision_resolutions, detection_precedence, language_registry, literal_projections,
};
use std::str::FromStr;

fn rel(value: &str) -> Result<RelPath, String> {
    RelPath::from_str(value).map_err(|error| format!("test path must be repo-relative: {error:?}"))
}

fn path_for_matcher(matcher: DetectionMatcher) -> Result<RelPath, String> {
    let value = match matcher {
        DetectionMatcher::Extension(value) => format!("src/fixture.{value}"),
        DetectionMatcher::ExactBasename(value) => format!("src/{value}"),
        DetectionMatcher::CompoundSuffix(value) => format!("src/fixture{value}"),
    };
    rel(&value)
}

fn first_matcher_for_kind(kind: DetectionMatcherKind) -> Result<DetectionMatcher, String> {
    language_registry()
        .iter()
        .flat_map(|record| record.matchers().iter().copied())
        .find(|matcher| matcher_kind(*matcher) == kind)
        .ok_or_else(|| "the canonical registry must contain the requested matcher kind".to_owned())
}

fn matcher_kind(matcher: DetectionMatcher) -> DetectionMatcherKind {
    match matcher {
        DetectionMatcher::Extension(_) => DetectionMatcherKind::Extension,
        DetectionMatcher::ExactBasename(_) => DetectionMatcherKind::ExactBasename,
        DetectionMatcher::CompoundSuffix(_) => DetectionMatcherKind::CompoundSuffix,
    }
}

fn canonical_route(
    routes: &[DetectedLanguageRoute],
) -> Result<
    (
        enforcer_domain::language_types::LanguageId,
        enforcer_syntax::registry::CanonicalLanguageName,
    ),
    String,
> {
    if routes.len() != 1 {
        return Err("one isolated matcher must produce one route".to_owned());
    }
    match &routes[0] {
        DetectedLanguageRoute::Canonical(route) => Ok((route.id(), route.canonical_name())),
        other => Err(format!("expected canonical route, got {other:?}")),
    }
}

#[test]
fn canonical_denominators_and_crosswalk_remain_exact() -> Result<(), String> {
    assert_eq!(language_registry().len(), 160);
    assert_eq!(
        language_registry()
            .iter()
            .filter(|record| record.structural() == StructuralLanguageSupport::ParseFile)
            .count(),
        156
    );
    assert_eq!(
        language_registry()
            .iter()
            .filter(|record| record.structural() == StructuralLanguageSupport::NoParseFile)
            .count(),
        4
    );
    assert_eq!(literal_projections().len(), 68);
    assert_eq!(
        literal_projections()
            .iter()
            .filter(|row| !matches!(
                row,
                LiteralProjection::Row(_, LiteralProjectionDisposition::Fallback, _, _, _)
            ))
            .count(),
        67
    );
    assert_eq!(
        literal_projections()
            .iter()
            .filter(|row| matches!(
                row,
                LiteralProjection::Row(_, LiteralProjectionDisposition::LiteralOnly, _, _, _)
            ))
            .count(),
        5
    );
    assert_eq!(
        language_registry()
            .iter()
            .filter(|record| {
                matches!(
                    record.literal_disposition(),
                    LiteralDisposition::Unsupported | LiteralDisposition::NotApplicable
                )
            })
            .count(),
        85
    );

    let manifest: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../enforcer-syntax/registry/languages.json"
    )))
    .map_err(|error| format!("reviewed language manifest must remain valid JSON: {error}"))?;
    for (name, expected) in [
        ("total", 68),
        ("named", 67),
        ("oneToOne", 51),
        ("aliasCollision", 3),
        ("oneToMany", 8),
        ("literalOnly", 5),
        ("fallback", 1),
    ] {
        assert_eq!(
            manifest["crosswalkCounts"][name].as_u64(),
            Some(expected),
            "crosswalk denominator drifted for {name}"
        );
    }
    Ok(())
}

#[test]
fn reviewed_precedence_is_typed_and_deterministic() -> Result<(), String> {
    assert_eq!(
        detection_precedence().ordered_kinds(),
        &[
            DetectionMatcherKind::ExactBasename,
            DetectionMatcherKind::CompoundSuffix,
            DetectionMatcherKind::Extension,
        ]
    );
    assert_eq!(
        detection_precedence().same_kind_tie_break(),
        enforcer_domain::language_types::DetectionPrecedenceTieBreak::LongestValue
    );
    assert!(!collision_resolutions().is_empty());
    Ok(())
}

#[test]
fn extension_match_preserves_canonical_identity_and_is_case_insensitive() -> Result<(), String> {
    let matcher = first_matcher_for_kind(DetectionMatcherKind::Extension)?;
    let path = path_for_matcher(matcher)?;
    let upper = rel(&path.as_str().to_ascii_uppercase())?;
    let first = detect_language_identities(&[path], UnknownLanguagePolicy::Exclude);
    let second = detect_language_identities(&[upper], UnknownLanguagePolicy::Exclude);
    assert_eq!(canonical_route(&first)?, canonical_route(&second)?);
    Ok(())
}

#[test]
fn exact_basename_and_compound_suffix_matchers_preserve_identity() -> Result<(), String> {
    let basename = first_matcher_for_kind(DetectionMatcherKind::ExactBasename)?;
    let compound = first_matcher_for_kind(DetectionMatcherKind::CompoundSuffix)?;
    let basename_path = path_for_matcher(basename)?;
    let compound_path = path_for_matcher(compound)?;
    assert!(matches!(
        detect_language_identities(&[basename_path], UnknownLanguagePolicy::Exclude).as_slice(),
        [DetectedLanguageRoute::Canonical(_)]
    ));
    assert!(matches!(
        detect_language_identities(&[compound_path], UnknownLanguagePolicy::Exclude).as_slice(),
        [DetectedLanguageRoute::Canonical(_)]
    ));
    Ok(())
}

#[test]
fn literal_only_projection_is_supplemental_not_parser_identity() -> Result<(), String> {
    let Some((name, matcher)) = literal_projections().iter().find_map(|row| match row {
        LiteralProjection::Row(name, LiteralProjectionDisposition::LiteralOnly, _, matchers, _) => {
            matchers.first().copied().map(|matcher| (*name, matcher))
        }
        _ => None,
    }) else {
        return Err("all five literal-only rows must expose a matcher".to_owned());
    };
    let path = path_for_matcher(matcher)?;
    let routes = detect_language_identities(&[path], UnknownLanguagePolicy::Exclude);
    assert_eq!(
        routes,
        vec![DetectedLanguageRoute::SupplementalLiteral { name }]
    );
    Ok(())
}

#[test]
fn recognized_structural_identity_is_explicitly_unsupported_in_p1b() -> Result<(), String> {
    let Some((record, matcher)) = language_registry().iter().find_map(|record| {
        (record.structural() == StructuralLanguageSupport::ParseFile)
            .then(|| {
                record
                    .matchers()
                    .first()
                    .copied()
                    .map(|matcher| (record, matcher))
            })
            .flatten()
    }) else {
        return Err(
            "a structural canonical identity must retain a matcher for this contract".to_owned(),
        );
    };
    let path = path_for_matcher(matcher)?;
    let routes = detect_language_identities(&[path], UnknownLanguagePolicy::Include);
    let [DetectedLanguageRoute::Canonical(route)] = routes.as_slice() else {
        return Err(format!("known canonical matcher fell through: {routes:?}"));
    };
    assert_eq!(route.id(), record.id());
    assert_eq!(route.capability(), RouteCapabilityDisposition::Unsupported);
    Ok(())
}

#[test]
fn scan_family_projection_is_separate_from_structural_and_literal_support() -> Result<(), String> {
    for (path, expected) in [
        (
            "src/lib.rs",
            ScanFamilyDisposition::Mapped(LanguageFamily::Rust),
        ),
        (
            "web/index.js",
            ScanFamilyDisposition::Mapped(LanguageFamily::TypeScript),
        ),
        (
            "config/settings.yaml",
            ScanFamilyDisposition::Mapped(LanguageFamily::YamlOrConfig),
        ),
    ] {
        let routes = detect_language_identities(&[rel(path)?], UnknownLanguagePolicy::Exclude);
        let [DetectedLanguageRoute::Canonical(route)] = routes.as_slice() else {
            return Err(format!(
                "expected one canonical route for {path}: {routes:?}"
            ));
        };
        assert_eq!(route.scan_family_disposition(), expected);
        assert_eq!(route.capability(), RouteCapabilityDisposition::Unsupported);
        let consumers = route.consumer_capabilities();
        assert_eq!(consumers.native_scan(), expected);
        assert_eq!(
            consumers.native_tool(),
            match expected {
                ScanFamilyDisposition::Mapped(LanguageFamily::Rust) => {
                    NativeToolProjection::Mapped(NativeTool::Cargo)
                }
                ScanFamilyDisposition::Mapped(LanguageFamily::TypeScript) => {
                    NativeToolProjection::Mapped(NativeTool::Tsc)
                }
                _ => NativeToolProjection::Unsupported,
            }
        );
        assert_eq!(consumers.rule_packs(), ConsumerCapabilityState::Unsupported);
        assert_eq!(consumers.cli(), ConsumerCapabilityState::Unsupported);
        assert_eq!(consumers.ui(), ConsumerCapabilityState::Unsupported);
    }
    Ok(())
}

#[test]
fn scan_family_wire_rejects_missing_duplicate_unknown_and_mismatched_dispositions(
) -> Result<(), String> {
    let base = serde_json::json!({
       "kind": "canonical",
        "languageId": 1,
        "canonicalName": "Rust",
        "structural": { "kind": "parseFile" },
        "capability": { "kind": "unsupported" },
        "scanFamilyDisposition": { "kind": "mapped", "family": { "kind": "rust" } },
        "consumerCapabilities": {
            "nativeScan": { "kind": "mapped", "family": { "kind": "rust" } },
            "nativeTool": { "kind": "mapped", "tool": "cargo" },
            "rulePacks": { "kind": "unsupported" },
            "cli": { "kind": "unsupported" },
            "ui": { "kind": "unsupported" }
        }
    });

    let round_tripped = serde_json::from_value::<CanonicalLanguageRouteResponse>(base.clone())
        .map_err(|error| error.to_string())?;
    assert_eq!(
        serde_json::to_value(&round_tripped).map_err(|error| error.to_string())?,
        base,
        "canonical consumer response DTO must round-trip exactly"
    );
    let consumer = serde_json::from_value::<CanonicalConsumerCapabilityProjectionResponse>(
        base["consumerCapabilities"].clone(),
    )
    .map_err(|error| error.to_string())?;
    let consumer_wire = serde_json::to_string(&consumer).map_err(|error| error.to_string())?;
    let consumer_back =
        serde_json::from_str::<CanonicalConsumerCapabilityProjectionResponse>(&consumer_wire)
            .map_err(|error| error.to_string())?;
    assert_eq!(consumer_back, consumer);

    let mut missing = base.clone();
    let Some(missing_object) = missing.as_object_mut() else {
        return Err("canonical wire fixture must be an object".to_owned());
    };
    missing_object.remove("scanFamilyDisposition");
    let missing_error = serde_json::from_value::<CanonicalLanguageRouteResponse>(missing)
        .expect_err("missing disposition must be rejected");
    assert!(missing_error.to_string().contains("scanFamilyDisposition"));

    let mut unknown = base.clone();
    unknown["scanFamilyDisposition"] = serde_json::json!({ "kind": "future" });
    let unknown_error = serde_json::from_value::<CanonicalLanguageRouteResponse>(unknown)
        .expect_err("unknown disposition must be rejected");
    assert!(unknown_error.to_string().contains("unknown variant"));

    let mut mismatched = base.clone();
    mismatched["scanFamilyDisposition"] =
        serde_json::json!({ "kind": "mapped", "family": { "kind": "python" } });
    let mismatch_error = serde_json::from_value::<CanonicalLanguageRouteResponse>(mismatched)
        .expect_err("mismatched disposition must be rejected");
    assert!(mismatch_error
        .to_string()
        .contains("does not match the canonical registry"));

    let duplicate = r#"{
        "kind":"canonical","languageId":1,"canonicalName":"Rust",
        "structural":{"kind":"parseFile"},"capability":{"kind":"unsupported"},
        "scanFamilyDisposition":{"kind":"mapped","family":{"kind":"rust"}},
        "scanFamilyDisposition":{"kind":"unsupported"}
    }"#;
    let duplicate_error = serde_json::from_str::<CanonicalLanguageRouteResponse>(duplicate)
        .expect_err("duplicate disposition must be rejected");
    assert!(duplicate_error.to_string().contains("duplicate field"));

    let mut missing_consumer = base.clone();
    let Some(missing_consumer_object) = missing_consumer.as_object_mut() else {
        return Err("canonical wire fixture must be an object".to_owned());
    };
    missing_consumer_object.remove("consumerCapabilities");
    let missing_consumer_error =
        serde_json::from_value::<CanonicalLanguageRouteResponse>(missing_consumer)
            .expect_err("missing consumer capabilities must be rejected");
    assert!(missing_consumer_error
        .to_string()
        .contains("consumerCapabilities"));

    let mut unknown_consumer = base.clone();
    unknown_consumer["consumerCapabilities"]["nativeTool"] =
        serde_json::json!({ "kind": "future" });
    let unknown_consumer_error =
        serde_json::from_value::<CanonicalLanguageRouteResponse>(unknown_consumer)
            .expect_err("unknown consumer disposition must be rejected");
    assert!(unknown_consumer_error
        .to_string()
        .contains("unknown variant"));

    let mut missing_native_tool_value = base.clone();
    missing_native_tool_value["consumerCapabilities"]["nativeTool"] =
        serde_json::json!({ "kind": "mapped" });
    let missing_native_tool_error =
        serde_json::from_value::<CanonicalLanguageRouteResponse>(missing_native_tool_value)
            .expect_err("mapped native-tool disposition without a tool must be rejected");
    assert!(missing_native_tool_error
        .to_string()
        .contains("missing field `tool`"));

    let mut unknown_native_tool_value = base.clone();
    unknown_native_tool_value["consumerCapabilities"]["nativeTool"] =
        serde_json::json!({ "kind": "mapped", "tool": "future" });
    let unknown_native_tool_error =
        serde_json::from_value::<CanonicalLanguageRouteResponse>(unknown_native_tool_value)
            .expect_err("unknown native-tool identity must be rejected");
    assert!(unknown_native_tool_error
        .to_string()
        .contains("unknown variant"));

    let mut mismatched_native_tool_value = base.clone();
    mismatched_native_tool_value["consumerCapabilities"]["nativeTool"] =
        serde_json::json!({ "kind": "mapped", "tool": "tsc" });
    let mismatched_native_tool_error =
        serde_json::from_value::<CanonicalLanguageRouteResponse>(mismatched_native_tool_value)
            .expect_err("mismatched native-tool identity must be rejected");
    assert!(mismatched_native_tool_error
        .to_string()
        .contains("consumerCapabilities does not match"));

    let mut unknown_consumer_field = base.clone();
    unknown_consumer_field["consumerCapabilities"]["futureField"] = serde_json::json!(true);
    let unknown_consumer_field_error =
        serde_json::from_value::<CanonicalLanguageRouteResponse>(unknown_consumer_field)
            .expect_err("unknown consumer field must be rejected");
    assert!(unknown_consumer_field_error
        .to_string()
        .contains("unknown field"));

    let mut mismatched_consumer = base.clone();
    mismatched_consumer["consumerCapabilities"]["nativeTool"] =
        serde_json::json!({ "kind": "notApplicable" });
    let mismatched_consumer_error =
        serde_json::from_value::<CanonicalLanguageRouteResponse>(mismatched_consumer)
            .expect_err("mismatched consumer disposition must be rejected");
    assert!(mismatched_consumer_error
        .to_string()
        .contains("consumerCapabilities does not match"));

    let duplicate_consumer = r#"{
        "kind":"canonical","languageId":1,"canonicalName":"Rust",
        "structural":{"kind":"parseFile"},"capability":{"kind":"unsupported"},
        "scanFamilyDisposition":{"kind":"mapped","family":{"kind":"rust"}},
        "consumerCapabilities":{"nativeScan":{"kind":"mapped","family":{"kind":"rust"}},"nativeTool":{"kind":"unsupported"},"rulePacks":{"kind":"unsupported"},"cli":{"kind":"unsupported"},"ui":{"kind":"unsupported"}},
        "consumerCapabilities":{"nativeScan":{"kind":"unsupported"},"nativeTool":{"kind":"unsupported"},"rulePacks":{"kind":"unsupported"},"cli":{"kind":"unsupported"},"ui":{"kind":"unsupported"}}
    }"#;
    let duplicate_consumer_error =
        serde_json::from_str::<CanonicalLanguageRouteResponse>(duplicate_consumer)
            .expect_err("duplicate consumer capabilities must be rejected");
    assert!(duplicate_consumer_error
        .to_string()
        .contains("duplicate field"));
    Ok(())
}

#[test]
fn unknown_is_explicit_only_when_enabled() -> Result<(), String> {
    let path = rel("notes.unregistered_extension")?;
    assert!(detect_language_identities(
        std::slice::from_ref(&path),
        UnknownLanguagePolicy::Exclude
    )
    .is_empty());
    assert_eq!(
        detect_language_identities(&[path], UnknownLanguagePolicy::Include),
        vec![DetectedLanguageRoute::Unknown]
    );
    Ok(())
}

#[test]
fn canonical_route_plan_narrows_scope_without_using_legacy_other() -> Result<(), String> {
    let paths = vec![rel("src/lib.rs")?, rel("web/index.ts")?, rel("notes.qux")?];
    let routes =
        build_canonical_route_plan(&paths, &RouteScope::Repo, UnknownLanguagePolicy::Include);
    assert!(routes.iter().any(|route| {
        matches!(
            route,
            DetectedLanguageRoute::Canonical(canonical)
                if canonical.id() == language_registry()[0].id()
        )
    }));
    assert!(routes
        .iter()
        .any(|route| matches!(route, DetectedLanguageRoute::Unknown)));
    assert!(routes.iter().all(|route| {
        matches!(
            route,
            DetectedLanguageRoute::Canonical(_)
                | DetectedLanguageRoute::SupplementalLiteral { .. }
                | DetectedLanguageRoute::Unknown
        )
    }));
    Ok(())
}

#[test]
fn collision_winner_is_typed_and_not_a_legacy_other_route() -> Result<(), String> {
    let Some(first_collision) = collision_resolutions().first() else {
        return Err("the registry must contain a reviewed collision".to_owned());
    };
    let (kind, key, _members, winner) = match first_collision {
        enforcer_domain::language_types::CollisionResolution::Group(kind, key, members, winner) => {
            (*kind, *key, *members, *winner)
        }
    };
    let path = match kind {
        DetectionMatcherKind::Extension => rel(&format!("fixture.{key}"))?,
        DetectionMatcherKind::ExactBasename => rel(&format!("src/{key}"))?,
        DetectionMatcherKind::CompoundSuffix => rel(&format!("src/fixture{key}"))?,
    };
    let routes = detect_language_identities(&[path], UnknownLanguagePolicy::Exclude);
    match (winner, routes.as_slice()) {
        (LiteralReference::ParserId(id), [DetectedLanguageRoute::Canonical(route)]) => {
            assert_eq!(id, route.id());
        }
        (
            LiteralReference::SupplementalLiteralName(name),
            [DetectedLanguageRoute::SupplementalLiteral { name: actual }],
        ) => assert_eq!(name, *actual),
        (expected, actual) => {
            return Err(format!(
                "collision winner {expected:?} routed as {actual:?}"
            ))
        }
    }
    Ok(())
}
