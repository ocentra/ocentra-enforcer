//! `LFE-*` — ADBP's five layered/frontend AST linters (d12), folded into
//! `enforcer-lang-ts` as first-class `Validator` impls: no-repo-in-router
//! (`LFE-1.1`), no-fetch-in-useEffect (`LFE-1.2`), feature-boundaries
//! (`LFE-1.3`), string-enum-only (`LFE-1.4`), and symbol-level-DI
//! (`LFE-1.5`). ADBP_PARITY_MATRIX rows FRONT-01..FRONT-05. Each parses the
//! target TS/JSX with line/symbol-level analysis (mirroring the
//! `frontend_react.rs` sibling's established precedent for this crate — a
//! text-position-guarded scan, not a full tree-sitter grammar walk; see that
//! module's doc comment for why line/symbol analysis is the crate's actual
//! T1-deterministic mechanism today) and emits structured `Finding`s.
//!
//! Rule records (ruleId <-> validator <-> {fail+pass fixtures} <->
//! doc-anchor <-> tier) for this family live in a NEW `enforcer-rules`
//! catalog (`rules/layered-frontend.json`), mirroring the `frontend-react`/
//! `fastapi-layered` precedent — NOT the 73-row `rules/rules.json` TS
//! registry this crate's `registry::build_all` walks. This module therefore
//! does not touch `registry.rs`/`registry::build_all`, per the workpack's
//! "must not edit the crate skeleton or the baseline TS validators" +
//! "disjoint by file from... e-pack-frontend-react" constraint; its rows
//! are wired only through the standalone catalog + this module's own
//! [`validators`] builder.
//!
//! # Position guard (mem-arc-06/07)
//! Every detector here is guarded by POSITION, not bare substring presence:
//! the router-repo check inspects only the CURRENT file's own path plus its
//! import/instantiation lines; the useEffect-fetch check looks only inside
//! the textual span between a `useEffect(` opener and its `}, [` closer;
//! the feature-boundary check parses the importer's OWN feature name from
//! its path before comparing against the imported target; the enum-shape
//! check inspects only the body of an `enum` block; the symbol-DI check
//! looks only at constructor-parameter type annotations.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::{from_source, SourceFinding};
use crate::boundary::source_analysis::{
    import_target, in_router_layer, is_symbol_di_token, layered_imported_feature,
    member_has_string_initializer, owning_feature,
};
use crate::boundary::source_text::{lines, source_line_role, SourceLineRole};

/// The per-call-site shape of a [`Finding`], everything EXCEPT the
/// `rule_id`/`file` (which come from the validator/input themselves).
/// Uses the shared source-boundary observation bundle to keep
/// [`finding`]'s arity under the workspace's `clippy::too_many_arguments`
/// gate.
/// Build one [`Finding`] with this module's common shape.
fn finding(
    rule_id: &RuleId,
    input: &ValidationInput<'_>,
    spec: SourceFinding<'_>,
) -> Result<Finding, DecodeError> {
    from_source(rule_id, input.file, spec)
}

/// Extract the quoted module path of an `import`/`export ... from "..."`
/// statement line, or `None` if the line isn't an import/export-from
/// statement. Mirrors `frontend_react::import_target`/
/// `import_boundaries::import_target` (arc-07 precedent) rather than
/// re-deriving its own parse.
/// FRONT-01 / `LFE-1.1` — no repository/data access inside the router
/// layer (`no-repo-in-router`, T1): a `router(s)/**` file importing or
/// instantiating a `*Repository` symbol directly is flagged; a router that
/// delegates to a service (no repo symbol referenced) stays clean. Position
/// guard: only fires when the CURRENT file's own path sits under a
/// `router`/`routers` directory, mirroring `fastapi_layered::in_layer` +
/// `NoRepoInRoutersValidator` (the exact same doctrine rule, TS-side).
#[derive(Debug)]
#[doc = "Router repository-access validator."]
pub struct NoRepoInRouterValidator {
    rule_id: RuleId,
}

impl NoRepoInRouterValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("LFE-1.1")?,
        })
    }
}

/// True when `path` has a `router/` or `routers/` path segment.
impl Validator for NoRepoInRouterValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if !in_router_layer(input.file) {
            return Vec::new();
        }
        for line in lines(input.source) {
            if source_line_role(line.text) == SourceLineRole::CommentOnly {
                continue;
            }
            let references_repo = line.text.as_str().contains("Repository(")
                || (import_target(line.text.as_str()).is_some()
                    && line.text.as_str().contains("Repository"));
            if references_repo {
                return finding(
                    &self.rule_id,
                    &input,
                    SourceFinding {
                        severity: Severity::Error,
                        title: "layered: router references a Repository symbol",
                        detail: format!(
                            "line {}: `{path}` (a router-layer file) imports/instantiates a \
                             `*Repository` symbol directly; routers must depend on a service, \
                             not the persistence layer. Fix: inject a service and let it own the \
                             repository.",
                            line.number
                        ),
                        line: line.number,
                        snippet: Some(line.text.as_str().trim()),
                    },
                )
                .into_iter()
                .collect();
            }
        }
        Vec::new()
    }
}

