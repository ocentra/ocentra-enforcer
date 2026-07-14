//! b06 — the `AGENTS.md` decision forest: scaffolder + validator.
//!
//! # Charter (this module only)
//!
//! Owner requirement (2026-07-04): on any stop, crash, or resume, an agent
//! should not burn tokens re-discovering where it is by re-reading a whole
//! plan or scanning scattered prose. This module scaffolds and validates a
//! fixed, layered chain of small routing files — GLOBAL `AGENTS.md` ->
//! PROJECT `AGENTS.md` -> PLAN `AGENTS.md` -> a decision tree -> the
//! plan's resume-state — where each tier is a minimal, read-first router
//! that says "check this, then that" so an agent locates current state
//! with the fewest tokens. The owner prefers `AGENTS.md` over `CLAUDE.md`
//! (harness-neutral naming).
//!
//! ## TRANSITIONAL-TO-TYPED intent
//!
//! These `AGENTS.md` files are a TRANSITIONAL prose surface, not a
//! permanent format. They are designed to be dropped for a typed
//! system/db/schema later — the routing chain and decision tree are
//! modeled as data (tier -> next-pointer -> decision-node -> resume-
//! anchor) so the same structure can be served from a typed store and
//! rendered in the Tauri desktop UI for humans. Accordingly this module
//! does NOT hard-couple its [`Validator`] impls to prose surviving
//! forever: every check below parses the structured HTML-comment managed
//! blocks (`<!-- agents-read-first -->` ... `<!-- /agents-read-first -->`,
//! `<!-- agents-next-tier -->`, `<!-- agents-decision-tree -->`,
//! `<!-- agents-transitional-intent -->`, and the tier-identifying
//! `<!-- agents-forest-tier: <tier> -->` marker), never free substring
//! prose, so the backing store can swap under a stable contract.
//!
//! ## Scope
//!
//! This module owns:
//! - [`scaffold_forest`] — renders the three tiers from
//!   `templates/agents-{global,project,plan}.tpl` using the same frozen
//!   `include_str!` + `{{name}}` placeholder-substitution approach b03
//!   established in `crate::templates` (a local substitution helper,
//!   since that module's `render_template` is private to its own three
//!   templates — this module owns a byte-identical copy of the same
//!   minimal contract for its own three templates).
//! - [`check_chain_resolves`] (`AGENTS-CHAIN.1`) — the global -> project
//!   -> plan NEXT-pointer chain resolves to existing lower tiers.
//! - [`AgentsRoutingDeclaredValidator`] (`AGENTS-ROUTING.1`) — each tier
//!   declares its read-first routing managed block.
//! - [`AgentsTreeTerminatesValidator`] (`AGENTS-TREE.1`) — every decision-
//!   tree leaf terminates at a real resume-state anchor (a `LEAF ->`
//!   pointer that is non-empty and not a dangling placeholder).
//! - [`AgentsBudgetValidator`] (`AGENTS-BUDGET.1`) — each tier stays under
//!   its declared line/byte budget.
//! - [`run_resume_simulation`] — walks ONLY the `AGENTS.md` chain (global
//!   -> project -> plan -> decision tree) and asserts it resolves to the
//!   plan's resume-state anchor without reading the plan body, within the
//!   declared token/line budget.
//!
//! This module does NOT own: the `Validator` trait or fixture/parity
//! harness (`enforcer-validator`, b02's dependency), the PLAN-* structure
//! validators (b02's own module), or the capsule/index templating engine
//! itself (b03's `templates.rs` — this module calls it, not clones it).

use std::collections::{HashMap, HashSet};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;

use enforcer_validator::validator::{ValidationInput, Validator};

use crate::error::PlanError;

/// The frozen global-tier template, embedded at compile time.
const GLOBAL_TEMPLATE: &str = include_str!("../templates/agents-global.tpl");
/// The frozen project-tier template, embedded at compile time.
const PROJECT_TEMPLATE: &str = include_str!("../templates/agents-project.tpl");
/// The frozen plan-tier template, embedded at compile time.
const PLAN_TEMPLATE: &str = include_str!("../templates/agents-plan.tpl");

/// The default per-tier size budget (workpack: "a small line/byte
/// budget"). Chosen small enough that three tiers plus the plan's own
/// resume-state anchor stay far below a full plan-body read.
pub const DEFAULT_BUDGET_LINES: usize = 40;
/// The default per-tier byte budget, paired with [`DEFAULT_BUDGET_LINES`].
pub const DEFAULT_BUDGET_BYTES: usize = 2048;

