//! `FE-*` — the React/Next/Vite frontend rule family (e-pack-frontend-react).
//! A greenfield rules module inside `enforcer-lang-ts` (arc-07); NOT a new
//! crate. Covers feature-layer boundaries, Server/Client discipline (via
//! the layer-inversion advisory), TanStack-Query data-fetching, hooks
//! discipline, typed errors, component/a11y shape, env centralization, TS
//! strictness, FSM routing (FE-facing rule only — the transition engine
//! itself is owned by d16's `enforcer-lang-common`), and the deliberate
//! Effect-Schema-not-Zod divergence from ADBP (`FE-EFFECT-1.1`).
//!
//! Rule records (ruleId <-> tier <-> fixtures <-> doc-anchor) for this
//! family live in a NEW `enforcer-rules` catalog
//! (`rules/frontend-react.json`), mirroring d16's `fsm.json` precedent —
//! NOT the 73-row `rules/rules.json` TS registry this crate's
//! `registry::build_all` walks. This module therefore does not touch
//! `registry.rs`/`registry::build_all` (per the workpack's "must not touch
//! the crate's `Validator` registration root" constraint); its rows are
//! wired only through the standalone catalog + this module's own
//! [`validators`] builder, exactly like `enforcer-lang-common::rules::fsm`
//! and `::size_shape`.
//!
//! # Position/double-dispatch guard (mem-arc-06/07)
//! Every detector here is guarded by POSITION, not bare substring
//! presence: an import-boundary check inspects only `import .. from "..."`
//! statement lines against the CURRENT file's own path (mirrors
//! `import_boundaries.rs`); the `useEffect` WHY-comment check looks at the
//! line(s) immediately preceding the `useEffect(` call, not any comment
//! anywhere in the file; the `any`-waiver check requires the waiver
//! comment on the line directly above the `: any` occurrence, not merely
//! present somewhere in the file. Each rule's comment-guard (skip
//! comment-only lines before matching) is per-rule opt-out-able via the
//! `comment_guard` parameter threaded through the shared line/text
//! scanners this module reuses from the source-text boundary, mirroring
//! `spec.rs`'s `RuleSpec::comment_guard` precedent (TS-2.1's own
//! comment-IS-the-violation carve-out).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::schema_framework::recognize_schema_framework;
use crate::boundary::finding::{from_source, SourceFinding};
use crate::boundary::source_analysis::{
    aliased_imported_feature, brace_import_names, import_target, is_any_annotation, owning_feature,
};
use crate::boundary::source_text::{lines, source_line_role, SourceLineRole};

/// The per-call-site shape of a [`Finding`], everything EXCEPT the
/// `rule_id`/`file` (which come from the validator/input themselves).
/// Bundled into one struct (rather than threaded as five separate
/// parameters) purely to keep [`finding`]'s arity under the workspace's
/// `clippy::too_many_arguments` gate — the fields still map 1:1 onto
/// [`Finding`]'s own shape.
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
/// statement. Mirrors `import_boundaries::import_target` (arc-07
/// precedent) rather than re-deriving its own parse.
/// FE-ARCH-1.3 — feature boundaries (T1): a `features/<a>/**` file
/// importing `@/features/<b>/...` (a DIFFERENT feature slice) is flagged.
/// Importing `@/lib`, `@/shared`, or `@/components` (or its OWN feature
/// slice) stays clean. Position guard: the importer's OWN feature name is
/// parsed from its file path and excluded from the forbidden-cross-import
/// check, so a feature importing its own siblings never fires.
#[derive(Debug)]
#[doc = "React feature-boundary validator."]
pub struct FeatureBoundaryValidator {
    rule_id: RuleId,
}

impl FeatureBoundaryValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-ARCH-1.3")?,
        })
    }
}