/// FRONT-02 / `LFE-1.2` — no data fetching inside `useEffect`
/// (`no-fetch-in-useEffect`, T1): a `useEffect(() => { fetch(...) /
/// axios. ... })` is flagged; a query hook (`useQuery({queryKey,
/// queryFn})`) stays clean. Position guard: only fires when a
/// `fetch(`/`axios.` call textually falls between a `useEffect(` opener and
/// its matching `}, [` effect-closer, mirroring
/// `frontend_react::NoFetchInUseEffectValidator` (FE-STATE-1.2) — the same
/// doctrine rule, restated here as its own registry-backed d12 entry per
/// ADBP_PARITY_MATRIX's FRONT-02 row.
#[derive(Debug)]
#[doc = "Layered frontend effect-fetch validator."]
pub struct NoFetchInUseEffectValidator {
    rule_id: RuleId,
}

impl NoFetchInUseEffectValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("LFE-1.2")?,
        })
    }
}

const FETCH_MARKERS: &[&str] = &["fetch(", "axios."];

impl Validator for NoFetchInUseEffectValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let all_lines: Vec<_> = lines(input.source).collect();
        let mut in_effect = false;
        let mut effect_start_line = None;
        for line in &all_lines {
            if line.text.as_str().contains("useEffect(") {
                in_effect = true;
                effect_start_line = Some(line.number);
                continue;
            }
            if in_effect {
                if source_line_role(line.text) == SourceLineRole::CommentOnly {
                    continue;
                }
                if FETCH_MARKERS.iter().any(|m| line.text.as_str().contains(m)) {
                    return finding(
                        &self.rule_id,
                        &input,
                        SourceFinding {
                            severity: Severity::Error,
                            title: "layered: fetch/axios data-loading inside useEffect",
                            detail: format!(
                                "line {}: `useEffect` (opened line {}) performs \
                                 a `fetch(`/`axios.` call directly for data-loading; use a query \
                                 hook (`useQuery({{queryKey, queryFn}})`) instead of a raw effect \
                                 fetch.",
                                line.number,
                                effect_start_line.unwrap_or(line.number)
                            ),
                            line: line.number,
                            snippet: Some(line.text.as_str().trim()),
                        },
                    )
                    .into_iter()
                    .collect();
                }
                if line.text.as_str().contains("}, [") {
                    in_effect = false;
                }
            }
        }
        Vec::new()
    }
}

/// Parse `features/<name>/...` out of a repo-relative path, returning
/// `<name>`, if the path has that shape.
/// Parse `@/features/<name>/...` (or a relative `../<name>/...` /
/// `../../features/<name>/...`) out of an import target, returning
/// `<name>`, if the import has that shape. Also recognizes the bare
/// relative-crossing shape `../<name>/internal/...` used by
/// ADBP_PARITY_MATRIX's FRONT-03 example (`../otherFeature/internal/x`).
/// FRONT-03 / `LFE-1.3` — feature-boundary imports (`feature-boundaries`,
/// T1): a `features/<a>/**` file importing `@/features/<b>/...` (or a
/// relative `../<b>/internal/...`) — a DIFFERENT feature slice — is
/// flagged; importing via the feature's own public entry, `@/lib`,
/// `@/shared`, `@/components`, or its OWN feature slice stays clean.
/// Position guard: the importer's OWN feature name is parsed from its file
/// path and excluded from the forbidden-cross-import check. Mirrors
/// `frontend_react::FeatureBoundaryValidator` (FE-ARCH-1.3), restated here
/// as its own registry-backed d12 entry per ADBP_PARITY_MATRIX's FRONT-03
/// row (deep-import shape, not just the `@/` alias shape).
#[derive(Debug)]
#[doc = "Layered frontend feature-boundary validator."]
pub struct FeatureBoundariesValidator {
    rule_id: RuleId,
}

impl FeatureBoundariesValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("LFE-1.3")?,
        })
    }
}

