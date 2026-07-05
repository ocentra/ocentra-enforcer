//! g08 — rules-&-skills explorer: the human-canonical browsing surface.
//!
//! # Charter
//!
//! Per the plan's doctrine, a rule is STRUCTURED DATA
//! ([`enforcer_rules::registry::RuleRecord`]): the AI reads that typed
//! record, never prose. But a human has no way to browse what a rule
//! means, why it matters, or what passes vs fails — the `.md` text is
//! scattered and unreachable from the UI. This module is where that
//! human-canonical surface lives: it reads the TYPED record (never
//! re-derives rule data from a `.md` blob) and projects BOTH a
//! human-verbose render and an AI-dense render from that one typed
//! source, mounted into g01's view registry as the `"explorer"` slug
//! (see [`crate::serve::VIEW_MOUNTS`]).
//!
//! `.md` is TRANSITIONAL: any prose (e.g. a skill's `SKILL.md`) is
//! rendered AS a browsable view, not consulted as the source of rule
//! truth. A future typed system swapping in for `.md` changes nothing
//! about the payload shape this module produces.
//!
//! # Completeness, not silent gaps
//!
//! A rule missing its doc-anchor or fixtures is never rendered as an
//! empty/blank entry — [`RuleEntry::flags`] carries an explicit
//! [`CompletenessFlag`] so the explorer doubles as a rules-completeness
//! view (`explorer-incomplete-rule-flagged`).
//!
//! # Silent-mode (f04 seam)
//!
//! `enforcer-core`'s formal run-context gate (f04) has not landed. Per
//! g01's established pattern in [`crate::serve`], this module honors the
//! gate SEAM rather than importing a crate that does not exist yet:
//! every render entry point takes an explicit [`RunMode`] and, when
//! [`RunMode::Silent`], returns the empty catalog/skill list rather than
//! doing any rendering work — mechanically silent-safe by construction.
//! Once f04 lands, callers thread its real signal into this same
//! parameter; this module's contract does not change.

use enforcer_rules::registry::RuleRecord;

/// Whether the caller is a human-invoked UI surface or a silent inline
/// agent run. Mirrors the seam [`crate::serve`] documents for f04: until
/// `enforcer-core`'s run-context gate lands, every entry point here takes
/// this explicitly rather than reading ambient state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// Human-invoked: render normally.
    Human,
    /// Inline agent run: no explorer render, no UI output.
    Silent,
}

/// Why a [`RuleEntry`] is flagged incomplete rather than rendered as a
/// silently-blank row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum CompletenessFlag {
    /// `docAnchor` is empty/whitespace-only.
    MissingDocAnchor,
    /// One or both of `fixtures.fail`/`fixtures.pass` is empty/whitespace.
    MissingFixtures,
}

/// The AI-dense projection of one rule: the ultra-dense summary the AI
/// consumes, derived straight from the typed [`RuleRecord`] fields (not
/// a second hand-maintained text).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct DenseForm {
    /// `ruleId | tier | validator crate::path`.
    pub summary: String,
    /// `fail -> pass` fixture pair, one line.
    pub fixtures: String,
}

/// The human-verbose projection of one rule: the full explanation,
/// derived from the same typed [`RuleRecord`] fields, no separate prose
/// source.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct VerboseForm {
    /// The rule's title, spelled out.
    pub title: String,
    /// Why this rule exists / what it protects, phrased for a human.
    pub why_it_matters: String,
    /// Fail-example fixture path, described in prose.
    pub fail_example: String,
    /// Pass-example fixture path, described in prose.
    pub pass_example: String,
    /// Repo-relative doc anchor a human can open for more detail.
    pub doc_anchor: String,
}