/// Parse `features/<name>/...` out of a repo-relative path, returning
/// `<name>`, if the path has that shape.
/// Parse `@/features/<name>/...` out of an import target, returning
/// `<name>`, if the import has that shape.
impl Validator for FeatureBoundaryValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(own_feature) = owning_feature(input.file) else {
            return Vec::new();
        };
        let mut findings = Vec::new();
        for line in lines(input.source) {
            let Some(target) = import_target(line.text.as_str()) else {
                continue;
            };
            if let Some(other_feature) = aliased_imported_feature(target) {
                if other_feature != own_feature {
                    findings.extend(finding(
                        &self.rule_id,
                        &input,
                        SourceFinding {
                            severity: Severity::Error,
                            title: "feature boundary: cross-feature import",
                            detail: format!(
                                "line {}: feature `{own_feature}` imports `@/features/{other_feature}/...` \
                                 (`{target}`); a feature may import only its own slice, `@/lib`, \
                                 `@/shared`, or `@/components`, never another feature's internals.",
                                line.number
                            ),
                            line: line.number,
                            snippet: Some(line.text.as_str().trim()),
                        },
                    ));
                }
            }
        }
        findings
    }
}

/// FE-ARCH-1.4 — components -> features layer inversion (T2, scored
/// advisory): `components/**` importing from `@/features/**`, OR calling
/// `useQuery(`/`fetch(` directly, scores signal; a presentational component
/// that only takes data via props stays clean. Mirrors the FSM-coverage
/// scored model (`enforcer-lang-common::rules::fsm`): each marker
/// contributes `1.0`, fires at `>= FIRE_THRESHOLD`.
#[derive(Debug)]
#[doc = "React component-layer inversion validator."]
pub struct ComponentsFeatureInversionValidator {
    rule_id: RuleId,
}

impl ComponentsFeatureInversionValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-ARCH-1.4")?,
        })
    }
}

const INVERSION_FIRE_THRESHOLD: f64 = 1.0;
const DATA_FETCH_MARKERS: &[&str] = &["useQuery(", "useQuery({", "fetch("];

impl Validator for ComponentsFeatureInversionValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if !path.contains("/components/") {
            return Vec::new();
        }
        let mut score = 0.0_f64;
        let mut first_hit_line = None;
        for line in lines(input.source) {
            if source_line_role(line.text) == SourceLineRole::CommentOnly {
                continue;
            }
            let imports_feature = import_target(line.text.as_str())
                .is_some_and(|target| target.contains("@/features/"));
            let calls_data_fetch = DATA_FETCH_MARKERS
                .iter()
                .any(|marker| line.text.as_str().contains(marker));
            if imports_feature || calls_data_fetch {
                score += 1.0;
                first_hit_line.get_or_insert(line.number);
            }
        }
        if score >= INVERSION_FIRE_THRESHOLD {
            let Some(first_hit_line) = first_hit_line else {
                return Vec::new();
            };
            return finding(
                &self.rule_id,
                &input,
                SourceFinding {
                    severity: Severity::Warning,
                    title: "layer inversion: components/ pulling feature/data-fetch concerns",
                    detail: format!(
                        "`{path}` is a `components/` (presentational) file but imports from \
                         `@/features/**` and/or calls a data-fetch hook directly (score {score:.1} \
                         >= threshold {INVERSION_FIRE_THRESHOLD:.1}); presentational components \
                         should receive data via props, not reach into feature/data layers."
                    ),
                    line: first_hit_line,
                    snippet: None,
                },
            )
            .into_iter()
            .collect();
        }
        Vec::new()
    }
}

/// FE-STATE-1.1 — no server-data-in-client-store (T1): a Zustand/`useState`
/// store field populated from an API response (a `set({ ... })`/
/// `set((state) => ...)` call whose block also `await fetch(`s /
/// `await axios.`s) is flagged; a store holding only UI flags (no
/// fetch/axios call anywhere in the same `create(...)` definition) stays
/// clean. Whole-file signal (the store factory is typically a single
/// `create(...)` call) rather than a per-line match, mirroring
/// `fsm::MandatoryFsmValidator`'s "does this file's ONE relevant construct
/// co-occur with the forbidden marker" shape.
#[derive(Debug)]
#[doc = "Client-store server-data validator."]
pub struct NoServerDataInClientStoreValidator {
    rule_id: RuleId,
}

impl NoServerDataInClientStoreValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-STATE-1.1")?,
        })
    }
}

const STORE_FACTORY_MARKERS: &[&str] = &["create((set", "create<", "useState("];
const FETCH_MARKERS: &[&str] = &["fetch(", "axios."];

impl Validator for NoServerDataInClientStoreValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let is_store = STORE_FACTORY_MARKERS
            .iter()
            .any(|marker| input.source.as_str().contains(marker));
        if !is_store {
            return Vec::new();
        }
        let has_fetch = FETCH_MARKERS
            .iter()
            .any(|marker| input.source.as_str().contains(marker));
        if !has_fetch {
            return Vec::new();
        }
        let Some(line) = lines(input.source)
            .find(|l| FETCH_MARKERS.iter().any(|m| l.text.as_str().contains(m)))
            .map(|l| l.number)
        else {
            return Vec::new();
        };
        finding(
            &self.rule_id,
            &input,
                        SourceFinding {
                severity: Severity::Error,
                title: "state: server data fetched directly inside a client store",
                detail: "A client store (Zustand `create(...)`/`useState`) must hold only UI/local state; \
                 it must not `fetch`/`axios.`-populate itself with server data — route server data \
                 through a query hook (`useQuery`) instead."
                    .into(),
                line,
                snippet: None,
            },
        )
        .into_iter()
        .collect()
    }
}

/// FE-STATE-1.2 — no fetch/axios in `useEffect` for data-loading (T1): a
/// `useEffect(() => { fetch(...) / axios. ... }, [])`-shaped effect is
/// flagged; a query hook (`useQuery({queryKey, queryFn})`) stays clean.
/// Position guard: only fires when a `fetch(`/`axios.` call textually
/// falls between a `useEffect(` opener and its matching effect body (this
/// module approximates "inside the effect" as "appears on a later line
/// before the next top-level `}, [` effect-closer", which is the same
/// text-proximity heuristic `fsm.rs` uses for its whole-construct checks).
#[derive(Debug)]
#[doc = "React effect-fetch validator."]
pub struct NoFetchInUseEffectValidator {
    rule_id: RuleId,
}

impl NoFetchInUseEffectValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-STATE-1.2")?,
        })
    }
}

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
                if FETCH_MARKERS.iter().any(|m| line.text.as_str().contains(m)) {
                    return finding(
                        &self.rule_id,
                        &input,
                        SourceFinding {
                            severity: Severity::Error,
                            title: "state: fetch/axios data-loading inside useEffect",
                            detail: format!(
                                "line {}: `useEffect` (opened line {}) performs a \
                                 `fetch(`/`axios.` call directly for data-loading; use a query hook \
                                 (`useQuery({{queryKey, queryFn}})`) instead of a raw effect fetch.",
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
                // A `}, [` (or `}, []` ) line closes the effect's dependency
                // array — leaves effect scope for the NEXT `useEffect(` scan.
                if line.text.as_str().contains("}, [") {
                    in_effect = false;
                }
            }
        }
        Vec::new()
    }
}

/// FE-HOOK-1.2 — `useEffect` requires a WHY comment (T1): a `useEffect(`
/// call with no `// why:` comment on the immediately preceding
/// (non-blank) line is flagged; one carrying `// why:` stays clean.
#[derive(Debug)]
#[doc = "React effect rationale validator."]
pub struct UseEffectWhyCommentValidator {
    rule_id: RuleId,
}

impl UseEffectWhyCommentValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-HOOK-1.2")?,
        })
    }
}