impl Validator for FeatureBoundariesValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(own_feature) = owning_feature(input.file) else {
            return Vec::new();
        };
        for line in lines(input.source) {
            if source_line_role(line.text) == SourceLineRole::CommentOnly {
                continue;
            }
            let Some(target) = import_target(line.text.as_str()) else {
                continue;
            };
            if let Some(other_feature) = layered_imported_feature(target) {
                if other_feature != own_feature {
                    return finding(
                        &self.rule_id,
                        &input,
                        SourceFinding {
                            severity: Severity::Error,
                            title: "layered: cross-feature deep import",
                            detail: format!(
                                "line {}: feature `{own_feature}` imports `{target}` — a deep \
                                 reach into a DIFFERENT feature's internals (`{other_feature}`); \
                                 a feature may import only its own slice, `@/lib`, `@/shared`, \
                                 `@/components`, or another feature's public entry point, never \
                                 another feature's internals.",
                                line.number
                            ),
                            line: line.number,
                            snippet: Some(line.text.as_str().trim()),
                        },
                    )
                    .into_iter()
                    .collect();
                }
            }
        }
        Vec::new()
    }
}

/// FRONT-04 / `LFE-1.4` — string-enum-only enums (`str-enum-only`, T1): a
/// numeric or implicit-value TS `enum` member (e.g. `enum Color { Red }` or
/// `enum Color { Red = 0 }`) is flagged; a string-valued enum (`enum Color
/// { Red = 'red' }`) stays clean. Whole-block signal: scans from an `enum
/// Name {` opener to its closing `}` and flags the first member with no
/// `= '...'`/`= "..."` string-literal initializer.
#[derive(Debug)]
#[doc = "TypeScript string-enum validator."]
pub struct StrEnumOnlyValidator {
    rule_id: RuleId,
}

impl StrEnumOnlyValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("LFE-1.4")?,
        })
    }
}

/// True when a (trimmed) enum-body line's member initializer, if any, is a
/// single/double-quoted string literal. A member with NO initializer at all
/// (implicit numeric) also fails this check.
impl Validator for StrEnumOnlyValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let all_lines: Vec<_> = lines(input.source).collect();
        let mut in_enum = false;
        for line in &all_lines {
            let trimmed = line.text.as_str().trim_start();
            if !in_enum && (trimmed.starts_with("enum ") || trimmed.starts_with("export enum ")) {
                in_enum = true;
                continue;
            }
            if !in_enum {
                continue;
            }
            if trimmed.starts_with('}') {
                in_enum = false;
                continue;
            }
            if trimmed.is_empty()
                || source_line_role(
                    enforcer_domain::boundary::validation::ValidationSource::from_text(trimmed),
                ) == SourceLineRole::CommentOnly
            {
                continue;
            }
            if !member_has_string_initializer(trimmed) {
                return finding(
                    &self.rule_id,
                    &input,
                    SourceFinding {
                        severity: Severity::Error,
                        title: "layered: enum member is numeric/implicit, not string-valued",
                        detail: format!(
                            "line {}: enum member `{}` has no string-literal initializer \
                             (numeric or implicit-value enums are forbidden); every member must \
                             be `Name = '...'`.",
                            line.number,
                            trimmed.trim_end_matches(',')
                        ),
                        line: line.number,
                        snippet: Some(trimmed.trim_end_matches(',')),
                    },
                )
                .into_iter()
                .collect();
            }
        }
        Vec::new()
    }
}

/// FRONT-05 / `LFE-1.5` — symbol-level dependency injection
/// (`symbol-level-DI`, T1): a constructor parameter typed as a concrete,
/// instantiable class (a bare `PascalCase` annotation with no leading `I`
/// interface-convention marker and no `Token`/`Symbol` suffix) is flagged
/// when injected via `@inject(ConcreteClass)`; injection via a
/// symbol/interface token (`@inject(ISomething)` / `@inject(SomethingToken)`
/// / `@inject(Symbol...)`) stays clean. Position guard: only fires on
/// `@inject(...)`-decorated constructor parameters, not every class
/// reference in the file.
#[derive(Debug)]
#[doc = "Symbol-level dependency-injection validator."]
pub struct SymbolLevelDiValidator {
    rule_id: RuleId,
}

impl SymbolLevelDiValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("LFE-1.5")?,
        })
    }
}