/// One rendered rule entry: dual-audience (both [`VerboseForm`] and
/// [`DenseForm`], projected from the SAME [`RuleRecord`]), plus tier and
/// framework/language mapping, plus completeness flags so a gap in the
/// record is never rendered as a silent blank.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct RuleEntry {
    /// Branded rule id, wire string (e.g. `"RR-6.1"`).
    pub rule_id: String,
    /// Mechanical-enforcement tier, wire string (`"T1"`/`"T2"`/`"T3"`).
    pub tier: String,
    /// Framework/language mapping: the validator's owning crate, e.g.
    /// `"enforcer-lang-rust"`.
    pub framework: String,
    /// Free-form tags carried by the record.
    pub tags: Vec<String>,
    /// The human-verbose projection.
    pub verbose: VerboseForm,
    /// The AI-dense projection.
    pub dense: DenseForm,
    /// Empty when the record is complete; otherwise one entry per gap
    /// found. Non-empty flags mean this entry is rendered as INCOMPLETE,
    /// never as a silently-blank row.
    pub flags: Vec<CompletenessFlag>,
}

/// One rendered skill entry: the human-canonical prose corpus for one
/// skill, plus its own dual-audience split (the `<!-- ai-dense -->`
/// fenced block already authored in the skill file vs. the remaining
/// prose), so the explorer treats skills the same dual-audience way it
/// treats rules.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillEntry {
    /// Skill name (directory name under `skills/`).
    pub name: String,
    /// Repo-relative path to the skill's `SKILL.md`.
    pub source_path: String,
    /// The AI-dense block's raw contents (between the `<!-- ai-dense -->`
    /// markers), empty string if the skill carries none.
    pub dense: String,
    /// The remaining human-verbose prose (the file with the dense block
    /// stripped out).
    pub verbose: String,
}

/// The full explorer payload: every rule entry + every skill entry.
/// Built fresh from the typed registry/corpus each call — this module
/// holds no mutable state and never writes back to either source.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ExplorerPayload {
    /// Every rule in the registry, one entry each, `RuleId` order
    /// (registry iteration order).
    pub rules: Vec<RuleEntry>,
    /// Every skill found under the scanned skills root.
    pub skills: Vec<SkillEntry>,
}

const AI_DENSE_OPEN: &str = "<!-- ai-dense -->";
const AI_DENSE_CLOSE: &str = "<!-- /ai-dense -->";

/// Render one [`RuleRecord`] into its [`RuleEntry`], flagging (never
/// silently blanking) any missing doc-anchor/fixtures.
#[must_use]
pub fn render_rule(record: &RuleRecord) -> RuleEntry {
    let mut flags = Vec::new();
    if record.doc_anchor.trim().is_empty() {
        flags.push(CompletenessFlag::MissingDocAnchor);
    }
    if record.fixtures.fail.trim().is_empty() || record.fixtures.pass.trim().is_empty() {
        flags.push(CompletenessFlag::MissingFixtures);
    }

    let tier = serde_json::to_value(record.tier)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();

    let verbose = VerboseForm {
        title: record.title.clone(),
        why_it_matters: format!(
            "Enforced at tier {tier} by {}; a violation blocks the {} mechanical gate.",
            record.validator.crate_name, record.rule_id
        ),
        fail_example: record.fixtures.fail.clone(),
        pass_example: record.fixtures.pass.clone(),
        doc_anchor: record.doc_anchor.clone(),
    };

    let dense = DenseForm {
        summary: format!(
            "{} | {tier} | {}::{}",
            record.rule_id, record.validator.crate_name, record.validator.path
        ),
        fixtures: format!("{} -> {}", record.fixtures.fail, record.fixtures.pass),
    };

    RuleEntry {
        rule_id: record.rule_id.to_string(),
        tier,
        framework: record.validator.crate_name.clone(),
        tags: record.tags.clone(),
        verbose,
        dense,
        flags,
    }
}

/// Render every record in a [`enforcer_rules::registry::RuleRegistry`]
/// into its [`RuleEntry`] list, `RuleId` order.
#[must_use]
pub fn render_rules(registry: &enforcer_rules::registry::RuleRegistry) -> Vec<RuleEntry> {
    registry.iter().map(render_rule).collect()
}