impl Validator for UseEffectWhyCommentValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let all_lines: Vec<_> = lines(input.source).collect();
        for (idx, line) in all_lines.iter().enumerate() {
            if !line.text.as_str().contains("useEffect(") {
                continue;
            }
            let has_why_above = all_lines
                .get(..idx)
                .unwrap_or(&[])
                .iter()
                .rev()
                .take_while(|prev| {
                    prev.text.as_str().trim().is_empty()
                        || source_line_role(prev.text) == SourceLineRole::CommentOnly
                })
                .any(|prev| prev.text.as_str().trim_start().starts_with("// why:"));
            if !has_why_above {
                return finding(
                    &self.rule_id,
                    &input,
                    SourceFinding {
                        severity: Severity::Error,
                        title: "hooks: useEffect missing a WHY comment",
                        detail: format!(
                            "line {}: `useEffect(` has no preceding `// why:` comment explaining \
                             why an imperative effect (rather than a declarative alternative) is \
                             necessary here.",
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

/// FE-PAT-1.4 — typed errors in `services/**` (T1): a `throw new
/// Error(...)` inside a `services/**` file is flagged; `throw new
/// ApiError(...)` (or any other NAMED, non-`Error` typed error class)
/// stays clean.
#[derive(Debug)]
#[doc = "Typed service-error validator."]
pub struct TypedErrorsInServicesValidator {
    rule_id: RuleId,
}

impl TypedErrorsInServicesValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-PAT-1.4")?,
        })
    }
}

impl Validator for TypedErrorsInServicesValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if !input.file.as_str().contains("/services/") {
            return Vec::new();
        }
        for line in lines(input.source) {
            if source_line_role(line.text) == SourceLineRole::CommentOnly {
                continue;
            }
            if line.text.as_str().contains("throw new Error(") {
                return finding(
                    &self.rule_id,
                    &input,
                    SourceFinding {
                        severity: Severity::Error,
                        title: "typed errors: bare Error thrown from services/",
                        detail: format!(
                            "line {}: `services/**` must throw a NAMED typed error class (e.g. \
                             `throw new ApiError(...)`), never the bare built-in `Error`.",
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

/// FE-CMP-1.12 + FE-A11Y-1.2 + FE-A11Y-1.3 — `next/image` + alt / a11y
/// (T1): a raw `<img src` (should be `next/image`'s `<Image>`), OR an
/// `<Image>`/`<img>` missing an `alt=` attribute, is flagged
/// (`FE-CMP-1.12`/`FE-A11Y-1.2`, same validator/finding — the "raw img" and
/// "missing alt" shapes are the two faces of one detector, matching the
/// workpack's "rides the same validator" instruction). An `<input>` with
/// no `aria-label`/`<label>` association also fires (`FE-A11Y-1.3`, a
/// second [`Validator`] impl below sharing this module's `Image` helpers'
/// spirit but a distinct rule id/finding).
#[derive(Debug)]
#[doc = "React image accessibility validator."]
pub struct ImageAltValidator {
    rule_id: RuleId,
}

impl ImageAltValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-CMP-1.12")?,
        })
    }
}

impl Validator for ImageAltValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for line in lines(input.source) {
            if source_line_role(line.text) == SourceLineRole::CommentOnly {
                continue;
            }
            let is_raw_img =
                line.text.as_str().contains("<img ") || line.text.as_str().contains("<img\n");
            let is_next_image =
                line.text.as_str().contains("<Image ") || line.text.as_str().contains("<Image\n");
            if is_raw_img {
                return finding(
                    &self.rule_id,
                    &input,
                    SourceFinding {
                        severity: Severity::Error,
                        title: "component: raw <img> instead of next/image",
                        detail: format!(
                            "line {}: a raw `<img src>` must be replaced by `next/image`'s \
                             `<Image width height alt=...>` for optimized, a11y-correct images.",
                            line.number
                        ),
                        line: line.number,
                        snippet: Some(line.text.as_str().trim()),
                    },
                )
                .into_iter()
                .collect();
            }
            if is_next_image && !line.text.as_str().contains("alt=") {
                return finding(
                    &self.rule_id,
                    &input,
                    SourceFinding {
                        severity: Severity::Error,
                        title: "a11y: <Image> missing alt text",
                        detail: format!(
                            "line {}: `<Image>` is missing an `alt=` attribute (required even for \
                             decorative images — use `alt=\"\"` explicitly if truly decorative).",
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

/// FE-A11Y-1.3 — input needs label/aria-label (T1): an `<input` element
/// with neither `aria-label=` nor `aria-labelledby=` is flagged (this
/// text-level detector cannot resolve a `<label htmlFor>` association
/// elsewhere in the tree, so it recognizes ONLY the inline-attribute
/// escape hatch — the common React pattern for a standalone input).
#[derive(Debug)]
#[doc = "React input-label validator."]
pub struct InputLabelValidator {
    rule_id: RuleId,
}

impl InputLabelValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-A11Y-1.3")?,
        })
    }
}

impl Validator for InputLabelValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for line in lines(input.source) {
            if source_line_role(line.text) == SourceLineRole::CommentOnly {
                continue;
            }
            if line.text.as_str().contains("<input ")
                && !line.text.as_str().contains("aria-label=")
                && !line.text.as_str().contains("aria-labelledby=")
            {
                return finding(
                    &self.rule_id,
                    &input,
                        SourceFinding {
                        severity: Severity::Error,
                        title: "a11y: input missing label association",
                        detail: format!(
                            "line {}: `<input>` has no `aria-label=`/`aria-labelledby=` (and no \
                             resolvable `<label htmlFor>` on this line); screen-reader users cannot \
                             identify the field's purpose.",
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

/// FE-CFG-1.1 — `import.meta.env`/`process.env` centralization (T1):
/// reading `import.meta.env.*`/`process.env.*` anywhere except
/// `lib/env.ts` is flagged; importing the typed `env` from `@/lib/env`
/// stays clean. Position guard: the exemption is keyed on the CURRENT
/// file's own path (`lib/env.ts` is allowed to read the raw env; every
/// other file must import the typed wrapper instead).
#[derive(Debug)]
#[doc = "Frontend environment-access validator."]
pub struct EnvCentralizationValidator {
    rule_id: RuleId,
}

impl EnvCentralizationValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-CFG-1.1")?,
        })
    }
}

const RAW_ENV_MARKERS: &[&str] = &["import.meta.env.", "process.env."];

impl Validator for EnvCentralizationValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if path.ends_with("lib/env.ts") || path.ends_with("lib/env.tsx") {
            return Vec::new();
        }
        for line in lines(input.source) {
            if source_line_role(line.text) == SourceLineRole::CommentOnly {
                continue;
            }
            if let Some(marker) = RAW_ENV_MARKERS
                .iter()
                .find(|m| line.text.as_str().contains(**m))
            {
                return finding(
                    &self.rule_id,
                    &input,
                    SourceFinding {
                        severity: Severity::Error,
                        title: "config: raw env access outside lib/env",
                        detail: format!(
                            "line {}: `{marker}` reads the environment directly outside \
                             `lib/env.ts`; import the typed `env` from `@/lib/env` instead so env \
                             access stays centralized and typed.",
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

/// FE-TS-1.5 — no-explicit-any (T1): a `: any` annotation with no
/// justifying inline waiver+reason is flagged; `unknown`+guard, or a `:
/// any` immediately preceded by a `// waiver: any // reason: ...` comment,
/// stays clean.
#[derive(Debug)]
#[doc = "Explicit-any validator."]
pub struct NoExplicitAnyValidator {
    rule_id: RuleId,
}

impl NoExplicitAnyValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-TS-1.5")?,
        })
    }
}

/// Byte index is a word boundary on the left, mirroring
/// the source-text boundary's word guard (kept local since this rule needs the
/// index, not just a boolean hit).
impl Validator for NoExplicitAnyValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let all_lines: Vec<_> = lines(input.source).collect();
        for (idx, line) in all_lines.iter().enumerate() {
            if source_line_role(line.text) == SourceLineRole::CommentOnly {
                continue;
            }
            if !is_any_annotation(line.text.as_str()) {
                continue;
            }
            let waived = idx
                .checked_sub(1)
                .and_then(|previous| all_lines.get(previous))
                .is_some_and(|previous| {
                    previous
                        .text
                        .as_str()
                        .trim_start()
                        .starts_with("// waiver: any")
                        && previous.text.as_str().contains("// reason:")
                });
            if waived {
                continue;
            }
            return finding(
                &self.rule_id,
                &input,
                        SourceFinding {
                    severity: Severity::Error,
                    title: "ts: explicit any with no justifying waiver",
                    detail: format!(
                        "line {}: `: any` annotation has no justifying inline waiver (a `// waiver: \
                         any // reason: <why> (TICKET)` comment on the preceding line); use \
                         `unknown` + a type guard, or add the waiver with a reason.",
                        line.number
                    ),
                    line: line.number,
                    snippet: Some(line.text.as_str().trim()),
                },
            )
            .into_iter()
            .collect();
        }
        Vec::new()
    }
}

/// FE-TS-1.14 — type-only import (T1): `import { X }` where `X` is used
/// only as a TYPE position (appears only after `:`, `<`, `extends`, or as
/// a function parameter/return annotation, never as a value — this
/// text-level detector approximates that as "never followed by `(` or
/// preceded by `new `") is flagged; `import type { X }` stays clean.
#[derive(Debug)]
#[doc = "Type-only import validator."]
pub struct TypeOnlyImportValidator {
    rule_id: RuleId,
}

impl TypeOnlyImportValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-TS-1.14")?,
        })
    }
}