/// True when `token` reads as a symbol/interface DI token rather than a
/// concrete, `new`-able class: an interface-convention `I`-prefixed name
/// (`IFoo`), a `*Token`/`*Symbol` suffix, or a `Symbol(...)` call/reference.
impl Validator for SymbolLevelDiValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for line in lines(input.source) {
            if source_line_role(line.text) == SourceLineRole::CommentOnly {
                continue;
            }
            let trimmed = line.text.as_str().trim_start();
            let Some((_, rest)) = trimmed.split_once("@inject(") else {
                continue;
            };
            let Some((token, _)) = rest.split_once(')') else {
                continue;
            };
            if !is_symbol_di_token(token) {
                return finding(
                    &self.rule_id,
                    &input,
                        SourceFinding {
                        severity: Severity::Error,
                        title: "layered: DI token is a concrete class, not a symbol/interface",
                        detail: format!(
                            "line {}: `@inject({token})` injects a concrete, `new`-able class \
                             directly; dependency injection must route through a symbol/interface \
                             token (e.g. `ISomething`, `SomethingToken`, or `Symbol('Something')`), \
                             never a concrete class reference.",
                            line.number
                        ),
                        line: line.number,
                        snippet: Some(line.text.as_str().trim()),
                    },
                )
                .into_iter()
                .collect();
            }
        }
        Vec::new()
    }
}

/// Build every `LFE-*` layered/frontend validator this module owns. Wired
/// through the standalone `enforcer-rules` catalog
/// (`rules/layered-frontend.json`), NOT `registry::build_all` (see module
/// docs).
pub fn validators(
) -> Result<Vec<Box<dyn Validator>>, enforcer_domain::boundary::decode_error::DecodeError> {
    Ok(vec![
        Box::new(NoRepoInRouterValidator::new()?),
        Box::new(NoFetchInUseEffectValidator::new()?),
        Box::new(FeatureBoundariesValidator::new()?),
        Box::new(StrEnumOnlyValidator::new()?),
        Box::new(SymbolLevelDiValidator::new()?),
    ])
}

#[cfg(test)]
mod tests {
    use crate::boundary::test_fixtures::run_fixture_parity;

    use super::{
        validators, FeatureBoundariesValidator, NoFetchInUseEffectValidator,
        NoRepoInRouterValidator, StrEnumOnlyValidator, SymbolLevelDiValidator, ValidationInput,
        Validator,
    };

    #[test]
    fn five_validators_registered_with_unique_rule_ids(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let vs = validators()?;
        assert_eq!(vs.len(), 5);
        let mut seen = std::collections::BTreeSet::new();
        for v in &vs {
            assert!(seen.insert(v.rule_id().to_string()));
        }
        assert_eq!(seen.len(), 5);
        Ok(())
    }

    #[test]
    fn lfe_no_repo_in_router() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoRepoInRouterValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/layered_frontend/no_repo_in_router/routers/fail.ts",
            "tests/fixtures/layered_frontend/no_repo_in_router/routers/pass.ts",
        )?;
        Ok(())
    }

    #[test]
    fn lfe_no_fetch_in_use_effect() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoFetchInUseEffectValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/layered_frontend/no_fetch_in_use_effect/fail.tsx",
            "tests/fixtures/layered_frontend/no_fetch_in_use_effect/pass.tsx",
        )?;
        Ok(())
    }

    #[test]
    fn lfe_feature_boundaries() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FeatureBoundariesValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/layered_frontend/feature_boundaries/features/checkout/fail.ts",
            "tests/fixtures/layered_frontend/feature_boundaries/features/checkout/pass.ts",
        )?;
        Ok(())
    }

    #[test]
    fn lfe_str_enum_only() -> Result<(), Box<dyn std::error::Error>> {
        let validator = StrEnumOnlyValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/layered_frontend/str_enum_only/fail.ts",
            "tests/fixtures/layered_frontend/str_enum_only/pass.ts",
        )?;
        Ok(())
    }

    #[test]
    fn lfe_symbol_level_di() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SymbolLevelDiValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/layered_frontend/symbol_level_di/fail.ts",
            "tests/fixtures/layered_frontend/symbol_level_di/pass.ts",
        )?;
        Ok(())
    }

    /// Confirms the feature-boundary check's own-feature exemption is a
    /// PATH exemption keyed off the importer's path, not a blanket
    /// "same-name substring" bug: importing the feature's OWN public entry
    /// (`@/features/<own>/...`) must stay clean.
    #[test]
    fn lfe_feature_boundaries_own_feature_import_is_exempt(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = FeatureBoundariesValidator::new()?;
        let root = crate::boundary::test_fixtures::fixture_root()?;
        let file = crate::boundary::test_fixtures::fixture_path(
            "tests/fixtures/layered_frontend/feature_boundaries/features/checkout/own_feature_import.ts",
        )?;
        let source = std::fs::read_to_string(root.resolve(&file))?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(
            findings.is_empty(),
            "own-feature import must stay clean: {findings:#?}"
        );
        Ok(())
    }
}
