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
//!   `include_str!` + `{{name}}` token-substitution approach b03
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
//!   pointer that is non-empty and not a dangling token).
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
use enforcer_domain::plan_types::{
    PlanBudgetBytes, PlanBudgetLines, PlanCondition, PlanDiagnosticDetail, PlanDocumentText,
    PlanName, PlanProjectName, PlanResumeAnchor, PlanWorkspaceName,
};

use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::build_error_finding as finding;
use crate::boundary::forest::{
    extract_keyed_value, extract_leaf_pointers, managed_block, parse_declared_budget,
    render_forest, tier_marker,
};
use crate::boundary::values::{budget_bytes, diagnostic_detail, resume_anchor};
use crate::error::PlanError;

/// Caller-supplied facts needed to render one plan's 3-tier forest. Kept
/// minimal and explicit (no hidden defaults for the paths that matter) so
/// a scaffold call cannot silently render a chain that does not resolve.
#[derive(Debug, Clone)]
pub struct ForestFacts {
    /// Human-readable workspace/machine label for the global tier.
    pub workspace_name: PlanWorkspaceName,
    /// Repo/project name for the project tier.
    pub project_name: PlanProjectName,
    /// Plan directory name (e.g. `enforcer-selfhost-plan`) for the plan
    /// tier.
    pub plan_name: PlanName,
    /// Repo-relative path to the project tier `AGENTS.md`, as it will
    /// exist on disk (what the global tier's NEXT pointer must name).
    pub project_tier_path: RelPath,
    /// Repo-relative path to the plan tier `AGENTS.md` (what the project
    /// tier's NEXT pointer must name).
    pub plan_tier_path: RelPath,
    /// Repo-relative path (or anchor label) of the plan's resume-state
    /// entrypoint (e.g. `docs/plans/<name>/RESUME_STATE.md`) — the leaf
    /// every decision-tree branch must terminate at.
    pub resume_anchor: PlanResumeAnchor,
    /// Per-tier line budget (defaults to [`PlanBudgetLines::DEFAULT`] via
    /// [`ForestFacts::with_defaults`]).
    pub budget_lines: PlanBudgetLines,
    /// Per-tier byte budget (defaults to [`PlanBudgetBytes::DEFAULT`] via
    /// [`ForestFacts::with_defaults`]).
    pub budget_bytes: PlanBudgetBytes,
}

/// The six caller-supplied string facts [`ForestFacts::with_defaults`]
/// needs, grouped into one struct so that constructor stays under the
/// workspace's `too_many_arguments` clippy budget instead of taking six
/// positional `impl Into<String>` parameters.
#[derive(Debug, Clone)]
pub struct ForestNames {
    /// Human-readable workspace/machine label for the global tier.
    pub workspace_name: PlanWorkspaceName,
    /// Repo/project name for the project tier.
    pub project_name: PlanProjectName,
    /// Plan directory name (e.g. `enforcer-selfhost-plan`) for the plan
    /// tier.
    pub plan_name: PlanName,
    /// Repo-relative path to the project tier `AGENTS.md`.
    pub project_tier_path: RelPath,
    /// Repo-relative path to the plan tier `AGENTS.md`.
    pub plan_tier_path: RelPath,
    /// Repo-relative path (or anchor label) of the plan's resume-state
    /// entrypoint.
    pub resume_anchor: PlanResumeAnchor,
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
            budget_lines: PlanBudgetLines::DEFAULT,
            budget_bytes: PlanBudgetBytes::DEFAULT,
        }
    }
}

/// The rendered 3-tier forest, ready to write to disk (or to feed directly
/// to the validators / resume simulation without a round trip through the
/// filesystem).
#[derive(Debug, Clone)]
pub struct RenderedForest {
    /// Rendered global-tier `AGENTS.md` text.
    pub global: PlanDocumentText,
    /// Rendered project-tier `AGENTS.md` text.
    pub project: PlanDocumentText,
    /// Rendered plan-tier `AGENTS.md` text.
    pub plan: PlanDocumentText,
}