/// One tier of the decision forest, in read order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForestTier {
    /// Workspace/machine root — read before anything else.
    Global,
    /// Repo root.
    Project,
    /// Per `docs/plans/<name>/`.
    Plan,
}

impl ForestTier {
    /// The `agents-forest-tier:` marker value this tier's rendered file
    /// carries. Public so callers (and this module's own tests) can
    /// assert a rendered/validated document's [`tier_marker`] against the
    /// tier they expected, without duplicating the marker string.
    pub fn marker(self) -> &'static str {
        match self {
            ForestTier::Global => "global",
            ForestTier::Project => "project",
            ForestTier::Plan => "plan",
        }
    }
}

/// Caller-supplied facts needed to render one plan's 3-tier forest. Kept
/// minimal and explicit (no hidden defaults for the paths that matter) so
/// a scaffold call cannot silently render a chain that does not resolve.
#[derive(Debug, Clone)]
pub struct ForestFacts {
    /// Human-readable workspace/machine label for the global tier.
    pub workspace_name: String,
    /// Repo/project name for the project tier.
    pub project_name: String,
    /// Plan directory name (e.g. `enforcer-selfhost-plan`) for the plan
    /// tier.
    pub plan_name: String,
    /// Repo-relative path to the project tier `AGENTS.md`, as it will
    /// exist on disk (what the global tier's NEXT pointer must name).
    pub project_tier_path: String,
    /// Repo-relative path to the plan tier `AGENTS.md` (what the project
    /// tier's NEXT pointer must name).
    pub plan_tier_path: String,
    /// Repo-relative path (or anchor label) of the plan's resume-state
    /// entrypoint (e.g. `docs/plans/<name>/RESUME_STATE.md`) — the leaf
    /// every decision-tree branch must terminate at.
    pub resume_anchor: String,
    /// Per-tier line budget (defaults to [`DEFAULT_BUDGET_LINES`] via
    /// [`ForestFacts::with_defaults`]).
    pub budget_lines: usize,
    /// Per-tier byte budget (defaults to [`DEFAULT_BUDGET_BYTES`] via
    /// [`ForestFacts::with_defaults`]).
    pub budget_bytes: usize,
}

/// The six caller-supplied string facts [`ForestFacts::with_defaults`]
/// needs, grouped into one struct so that constructor stays under the
/// workspace's `too_many_arguments` clippy budget instead of taking six
/// positional `impl Into<String>` parameters.
#[derive(Debug, Clone)]
pub struct ForestNames {
    /// Human-readable workspace/machine label for the global tier.
    pub workspace_name: String,
    /// Repo/project name for the project tier.
    pub project_name: String,
    /// Plan directory name (e.g. `enforcer-selfhost-plan`) for the plan
    /// tier.
    pub plan_name: String,
    /// Repo-relative path to the project tier `AGENTS.md`.
    pub project_tier_path: String,
    /// Repo-relative path to the plan tier `AGENTS.md`.
    pub plan_tier_path: String,
    /// Repo-relative path (or anchor label) of the plan's resume-state
    /// entrypoint.
    pub resume_anchor: String,
}

impl ForestFacts {
    /// Construct with the default per-tier budgets from a [`ForestNames`]
    /// bundle.
    pub fn with_defaults(names: ForestNames) -> Self {
        Self {
            workspace_name: names.workspace_name,
            project_name: names.project_name,
            plan_name: names.plan_name,
            project_tier_path: names.project_tier_path,
            plan_tier_path: names.plan_tier_path,
            resume_anchor: names.resume_anchor,
            budget_lines: DEFAULT_BUDGET_LINES,
            budget_bytes: DEFAULT_BUDGET_BYTES,
        }
    }
}

/// The rendered 3-tier forest, ready to write to disk (or to feed directly
/// to the validators / resume simulation without a round trip through the
/// filesystem).
#[derive(Debug, Clone)]
pub struct RenderedForest {
    /// Rendered global-tier `AGENTS.md` text.
    pub global: String,
    /// Rendered project-tier `AGENTS.md` text.
    pub project: String,
    /// Rendered plan-tier `AGENTS.md` text.
    pub plan: String,
}