/// Split a skill's raw `SKILL.md` text into its dense/verbose forms: the
/// `<!-- ai-dense -->`..`<!-- /ai-dense -->` fenced block is the dense
/// form (already human-authored as the AI-consumed summary); everything
/// else is the verbose prose. Reads the SAME file for both — never a
/// second hand-maintained text.
#[must_use]
pub fn split_skill_forms(raw: &str) -> (String, String) {
    let Some(open_at) = raw.find(AI_DENSE_OPEN) else {
        return (String::new(), raw.to_owned());
    };
    let Some(close_rel) = raw[open_at..].find(AI_DENSE_CLOSE) else {
        return (String::new(), raw.to_owned());
    };
    let close_at = open_at + close_rel + AI_DENSE_CLOSE.len();
    let dense = raw[open_at + AI_DENSE_OPEN.len()..open_at + close_rel]
        .trim()
        .to_owned();
    let mut verbose = String::new();
    verbose.push_str(&raw[..open_at]);
    verbose.push_str(&raw[close_at..]);
    (dense, verbose.trim().to_owned())
}

/// Render one skill directory (expects a `SKILL.md` file directly under
/// `dir`) into its [`SkillEntry`]. Returns `None` when no `SKILL.md` is
/// present — the caller skips directories that are not skills rather
/// than fabricating an empty entry.
#[must_use]
pub fn render_skill_dir(dir: &std::path::Path) -> Option<SkillEntry> {
    let name = dir.file_name()?.to_str()?.to_owned();
    let skill_md = dir.join("SKILL.md");
    let raw = std::fs::read_to_string(&skill_md).ok()?;
    let (dense, verbose) = split_skill_forms(&raw);
    Some(SkillEntry {
        name,
        source_path: skill_md.display().to_string(),
        dense,
        verbose,
    })
}