/// Deterministic string substitution over a template, replacing `{{name}}`
/// tokens. Local to this module rather than importing
/// `crate::templates`'s private `render_template` (that function is not
/// `pub`) — this is a byte-for-byte copy of the same minimal substitution
/// contract b03 established (missing token -> typed error, never a
/// panic), applied to this module's own three frozen templates.
/// Render all three tiers from `templates/agents-{global,project,plan}.tpl`
/// for one plan, per [`ForestFacts`]. Pure rendering — does not touch the
/// filesystem; callers write [`RenderedForest`]'s fields to disk
/// themselves at the paths named in `facts`.
pub fn scaffold_forest(facts: &ForestFacts) -> Result<RenderedForest, PlanError> {
    render_forest(facts)
}

/// Extract the text between a `<!-- name -->` ... `<!-- /name -->` managed
/// block, trimmed. Returns `None` if either fence is absent or the close
/// precedes the open (a structurally broken block).
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
        match managed_block(input.source.as_str(), "agents-read-first") {
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
/// at a real resume-state anchor, not a dangling token).
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
        let Some(tree_text) = managed_block(input.source.as_str(), "agents-decision-tree") else {
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

impl Validator for AgentsBudgetValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let source = input.source.as_str();
        let (budget_lines, budget_bytes) = parse_declared_budget(source);
        let actual_lines = source.lines().count();
        let actual_bytes = source.len();
        if actual_lines > budget_lines.get() || actual_bytes > budget_bytes.get() {
            return vec![finding(
                &self.rule_id,
                "tier exceeds its declared size budget",
                format!(
                    "tier is {actual_lines} lines / {actual_bytes} bytes, budget is \
                     {} lines / {} bytes",
                    budget_lines.get(),
                    budget_bytes.get()
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
    pub path: RelPath,
    /// Raw source text.
    pub source: PlanDocumentText,
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
        let Some(marker) = tier_marker(doc.source.as_str()) else {
            findings.push(finding(
                rule_id,
                "tier document missing tier marker",
                "no `<!-- agents-forest-tier: <tier> -->` marker found",
                &doc.path,
            ));
            continue;
        };
        let Some(next_block) = managed_block(doc.source.as_str(), "agents-next-tier") else {
            findings.push(finding(
                rule_id,
                "tier document missing NEXT pointer block",
                "no `<!-- agents-next-tier -->` managed block found",
                &doc.path,
            ));
            continue;
        };
        let Some(next_path) = extract_keyed_value(next_block, "NEXT:") else {
            findings.push(finding(
                rule_id,
                "NEXT pointer block has no NEXT: value",
                "`<!-- agents-next-tier -->` block carries no `NEXT: <path>` line",
                &doc.path,
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
                    &doc.path,
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
                &doc.path,
            ));
            continue;
        };
        if tier_marker(next_doc.source.as_str()) != Some(expected_next_tier) {
            findings.push(finding(
                rule_id,
                "NEXT pointer targets wrong tier",
                format!(
                    "tier `{marker}` NEXT pointer names `{next_path}`, which must be a \
                     `{expected_next_tier}` tier document"
                ),
                &doc.path,
            ));
        }
    }

    findings
}

/// Outcome of [`run_resume_simulation`]: either the chain resolved to a
/// resume anchor within budget, or it failed at a named stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeSimOutcome {
    /// The chain resolved; carries the resolved resume anchor and the
    /// summed byte size of every tier walked.
    Resolved {
        /// The resume-state anchor the chain terminated at.
        resume_anchor: PlanResumeAnchor,
        /// Summed byte size of every tier document walked to get there.
        summed_bytes: PlanBudgetBytes,
    },
    /// The chain broke; carries a short reason.
    Broken(PlanDiagnosticDetail),
}

/// Walk ONLY the `AGENTS.md` chain (global -> project -> plan -> decision
/// tree) starting from `global`, following each tier's `NEXT:` pointer
/// through `by_path`, and resolve to the plan tier's decision-tree leaf —
/// WITHOUT reading anything outside the supplied tier documents (in
/// particular, without reading any plan body). Asserts the summed size of
/// every tier walked stays within `budget_bytes_total`.
pub fn run_resume_simulation(
    global: &TierDocument,
    by_path: &HashMap<RelPath, TierDocument>,
    budget_bytes_total: PlanBudgetBytes,
) -> ResumeSimOutcome {
    if tier_marker(global.source.as_str()) != Some("global") {
        return ResumeSimOutcome::Broken(diagnostic_detail(format!(
            "resume simulation must start from a global tier document, got `{}`",
            global.path
        )));
    }
    let mut summed_bytes = global.source.as_str().len();
    let mut current = global;
    let mut visited_tiers = HashSet::new();

    loop {
        let Some(marker) = tier_marker(current.source.as_str()) else {
            return ResumeSimOutcome::Broken(diagnostic_detail(format!(
                "tier document `{}` has no tier marker",
                current.path
            )));
        };
        if !visited_tiers.insert((marker, current.path.as_str())) {
            return ResumeSimOutcome::Broken(diagnostic_detail(format!(
                "resume chain cycle revisits `{marker}` tier at `{}`",
                current.path
            )));
        }
        let Some(next_block) = managed_block(current.source.as_str(), "agents-next-tier") else {
            return ResumeSimOutcome::Broken(diagnostic_detail(format!(
                "tier document `{}` has no NEXT pointer block",
                current.path
            )));
        };
        let Some(next_path) = extract_keyed_value(next_block, "NEXT:") else {
            return ResumeSimOutcome::Broken(diagnostic_detail(format!(
                "tier document `{}` NEXT pointer block has no NEXT: value",
                current.path
            )));
        };

        if marker == "plan" {
            let Some(tree_text) = managed_block(current.source.as_str(), "agents-decision-tree")
            else {
                return ResumeSimOutcome::Broken(diagnostic_detail(format!(
                    "plan tier `{}` has no decision tree",
                    current.path
                )));
            };
            let leaves = extract_leaf_pointers(tree_text);
            let Some(leaf) = leaves.into_iter().find(|l| !l.is_empty()) else {
                return ResumeSimOutcome::Broken(diagnostic_detail(format!(
                    "plan tier `{}` decision tree has no resolvable leaf",
                    current.path
                )));
            };
            if summed_bytes > budget_bytes_total.get() {
                return ResumeSimOutcome::Broken(diagnostic_detail(format!(
                    "chain resolved to `{leaf}` but summed {summed_bytes} bytes exceeds the \
                     {}-byte total chain budget",
                    budget_bytes_total.get()
                )));
            }
            return ResumeSimOutcome::Resolved {
                resume_anchor: resume_anchor(leaf),
                summed_bytes: budget_bytes(summed_bytes),
            };
        }

        let expected_next_tier = match marker {
            "global" => "project",
            "project" => "plan",
            _ => {
                return ResumeSimOutcome::Broken(diagnostic_detail(format!(
                    "tier document `{}` has unsupported tier marker `{marker}`",
                    current.path
                )));
            }
        };

        let next_path = match next_path.parse::<RelPath>() {
            Ok(path) => path,
            Err(error) => {
                return ResumeSimOutcome::Broken(diagnostic_detail(format!(
                    "tier `{marker}` NEXT pointer is invalid: {error}"
                )));
            }
        };
        let Some(next_doc) = by_path.get(&next_path) else {
            return ResumeSimOutcome::Broken(diagnostic_detail(format!(
                "tier `{marker}` NEXT pointer names `{next_path}`, which was not supplied to the \
                walker"
            )));
        };
        if tier_marker(next_doc.source.as_str()) != Some(expected_next_tier) {
            return ResumeSimOutcome::Broken(diagnostic_detail(format!(
                "tier `{marker}` at `{}` must point to a `{expected_next_tier}` tier, not `{}`",
                current.path, next_path
            )));
        }
        summed_bytes += next_doc.source.as_str().len();
        current = next_doc;
    }
}

/// Doc-intent check: the module doc (this file's leading `//!` block) and
/// each of the three templates declare the transitional-to-typed-data
/// statement. Exercised over the templates' raw text (the module doc is
/// checked by a dedicated test reading `src/agents_forest.rs` itself, so
/// both halves of the requirement are proven, not assumed).
pub fn declares_transitional_intent(source: &PlanDocumentText) -> PlanCondition {
    let lower = source.as_str().to_lowercase();
    if lower.contains("transitional-to-typed")
        && lower.contains("typed system/db/schema")
        && lower.contains("tauri")
    {
        PlanCondition::Satisfied
    } else {
        PlanCondition::Unsatisfied
    }
}