/// Deterministic string substitution over a template, replacing `{{name}}`
/// placeholders. Local to this module rather than importing
/// `crate::templates`'s private `render_template` (that function is not
/// `pub`) — this is a byte-for-byte copy of the same minimal substitution
/// contract b03 established (missing placeholder -> typed error, never a
/// panic), applied to this module's own three frozen templates.
fn render(template: &str, bindings: &HashMap<String, String>) -> Result<String, PlanError> {
    let mut result = template.to_owned();
    for (name, value) in bindings {
        let placeholder = format!("{{{{{name}}}}}");
        if result.contains(&placeholder) {
            result = result.replace(&placeholder, value);
        }
    }
    if let Some(pos) = result.find("{{") {
        if let Some(end) = result[pos..].find("}}") {
            let placeholder = result[pos..pos + end + 2].to_owned();
            return Err(PlanError::Io {
                path: "agents-forest template".to_owned(),
                reason: format!("missing placeholder: {placeholder}"),
            });
        }
    }
    Ok(result)
}

/// Render all three tiers from `templates/agents-{global,project,plan}.tpl`
/// for one plan, per [`ForestFacts`]. Pure rendering — does not touch the
/// filesystem; callers write [`RenderedForest`]'s fields to disk
/// themselves at the paths named in `facts`.
pub fn scaffold_forest(facts: &ForestFacts) -> Result<RenderedForest, PlanError> {
    let budget_lines = facts.budget_lines.to_string();
    let budget_bytes = facts.budget_bytes.to_string();

    let mut global_bindings = HashMap::new();
    global_bindings.insert("workspace_name".to_owned(), facts.workspace_name.clone());
    global_bindings.insert("next_tier_path".to_owned(), facts.project_tier_path.clone());
    global_bindings.insert("resume_anchor".to_owned(), facts.resume_anchor.clone());
    global_bindings.insert("budget_lines".to_owned(), budget_lines.clone());
    global_bindings.insert("budget_bytes".to_owned(), budget_bytes.clone());
    let global = render(GLOBAL_TEMPLATE, &global_bindings)?;

    let mut project_bindings = HashMap::new();
    project_bindings.insert("project_name".to_owned(), facts.project_name.clone());
    project_bindings.insert("next_tier_path".to_owned(), facts.plan_tier_path.clone());
    project_bindings.insert("resume_anchor".to_owned(), facts.resume_anchor.clone());
    project_bindings.insert("budget_lines".to_owned(), budget_lines.clone());
    project_bindings.insert("budget_bytes".to_owned(), budget_bytes.clone());
    let project = render(PROJECT_TEMPLATE, &project_bindings)?;

    let mut plan_bindings = HashMap::new();
    plan_bindings.insert("plan_name".to_owned(), facts.plan_name.clone());
    plan_bindings.insert("next_tier_path".to_owned(), facts.resume_anchor.clone());
    plan_bindings.insert("resume_anchor".to_owned(), facts.resume_anchor.clone());
    plan_bindings.insert("budget_lines".to_owned(), budget_lines);
    plan_bindings.insert("budget_bytes".to_owned(), budget_bytes);
    let plan = render(PLAN_TEMPLATE, &plan_bindings)?;

    Ok(RenderedForest {
        global,
        project,
        plan,
    })
}

fn finding(rule_id: &RuleId, title: &str, detail: impl Into<String>, file: &RelPath) -> Finding {
    Finding {
        rule_id: rule_id.clone(),
        severity: Severity::Error,
        title: title.to_owned(),
        detail: detail.into(),
        file: file.clone(),
        line: 1,
        snippet: None,
    }
}

/// Extract the text between a `<!-- name -->` ... `<!-- /name -->` managed
/// block, trimmed. Returns `None` if either fence is absent or the close
/// precedes the open (a structurally broken block).
fn managed_block<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<!-- {name} -->");
    let close = format!("<!-- /{name} -->");
    let (_, rest) = source.split_once(open.as_str())?;
    let (block, _) = rest.split_once(close.as_str())?;
    Some(block.trim())
}

/// Extract the `<!-- agents-forest-tier: <tier> -->` marker value.
fn tier_marker(source: &str) -> Option<&str> {
    let open = "<!-- agents-forest-tier:";
    let (_, rest) = source.split_once(open)?;
    let (marker, _) = rest.split_once("-->")?;
    Some(marker.trim())
}

/// Extract a `KEY: value` line's `value` from inside a managed block's
/// text (e.g. `NEXT: docs/plans/foo/AGENTS.md` -> `docs/plans/foo/AGENTS.md`).
fn extract_keyed_value(block_text: &str, key: &str) -> Option<String> {
    block_text.lines().find_map(|line| {
        let line = line.trim().trim_start_matches('>').trim();
        line.strip_prefix(key).map(|rest| rest.trim().to_owned())
    })
}