/// Parse the brace-import specifier list out of an `import { A, B } from
/// "..."` line (returns `None` for `import type { .. }`, default imports,
/// or non-brace imports).
impl Validator for TypeOnlyImportValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for line in lines(input.source) {
            let Some(names) = brace_import_names(line.text.as_str()) else {
                continue;
            };
            for name in names {
                // A value-position use: called as a function/constructed,
                // or referenced as a bare expression (assigned/returned).
                // This detector recognizes the two unambiguous VALUE shapes
                // (`new Name(` / `Name(`) and treats everything else in
                // the rest of the file as a type-only use, matching the
                // rule's stated fixture shape (a type annotation only).
                let used_as_value = input.source.as_str().contains(&format!("new {name}("))
                    || input.source.as_str().contains(&format!("{name}("));
                if !used_as_value && input.source.as_str().contains(name) {
                    return finding(
                        &self.rule_id,
                        &input,
                        SourceFinding {
                            severity: Severity::Error,
                            title: "ts: value import used only as a type",
                            detail: format!(
                                "line {}: `{name}` is imported as a value (`import {{ {name} }}`) \
                                 but used only in type position; use `import type {{ {name} }}` \
                                 instead.",
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

/// FE-FSM-1.2 — explicit FSM transitions, FE-facing rule (T1, via d16): an
/// ad-hoc `this.status = "..."` string mutation with no `as const`
/// transition map routed through `assertTransition(from, to)` anywhere in
/// the file is flagged; a file that declares an `as const` transitions map
/// AND routes its mutation through `assertTransition(`/`assert_transition(`
/// stays clean. This validator does NOT redefine the transition engine —
/// it only recognizes the FE-specific TSX/TS marker shapes, per the
/// workpack's "consume d16, don't re-implement" instruction; the shared
/// FSM semantics live in `enforcer_lang_common::rules::fsm`.
#[derive(Debug)]
#[doc = "Frontend FSM transition validator."]
pub struct ExplicitFsmTransitionsValidator {
    rule_id: RuleId,
}

impl ExplicitFsmTransitionsValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-FSM-1.2")?,
        })
    }
}

const RAW_STATUS_ASSIGN_MARKERS: &[&str] = &["this.status = \"", "self.status = \""];
const ASSERT_TRANSITION_MARKERS: &[&str] = &["assertTransition(", "assert_transition("];

impl Validator for ExplicitFsmTransitionsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_transition_map = input.source.as_str().contains("as const")
            && (input.source.as_str().contains("transitions = {")
                || input.source.as_str().contains("transitions: {"));
        let routes_through_assert = ASSERT_TRANSITION_MARKERS
            .iter()
            .any(|m| input.source.as_str().contains(m));
        if has_transition_map && routes_through_assert {
            return Vec::new();
        }
        for line in lines(input.source) {
            if source_line_role(line.text) == SourceLineRole::CommentOnly {
                continue;
            }
            if RAW_STATUS_ASSIGN_MARKERS
                .iter()
                .any(|m| line.text.as_str().contains(m))
            {
                return finding(
                    &self.rule_id,
                    &input,
                    SourceFinding {
                        severity: Severity::Error,
                        title: "fsm: ad-hoc status assignment with no explicit transition map",
                        detail: format!(
                            "line {}: a raw `this.status = \"...\"`/`self.status = \"...\"` string \
                             mutation is present with no `as const` transitions map routed through \
                             `assertTransition(from, to)`; declare the allowed transitions \
                             explicitly and route every mutation through the transition guard.",
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

/// FE-EFFECT-1.1 — Effect-not-Zod (T1, the divergence rule): any Zod usage
/// in the validated code (`from "zod"`, `z.object(`, `zodResolver`) is
/// flagged as a violation mandating Effect Schema; boundary validation via
/// `@effect/schema` (`Schema.Struct`) stays clean. See this module's top
/// doc comment / the workpack's doctrine-divergence note: this is the
/// DELIBERATE inversion of ADBP's `FE-TS-1.11` (Zod-as-SoT) — never
/// "restore parity" by dropping this rule or re-permitting Zod.
#[derive(Debug)]
#[doc = "Effect Schema source-of-truth validator."]
pub struct EffectNotZodValidator {
    rule_id: RuleId,
}

impl EffectNotZodValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id("FE-EFFECT-1.1")?,
        })
    }
}

impl Validator for EffectNotZodValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if let Some(evidence) = recognize_schema_framework(input.source).filter(|evidence| {
            evidence.family()
                == enforcer_domain::doctrine_profile_types::DoctrineFrameworkFamily::Zod
        }) {
            let marker = evidence.marker();
            let capability = evidence.capability();
            let line = evidence.line();
            return finding(
                &self.rule_id,
                &input,
                SourceFinding {
                        severity: Severity::Error,
                        title: "effect: Zod usage forbidden, Effect Schema mandated",
                        detail: format!(
                        "line {}: `{}` ({capability:?}) — Zod is forbidden as a validation source-of-truth \
                             in this codebase (house doctrine diverges from ADBP's Zod mandate); use \
                             Effect Schema (`import {{ Schema }} from \"@effect/schema\"`, \
                             `Schema.Struct`) instead.",
                        line,
                        marker.as_str()
                        ),
                    line,
                    snippet: None,
                },
            )
            .into_iter()
            .collect();
        }
        Vec::new()
    }
}

/// Build every `FE-*` frontend-react family validator this module owns.
/// Wired through the standalone `enforcer-rules` catalog
/// (`rules/frontend-react.json`), NOT `registry::build_all` (see module
/// docs).
pub fn validators(
) -> Result<Vec<Box<dyn Validator>>, enforcer_domain::boundary::decode_error::DecodeError> {
    Ok(vec![
        Box::new(FeatureBoundaryValidator::new()?),
        Box::new(ComponentsFeatureInversionValidator::new()?),
        Box::new(NoServerDataInClientStoreValidator::new()?),
        Box::new(NoFetchInUseEffectValidator::new()?),
        Box::new(UseEffectWhyCommentValidator::new()?),
        Box::new(TypedErrorsInServicesValidator::new()?),
        Box::new(ImageAltValidator::new()?),
        Box::new(InputLabelValidator::new()?),
        Box::new(EnvCentralizationValidator::new()?),
        Box::new(NoExplicitAnyValidator::new()?),
        Box::new(TypeOnlyImportValidator::new()?),
        Box::new(ExplicitFsmTransitionsValidator::new()?),
        Box::new(EffectNotZodValidator::new()?),
    ])
}

#[cfg(test)]
mod tests {
    use crate::boundary::test_fixtures::run_fixture_parity;