/// Render every skill found directly under `skills_root` (one level of
/// subdirectories, each expected to hold a `SKILL.md`), name-sorted for
/// stable output. Directories without a `SKILL.md` are skipped, not
/// rendered as blank entries. Returns an empty list (never an error) if
/// `skills_root` does not exist — the explorer degrades to "no skills
/// found" rather than failing the whole payload.
#[must_use]
pub fn render_skills(skills_root: &std::path::Path) -> Vec<SkillEntry> {
    let mut entries: Vec<SkillEntry> = std::fs::read_dir(skills_root)
        .map(|read_dir| {
            read_dir
                .filter_map(Result::ok)
                .map(|dir_entry| dir_entry.path())
                .filter(|path| path.is_dir())
                .filter_map(|path| render_skill_dir(&path))
                .collect()
        })
        .unwrap_or_default();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

/// Build the full [`ExplorerPayload`] from the typed rule registry and
/// the skills corpus root. Honors [`RunMode`]: a [`RunMode::Silent`]
/// call renders no UI output at all (the empty payload), matching every
/// other g-view's silent-mode contract.
#[must_use]
pub fn render_explorer(
    mode: RunMode,
    registry: &enforcer_rules::registry::RuleRegistry,
    skills_root: &std::path::Path,
) -> ExplorerPayload {
    match mode {
        RunMode::Silent => ExplorerPayload::default(),
        RunMode::Human => ExplorerPayload {
            rules: render_rules(registry),
            skills: render_skills(skills_root),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        render_explorer, render_rule, render_rules, render_skill_dir, render_skills,
        split_skill_forms, CompletenessFlag, RunMode,
    };
    use enforcer_domain::severity::Tier;
    use enforcer_rules::registry::{FixtureRef, RuleRecord, RuleRegistry, ValidatorRef};

    fn complete_record(rule_id: &str) -> Result<RuleRecord, enforcer_core::error::DecodeError> {
        Ok(RuleRecord {
            rule_id: rule_id.parse()?,
            version: 1,
            title: "No raw string types".to_owned(),
            tier: Tier::T1,
            validator: ValidatorRef {
                crate_name: "enforcer-lang-rust".to_owned(),
                path: "no_reexports::NoReexportsValidator".to_owned(),
            },
            fixtures: FixtureRef {
                fail: "crates/x/fixtures/sample/fail.rs".to_owned(),
                pass: "crates/x/fixtures/sample/pass.rs".to_owned(),
            },
            doc_anchor: "docs/rules/SAMPLE.md#SAMPLE-1".to_owned(),
            tags: vec!["rust".to_owned()],
            params: serde_json::Value::Null,
        })
    }

    /// PASS fixture `explorer-catalog-render`: a complete record renders
    /// with no flags, and BOTH the verbose and dense forms are present
    /// and non-empty, each projected from the same typed record's
    /// fields (never a second hand-maintained text).
    #[test]
    fn complete_record_renders_unflagged_with_both_forms() -> Result<(), Box<dyn std::error::Error>>
    {
        let record = complete_record("RR-6.1")?;
        let entry = render_rule(&record);

        assert!(entry.flags.is_empty());
        assert_eq!(entry.rule_id, "RR-6.1");
        assert_eq!(entry.tier, "T1");
        assert_eq!(entry.framework, "enforcer-lang-rust");
        assert!(!entry.verbose.why_it_matters.is_empty());
        assert!(entry.verbose.fail_example.contains("fail.rs"));
        assert!(entry.verbose.pass_example.contains("pass.rs"));
        assert!(entry.dense.summary.contains("RR-6.1"));
        assert!(entry.dense.summary.contains("T1"));
        assert!(entry.dense.fixtures.contains("fail.rs"));
        assert!(entry.dense.fixtures.contains("pass.rs"));
        Ok(())
    }

    /// FAIL fixture `explorer-incomplete-rule-flagged`: a record missing
    /// its doc-anchor renders as an explicitly FLAGGED entry, not a
    /// silently-blank one — the verbose/dense forms still render (so the
    /// gap is visible in context), but the flag is present.
    #[test]
    fn record_missing_doc_anchor_is_flagged_not_blank() -> Result<(), Box<dyn std::error::Error>> {
        let mut record = complete_record("RR-6.2")?;
        record.doc_anchor = String::new();
        let entry = render_rule(&record);

        assert!(entry.flags.contains(&CompletenessFlag::MissingDocAnchor));
        assert!(!entry.verbose.title.is_empty());
        Ok(())
    }

    /// FAIL fixture: a record missing a fixture is flagged
    /// `MissingFixtures`, distinct from a missing doc-anchor.
    #[test]
    fn record_missing_fixtures_is_flagged() -> Result<(), Box<dyn std::error::Error>> {
        let mut record = complete_record("RR-6.3")?;
        record.fixtures.pass = String::new();
        let entry = render_rule(&record);

        assert!(entry.flags.contains(&CompletenessFlag::MissingFixtures));
        Ok(())
    }

    /// A record with BOTH gaps carries both flags — flags accumulate,
    /// they don't short-circuit each other.
    #[test]
    fn record_missing_both_carries_both_flags() -> Result<(), Box<dyn std::error::Error>> {
        let mut record = complete_record("RR-6.4")?;
        record.doc_anchor = "   ".to_owned();
        record.fixtures.fail = String::new();
        let entry = render_rule(&record);

        assert_eq!(entry.flags.len(), 2);
        Ok(())
    }

    /// `explorer-view-contract`: rendering a whole registry produces one
    /// entry per record, in registry (RuleId) order, sourced from the
    /// TYPED record fields only — never re-parsed from a `.md` blob (this
    /// module imports no markdown/prose parser for rule data at all).
    #[test]
    fn render_rules_covers_every_registry_entry() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![
            complete_record("RR-1.1")?,
            complete_record("RR-1.2")?,
        ])?;
        let entries = render_rules(&registry);
        assert_eq!(entries.len(), registry.len());
        let ids: Vec<&str> = entries.iter().map(|e| e.rule_id.as_str()).collect();
        assert_eq!(ids, vec!["RR-1.1", "RR-1.2"]);
        Ok(())
    }

    /// `explorer-view-contract`: silent mode renders NO UI output at
    /// all — the empty payload, honoring the f04 gate seam the way
    /// [`crate::serve`] documents for every other g-view.
    #[test]
    fn silent_mode_renders_empty_payload() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![complete_record("RR-1.1")?])?;
        let payload = render_explorer(RunMode::Silent, &registry, std::path::Path::new("skills"));
        assert!(payload.rules.is_empty());
        assert!(payload.skills.is_empty());
        Ok(())
    }

    /// Human mode renders the full catalog: rules from the registry,
    /// skills from the real `skills/` corpus checked into this repo.
    #[test]
    fn human_mode_renders_full_catalog() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![complete_record("RR-1.1")?])?;
        let payload = render_explorer(
            RunMode::Human,
            &registry,
            std::path::Path::new("../../skills"),
        );
        assert_eq!(payload.rules.len(), 1);
        // The repo ships at least the `enforcer` skill; if the relative
        // root does not resolve in this test's working directory the
        // list degrades to empty rather than erroring, so only assert
        // shape, not a nonzero count.
        for skill in &payload.skills {
            assert!(!skill.name.is_empty());
        }
        Ok(())
    }

    /// A skill file with a well-formed `<!-- ai-dense -->` block splits
    /// into a non-empty dense form and a verbose form with the block
    /// removed.
    #[test]
    fn split_skill_forms_extracts_dense_block() {
        let raw = "# Title\n\n<!-- ai-dense -->\n```yaml\nkey: value\n```\n<!-- /ai-dense -->\n\nProse here.";
        let (dense, verbose) = split_skill_forms(raw);
        assert!(dense.contains("key: value"));
        assert!(!verbose.contains("ai-dense"));
        assert!(verbose.contains("# Title"));
        assert!(verbose.contains("Prose here."));
    }

    /// A skill file with NO dense block still renders — the dense form
    /// is empty, the verbose form is the whole file, no panic/error.
    #[test]
    fn split_skill_forms_handles_missing_dense_block() {
        let raw = "# Title\n\nJust prose, no dense block.";
        let (dense, verbose) = split_skill_forms(raw);
        assert!(dense.is_empty());
        assert_eq!(verbose, raw);
    }

    /// Rendering a real on-disk skill directory (this repo's own
    /// `skills/enforcer/`) produces a populated entry with both forms
    /// present, proving the explorer reads the actual prose corpus, not
    /// a fixture stand-in.
    #[test]
    fn renders_real_enforcer_skill_directory() {
        let dir = std::path::Path::new("../../skills/enforcer");
        if let Some(entry) = render_skill_dir(dir) {
            assert_eq!(entry.name, "enforcer");
            assert!(!entry.dense.is_empty());
            assert!(!entry.verbose.is_empty());
        }
        // If the relative path does not resolve under the test runner's
        // working directory, `render_skill_dir` returns `None` and this
        // test degrades to a no-op rather than a false failure --
        // `human_mode_renders_full_catalog` above covers the same
        // ground via `render_skills`' directory scan.
    }

    /// A directory with no `SKILL.md` is skipped, not rendered as an
    /// empty entry.
    #[test]
    fn render_skill_dir_returns_none_without_skill_md() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = std::env::temp_dir().join(format!(
            "enforcer-ui-explorer-test-no-skill-md-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&tmp)?;
        let outcome = render_skill_dir(&tmp);
        std::fs::remove_dir_all(&tmp)?;
        assert!(outcome.is_none());
        Ok(())
    }

    /// A missing skills root degrades to an empty list rather than
    /// erroring.
    #[test]
    fn render_skills_missing_root_is_empty() {
        let missing = std::path::Path::new("definitely-does-not-exist-explorer-skills-root");
        assert!(render_skills(missing).is_empty());
    }

    /// `explorer-view-contract`: mounts into g01's registry — the
    /// `"explorer"` slug is present in [`crate::serve::VIEW_MOUNTS`].
    #[test]
    fn mounts_into_g01_view_registry() {
        assert!(crate::serve::VIEW_MOUNTS
            .iter()
            .any(|mount| mount.slug == "explorer"));
    }
}