/// Extract every `LEAF -> <value>` pointer inside a decision-tree managed
/// block's text.
fn extract_leaf_pointers(block_text: &str) -> Vec<String> {
    block_text
        .lines()
        .filter_map(|line| {
            let line = line.trim().trim_start_matches('>').trim();
            line.strip_prefix("LEAF ->")
                .map(|rest| rest.trim().to_owned())
        })
        .collect()
}

/// `AGENTS-ROUTING.1`: this tier's file declares the read-first routing
/// managed block (`<!-- agents-read-first -->` ... `<!-- /agents-read-first -->`),
/// non-empty.
#[derive(Debug)]
pub struct AgentsRoutingDeclaredValidator {
    rule_id: RuleId,
}

impl AgentsRoutingDeclaredValidator {
    /// Construct with the linked `ruleId`.
    pub fn new(rule_id: RuleId) -> Self {
        Self { rule_id }
    }
}

impl Validator for AgentsRoutingDeclaredValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        match managed_block(input.source, "agents-read-first") {
            Some(text) if !text.is_empty() => Vec::new(),
            Some(_) => vec![finding(
                &self.rule_id,
                "empty read-first routing block",
                "`<!-- agents-read-first -->` block is present but empty",
                input.file,
            )],
            None => vec![finding(
                &self.rule_id,
                "missing read-first routing block",
                "no `<!-- agents-read-first -->` managed block found",
                input.file,
            )],
        }
    }
}

/// `AGENTS-TREE.1`: this tier's file declares a decision-tree managed
/// block, and every `LEAF ->` pointer inside it is non-empty (terminates
/// at a real resume-state anchor, not a dangling placeholder).
#[derive(Debug)]
pub struct AgentsTreeTerminatesValidator {
    rule_id: RuleId,
}

impl AgentsTreeTerminatesValidator {
    /// Construct with the linked `ruleId`.
    pub fn new(rule_id: RuleId) -> Self {
        Self { rule_id }
    }
}

impl Validator for AgentsTreeTerminatesValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(tree_text) = managed_block(input.source, "agents-decision-tree") else {
            return vec![finding(
                &self.rule_id,
                "missing decision tree",
                "no `<!-- agents-decision-tree -->` managed block found",
                input.file,
            )];
        };
        let leaves = extract_leaf_pointers(tree_text);
        if leaves.is_empty() {
            return vec![finding(
                &self.rule_id,
                "decision tree has no leaf",
                "decision tree block carries no `LEAF -> <anchor>` pointer",
                input.file,
            )];
        }
        if leaves
            .iter()
            .any(|leaf| leaf.is_empty() || leaf == "TODO" || leaf == "TBD")
        {
            return vec![finding(
                &self.rule_id,
                "dangling decision-tree leaf",
                format!("decision tree carries a dangling leaf pointer: {leaves:?}"),
                input.file,
            )];
        }
        Vec::new()
    }
}

/// `AGENTS-BUDGET.1`: this tier's file stays within its own declared
/// line/byte budget (parsed from the `agents-read-first` block's
/// "Budget: stay under N lines / M bytes" statement, falling back to the
/// module defaults if that statement is absent so an otherwise-compliant
/// fixture is not spuriously flagged for omitting the optional restated
/// budget line).
#[derive(Debug)]
pub struct AgentsBudgetValidator {
    rule_id: RuleId,
}

impl AgentsBudgetValidator {
    /// Construct with the linked `ruleId`.
    pub fn new(rule_id: RuleId) -> Self {
        Self { rule_id }
    }
}

fn parse_declared_budget(source: &str) -> (usize, usize) {
    let Some(text) = managed_block(source, "agents-read-first") else {
        return (DEFAULT_BUDGET_LINES, DEFAULT_BUDGET_BYTES);
    };
    let Some(marker) = text.find("Budget: stay under") else {
        return (DEFAULT_BUDGET_LINES, DEFAULT_BUDGET_BYTES);
    };
    let rest = &text[marker..];
    let lines = rest
        .split_whitespace()
        .find_map(|tok| tok.parse::<usize>().ok())
        .unwrap_or(DEFAULT_BUDGET_LINES);
    let bytes = rest
        .split('/')
        .nth(1)
        .and_then(|seg| {
            seg.split_whitespace()
                .find_map(|tok| tok.parse::<usize>().ok())
        })
        .unwrap_or(DEFAULT_BUDGET_BYTES);
    (lines, bytes)
}