    use super::{
        validators, ComponentsFeatureInversionValidator, EffectNotZodValidator,
        EnvCentralizationValidator, ExplicitFsmTransitionsValidator, FeatureBoundaryValidator,
        ImageAltValidator, InputLabelValidator, NoExplicitAnyValidator,
        NoFetchInUseEffectValidator, NoServerDataInClientStoreValidator, TypeOnlyImportValidator,
        TypedErrorsInServicesValidator, UseEffectWhyCommentValidator, ValidationInput, Validator,
    };

    #[test]
    fn thirteen_validators_registered_with_unique_rule_ids(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let vs = validators()?;
        assert_eq!(vs.len(), 13);
        let mut seen = std::collections::BTreeSet::new();
        for v in &vs {
            assert!(seen.insert(v.rule_id().to_string()));
        }
        assert_eq!(seen.len(), 13);
        Ok(())
    }

    #[test]
    fn fe_arch_feature_boundary() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FeatureBoundaryValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/arch-1.3/features/checkout/fail.tsx",
            "tests/fixtures/frontend_react/arch-1.3/features/checkout/pass.tsx",
        )?;
        Ok(())
    }

    #[test]
    fn fe_arch_components_feature_inversion() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ComponentsFeatureInversionValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/arch-1.4/components/fail.tsx",
            "tests/fixtures/frontend_react/arch-1.4/components/pass.tsx",
        )?;
        Ok(())
    }

    #[test]
    fn fe_state_no_server_data_in_client_store() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoServerDataInClientStoreValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/state-1.1/fail.ts",
            "tests/fixtures/frontend_react/state-1.1/pass.ts",
        )?;
        Ok(())
    }

    #[test]
    fn fe_state_no_fetch_in_use_effect() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoFetchInUseEffectValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/state-1.2/fail.tsx",
            "tests/fixtures/frontend_react/state-1.2/pass.tsx",
        )?;
        Ok(())
    }

    #[test]
    fn fe_hook_use_effect_why_comment() -> Result<(), Box<dyn std::error::Error>> {
        let validator = UseEffectWhyCommentValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/hook-1.2/fail.tsx",
            "tests/fixtures/frontend_react/hook-1.2/pass.tsx",
        )?;
        Ok(())
    }

    #[test]
    fn fe_pat_typed_errors_in_services() -> Result<(), Box<dyn std::error::Error>> {
        let validator = TypedErrorsInServicesValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/pat-1.4/services/fail.ts",
            "tests/fixtures/frontend_react/pat-1.4/services/pass.ts",
        )?;
        Ok(())
    }

    #[test]
    fn fe_cmp_image_alt() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ImageAltValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/cmp-1.12/fail.tsx",
            "tests/fixtures/frontend_react/cmp-1.12/pass.tsx",
        )?;
        Ok(())
    }

    #[test]
    fn fe_a11y_input_label() -> Result<(), Box<dyn std::error::Error>> {
        let validator = InputLabelValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/a11y-1.3/fail.tsx",
            "tests/fixtures/frontend_react/a11y-1.3/pass.tsx",
        )?;
        Ok(())
    }

    #[test]
    fn fe_cfg_env_centralization() -> Result<(), Box<dyn std::error::Error>> {
        let validator = EnvCentralizationValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/cfg-1.1/other/fail.ts",
            "tests/fixtures/frontend_react/cfg-1.1/other/pass.ts",
        )?;
        Ok(())
    }

    /// Confirms the `lib/env.ts` exemption is a PATH exemption, not a
    /// blanket "no findings" bug: this fixture itself reads raw env vars
    /// AND lives at `.../lib/env.ts`, so it must stay clean precisely
    /// because of its path, not because the detector is broken.
    #[test]
    fn fe_cfg_env_centralization_lib_env_itself_is_exempt() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = EnvCentralizationValidator::new()?;
        let root = crate::boundary::test_fixtures::fixture_root()?;
        let file = crate::boundary::test_fixtures::fixture_path(
            "tests/fixtures/frontend_react/cfg-1.1/lib/env.ts",
        )?;
        let source = std::fs::read_to_string(root.resolve(&file))?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(
            findings.is_empty(),
            "lib/env.ts itself must be exempt from FE-CFG-1.1: {findings:#?}"
        );
        Ok(())
    }

    #[test]
    fn fe_ts_no_explicit_any() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoExplicitAnyValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/ts-1.5/fail.ts",
            "tests/fixtures/frontend_react/ts-1.5/pass.ts",
        )?;
        Ok(())
    }

    #[test]
    fn fe_ts_type_only_import() -> Result<(), Box<dyn std::error::Error>> {
        let validator = TypeOnlyImportValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/ts-1.14/fail.ts",
            "tests/fixtures/frontend_react/ts-1.14/pass.ts",
        )?;
        Ok(())
    }

    #[test]
    fn fe_fsm_explicit_transitions() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ExplicitFsmTransitionsValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/fsm-1.2/fail.ts",
            "tests/fixtures/frontend_react/fsm-1.2/pass.ts",
        )?;
        Ok(())
    }

    #[test]
    fn fe_effect_not_zod() -> Result<(), Box<dyn std::error::Error>> {
        let validator = EffectNotZodValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/effect-1.1/fail.ts",
            "tests/fixtures/frontend_react/effect-1.1/pass.ts",
        )?;
        Ok(())
    }

    /// Pins the ADBP-form re-expression: React Hook Form + `zodResolver`
    /// must ALSO flag (not just a bare `z.object`), and its Effect-based
    /// replacement (`effectResolver`) stays clean.
    #[test]
    fn fe_effect_not_zod_form_resolver() -> Result<(), Box<dyn std::error::Error>> {
        let validator = EffectNotZodValidator::new()?;
        run_fixture_parity(
            &validator,
            "tests/fixtures/frontend_react/effect-1.1-form/fail.tsx",
            "tests/fixtures/frontend_react/effect-1.1-form/pass.tsx",
        )?;
        Ok(())
    }
}