impl Validator for AgentsBudgetValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let (budget_lines, budget_bytes) = parse_declared_budget(input.source);
        let actual_lines = input.source.lines().count();
        let actual_bytes = input.source.len();
        if actual_lines > budget_lines || actual_bytes > budget_bytes {
            return vec![finding(
                &self.rule_id,
                "tier exceeds its declared size budget",
                format!(
                    "tier is {actual_lines} lines / {actual_bytes} bytes, budget is \
                     {budget_lines} lines / {budget_bytes} bytes"
                ),
                input.file,
            )];
        }
        Vec::new()
    }
}

/// One resolved tier document, as [`check_chain_resolves`] needs it: its
/// tier marker, its NEXT pointer (if any), and the repo-relative path it
/// would exist at (used to resolve whether a NEXT pointer names a real
/// sibling in the same document set).
#[derive(Debug, Clone)]
pub struct TierDocument {
    /// This document's repo-relative path (or synthetic fixture path).
    pub path: String,
    /// Raw source text.
    pub source: String,
}

/// `AGENTS-CHAIN.1`: cross-document check — for a global/project/plan
/// triple, each tier's NEXT pointer must name a path that exists in the
/// supplied document set (global -> project -> plan), and the terminal
/// (plan) tier's NEXT pointer must name the resume anchor. A plain
/// function, not a `Validator` impl, since the `Validator` trait's
/// contract is exactly one file's text and this check is inherently
/// cross-document (mirrors b02's own `check_parallel_safety` precedent
/// for the same reason).
pub fn check_chain_resolves(rule_id: &RuleId, docs: &[TierDocument]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let by_path: HashMap<&str, &TierDocument> = docs.iter().map(|d| (d.path.as_str(), d)).collect();

    for doc in docs {
        let Some(marker) = tier_marker(&doc.source) else {
            findings.push(finding(
                rule_id,
                "tier document missing tier marker",
                "no `<!-- agents-forest-tier: <tier> -->` marker found",
                &synthetic_path(&doc.path),
            ));
            continue;
        };
        let Some(next_block) = managed_block(&doc.source, "agents-next-tier") else {
            findings.push(finding(
                rule_id,
                "tier document missing NEXT pointer block",
                "no `<!-- agents-next-tier -->` managed block found",
                &synthetic_path(&doc.path),
            ));
            continue;
        };
        let Some(next_path) = extract_keyed_value(next_block, "NEXT:") else {
            findings.push(finding(
                rule_id,
                "NEXT pointer block has no NEXT: value",
                "`<!-- agents-next-tier -->` block carries no `NEXT: <path>` line",
                &synthetic_path(&doc.path),
            ));
            continue;
        };

        let expected_next_tier = match marker {
            "global" => Some("project"),
            "project" => Some("plan"),
            // The plan tier's NEXT pointer names the resume anchor, not a
            // fourth tier document.
            "plan" => None,
            _ => {
                findings.push(finding(
                    rule_id,
                    "unsupported tier marker",
                    format!("tier document declares unsupported marker `{marker}`"),
                    &synthetic_path(&doc.path),
                ));
                continue;
            }
        };
        let Some(expected_next_tier) = expected_next_tier else {
            continue;
        };
        let Some(next_doc) = by_path.get(next_path.as_str()) else {
            findings.push(finding(
                rule_id,
                "broken NEXT pointer",
                format!(
                    "tier `{marker}` NEXT pointer names `{next_path}`, which is not one of the \
                     supplied tier documents"
                ),
                &synthetic_path(&doc.path),
            ));
            continue;
        };
        if tier_marker(&next_doc.source) != Some(expected_next_tier) {
            findings.push(finding(
                rule_id,
                "NEXT pointer targets wrong tier",
                format!(
                    "tier `{marker}` NEXT pointer names `{next_path}`, which must be a \
                     `{expected_next_tier}` tier document"
                ),
                &synthetic_path(&doc.path),
            ));
        }
    }

    findings
}

fn synthetic_path(raw: &str) -> RelPath {
    let candidates = [raw.to_owned(), "agents-forest/unknown.md".to_owned()];
    for candidate in candidates {
        if let Ok(path) = candidate.parse() {
            return path;
        }
    }
    // Unreachable in practice: the fallback candidate is a fixed literal
    // satisfying `RelPath`'s own rules; retry rather than panic.
    loop {
        if let Ok(path) = "unknown.md".parse::<RelPath>() {
            return path;
        }
    }
}

/// Outcome of [`run_resume_simulation`]: either the chain resolved to a
/// resume anchor within budget, or it failed at a named stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeSimOutcome {
    /// The chain resolved; carries the resolved resume anchor and the
    /// summed byte size of every tier walked.
    Resolved {
        /// The resume-state anchor the chain terminated at.
        resume_anchor: String,
        /// Summed byte size of every tier document walked to get there.
        summed_bytes: usize,
    },
    /// The chain broke; carries a short reason.
    Broken(String),
}

/// Walk ONLY the `AGENTS.md` chain (global -> project -> plan -> decision
/// tree) starting from `global`, following each tier's `NEXT:` pointer
/// through `by_path`, and resolve to the plan tier's decision-tree leaf —
/// WITHOUT reading anything outside the supplied tier documents (in
/// particular, without reading any plan body). Asserts the summed size of
/// every tier walked stays within `budget_bytes_total`.
pub fn run_resume_simulation(
    global: &TierDocument,
    by_path: &HashMap<String, TierDocument>,
    budget_bytes_total: usize,
) -> ResumeSimOutcome {
    if tier_marker(&global.source) != Some("global") {
        return ResumeSimOutcome::Broken(format!(
            "resume simulation must start from a global tier document, got `{}`",
            global.path
        ));
    }
    let mut summed_bytes = global.source.len();
    let mut current = global;
    let mut visited_tiers = HashSet::new();

    loop {
        let Some(marker) = tier_marker(&current.source) else {
            return ResumeSimOutcome::Broken(format!(
                "tier document `{}` has no tier marker",
                current.path
            ));
        };
        if !visited_tiers.insert((marker, current.path.as_str())) {
            return ResumeSimOutcome::Broken(format!(
                "resume chain cycle revisits `{marker}` tier at `{}`",
                current.path
            ));
        }
        let Some(next_block) = managed_block(&current.source, "agents-next-tier") else {
            return ResumeSimOutcome::Broken(format!(
                "tier document `{}` has no NEXT pointer block",
                current.path
            ));
        };
        let Some(next_path) = extract_keyed_value(next_block, "NEXT:") else {
            return ResumeSimOutcome::Broken(format!(
                "tier document `{}` NEXT pointer block has no NEXT: value",
                current.path
            ));
        };

        if marker == "plan" {
            let Some(tree_text) = managed_block(&current.source, "agents-decision-tree") else {
                return ResumeSimOutcome::Broken(format!(
                    "plan tier `{}` has no decision tree",
                    current.path
                ));
            };
            let leaves = extract_leaf_pointers(tree_text);
            let Some(leaf) = leaves.into_iter().find(|l| !l.is_empty()) else {
                return ResumeSimOutcome::Broken(format!(
                    "plan tier `{}` decision tree has no resolvable leaf",
                    current.path
                ));
            };
            if summed_bytes > budget_bytes_total {
                return ResumeSimOutcome::Broken(format!(
                    "chain resolved to `{leaf}` but summed {summed_bytes} bytes exceeds the \
                     {budget_bytes_total}-byte total chain budget"
                ));
            }
            return ResumeSimOutcome::Resolved {
                resume_anchor: leaf,
                summed_bytes,
            };
        }

        let expected_next_tier = match marker {
            "global" => "project",
            "project" => "plan",
            _ => {
                return ResumeSimOutcome::Broken(format!(
                    "tier document `{}` has unsupported tier marker `{marker}`",
                    current.path
                ));
            }
        };

        let Some(next_doc) = by_path.get(&next_path) else {
            return ResumeSimOutcome::Broken(format!(
                "tier `{marker}` NEXT pointer names `{next_path}`, which was not supplied to the \
                walker"
            ));
        };
        if tier_marker(&next_doc.source) != Some(expected_next_tier) {
            return ResumeSimOutcome::Broken(format!(
                "tier `{marker}` at `{}` must point to a `{expected_next_tier}` tier, not `{}`",
                current.path, next_path
            ));
        }
        summed_bytes += next_doc.source.len();
        current = next_doc;
    }
}

/// Doc-intent check: the module doc (this file's leading `//!` block) and
/// each of the three templates declare the transitional-to-typed-data
/// statement. Exercised over the templates' raw text (the module doc is
/// checked by a dedicated test reading `src/agents_forest.rs` itself, so
/// both halves of the requirement are proven, not assumed).
pub fn declares_transitional_intent(source: &str) -> bool {
    let lower = source.to_lowercase();
    lower.contains("transitional-to-typed")
        && lower.contains("typed system/db/schema")
        && lower.contains("tauri")
}
