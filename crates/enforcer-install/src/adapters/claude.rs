//! c03 — the Claude Code [`crate::core::HarnessAdapter`].
//!
//! # Charter (workpack c03, RUST_ARCHITECTURE.md "Global-install scope
//! contract" — BINDING)
//!
//! Claude Code reads its MCP server registry from the top-level
//! `mcpServers` map of **`~/.claude.json`** — the SAME user/global file
//! `codebase-memory-mcp` registers into. This is USER/GLOBAL scope: NEVER
//! a per-repo `.mcp.json`, and NEVER `~/.claude/.mcp.json`. This adapter:
//! - Upserts `mcpServers[<SERVER_NAME>] = { command: <absolute enforcer
//!   binary path>, args: [], env: { OCENTRA_LEDGER_HOME } }` into
//!   `~/.claude.json`, preserving every unrelated top-level key AND every
//!   unrelated `mcpServers` entry (e.g. `codebase-memory-mcp`) via a
//!   `serde_json::Value` merge — never a destructive overwrite of the
//!   whole file.
//! - Drops the enforcer skill under `~/.claude/skills/<SERVER_NAME>`.
//! - Emits `.claude/agents/<SERVER_NAME>.md` — the Claude-native analog of
//!   Codex's `agents/openai.yaml` subagent descriptor (YAML frontmatter +
//!   body prompt), so Claude gets the same implicit-invocation/agent setup
//!   Codex has.
//! - Upserts a `CLAUDE.md` managed block (via [`crate::managed_block`])
//!   naming the enforcer subagent + MCP tools.
//! - `verify` re-reads `~/.claude.json` AND the descriptor, fail-closed on
//!   malformed JSON or a missing/corrupt descriptor.
//!
//! `command` MUST be the absolute path this adapter is constructed with
//! ([`ClaudeAdapter::new`]) — a relative path cannot resolve from an
//! arbitrary repo cwd (RUST_ARCHITECTURE.md). The [`crate::core::HarnessAdapter`]
//! trait's `apply(&self, report)` step is not hand this adapter a fresh
//! [`RequestContext`], so the binary path is adapter STATE (set once at
//! construction) rather than re-derived per call — `plan`/`verify` both
//! cross-check their `ctx.binary_path` against this same stored value so
//! plan/apply/verify can never silently drift from one another.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::backup::backup_before_write;
use crate::cli_contract::RequestContext;
use crate::core::HarnessAdapter;
use crate::error::{InstallError, InstallResult};
use crate::hooks::pretooluse::{
    build_hook_config as build_pretooluse_hook_config,
    render_settings_entry as render_pretooluse_settings_entry,
};
use crate::hooks::sessionstart::{
    render_settings_entry as render_sessionstart_settings_entry, sessionstart_hook_config,
};
use crate::managed_block::upsert_block;
use crate::report::{AppliedChange, ApplyResult, ArtifactKind, InstallReport, PlannedChange};
use crate::report::{VerifyCheck, VerifyReport};
use enforcer_domain::paths::RepoRoot;
use enforcer_mcp::name::SERVER_NAME;

/// Env var every adapter (Claude included) sets on the registered MCP
/// server entry, consistent with the Codex adapter (workpack c03/c06
/// shared contract).
pub const LEDGER_HOME_ENV: &str = "OCENTRA_LEDGER_HOME";

/// This adapter's registration key, matching
/// [`crate::report::HarnessKey`]. Distinct from [`SERVER_NAME`] (the MCP
/// server identity key inside `mcpServers`) — this is the harness's OWN
/// identity in cross-harness reports.
const HARNESS_KEY: &str = "claude";

/// The managed-block marker name this adapter upserts into `CLAUDE.md`.
const CLAUDE_MD_BLOCK_NAME: &str = "claude-doctrine";
const SESSION_START_EVENT: &str = "SessionStart";
const PRE_TOOL_USE_EVENT: &str = "PreToolUse";

/// The Claude Code [`HarnessAdapter`]. Rooted at a `home` directory (the
/// parent of both `~/.claude.json` and `~/.claude/`) and a `binary_path`
/// (the absolute path to the installed `enforcer` binary) fixed at
/// construction, so `plan`/`apply`/`verify` all compute against the exact
/// same target — never re-derived per call in a way that could drift.
/// Tests point `home` at an isolated temp-dir fixture instead of the real
/// `~`.
#[derive(Debug, Clone)]
pub struct ClaudeAdapter {
    /// The Claude "home" root. In production this is the user's real home
    /// directory; in a test this is a temp-dir fixture root — NEVER the
    /// real home in a test.
    home: PathBuf,
    /// Absolute path to the installed `enforcer` binary this adapter
    /// registers as `mcpServers[<SERVER_NAME>].command`.
    binary_path: PathBuf,
}

impl ClaudeAdapter {
    /// Build an adapter rooted at `home` (the parent directory of both
    /// `~/.claude.json` and the `~/.claude/` directory), registering
    /// `binary_path` as the MCP server command.
    #[must_use]
    pub fn new(home: impl Into<PathBuf>, binary_path: impl Into<PathBuf>) -> Self {
        Self {
            home: home.into(),
            binary_path: binary_path.into(),
        }
    }

    /// `~/.claude.json` — the top-level `mcpServers` registry.
    #[must_use]
    pub fn claude_json_path(&self) -> PathBuf {
        self.home.join(".claude.json")
    }

    /// `~/.claude/skills/<SERVER_NAME>` — the skill directory this
    /// adapter drops the enforcer skill under.
    #[must_use]
    pub fn skill_dir(&self) -> PathBuf {
        self.home.join(".claude").join("skills").join(SERVER_NAME)
    }

    /// `~/.claude/skills/<SERVER_NAME>/SKILL.md`.
    #[must_use]
    pub fn skill_md_path(&self) -> PathBuf {
        self.skill_dir().join("SKILL.md")
    }

    /// `.claude/agents/<SERVER_NAME>.md` — the Claude-native subagent
    /// descriptor, the analog of Codex's `agents/openai.yaml`. Lives
    /// under `.claude/agents` off the SAME home root as everything else
    /// this adapter owns (a test fixture roots the whole `.claude` tree
    /// at one temp dir).
    #[must_use]
    pub fn agent_descriptor_path(&self) -> PathBuf {
        self.home
            .join(".claude")
            .join("agents")
            .join(format!("{SERVER_NAME}.md"))
    }

    /// `CLAUDE.md` — the managed-block doctrine reference.
    #[must_use]
    pub fn claude_md_path(&self) -> PathBuf {
        self.home.join("CLAUDE.md")
    }

    fn io_err(path: &Path, e: impl std::fmt::Display) -> InstallError {
        InstallError::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        }
    }

    fn repo_root(path: &Path) -> InstallResult<RepoRoot> {
        path.display().to_string().try_into().map_err(
            |e: enforcer_domain::boundary::decode_error::DecodeError| {
                InstallError::MalformedConfig {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                }
            },
        )
    }

    /// Read `~/.claude.json` as a JSON [`Value`], defaulting to an empty
    /// object when the file does not exist yet (fresh install). A file
    /// that exists but fails to parse is a detected
    /// [`InstallError::MalformedConfig`] — never silently treated as
    /// empty.
    fn read_claude_json(&self) -> InstallResult<Value> {
        let path = self.claude_json_path();
        if !path.is_file() {
            return Ok(Value::Object(Map::new()));
        }
        let raw = std::fs::read_to_string(&path).map_err(|e| Self::io_err(&path, e))?;
        serde_json::from_str(&raw).map_err(|e| InstallError::MalformedConfig {
            path: path.display().to_string(),
            reason: format!("not valid JSON: {e}"),
        })
    }

    fn write_claude_json(&self, root: &Value) -> InstallResult<()> {
        let path = self.claude_json_path();
        let rendered = serde_json::to_string_pretty(root).map_err(|e| InstallError::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        std::fs::write(&path, rendered).map_err(|e| Self::io_err(&path, e))
    }

    /// Compute the `mcpServers[<SERVER_NAME>]` entry this adapter wants,
    /// pointing `command` at the absolute `binary_path`.
    fn desired_entry(binary_path: &Path) -> Value {
        serde_json::json!({
            "command": binary_path.display().to_string(),
            "args": [],
            "env": {
                LEDGER_HOME_ENV: "${HOME}/.enforcer/ledger",
            },
        })
    }

    fn desired_sessionstart_hook(binary_path: &Path) -> Value {
        render_sessionstart_settings_entry(&sessionstart_hook_config(binary_path))
    }

    fn desired_pretooluse_hook(binary_path: &Path) -> Value {
        render_pretooluse_settings_entry(&build_pretooluse_hook_config(binary_path))
    }

    /// True when `existing_root`'s `mcpServers[<SERVER_NAME>]` entry
    /// already equals `desired` — the idempotent-reinstall no-op signal.
    fn entry_matches(existing_root: &Value, desired: &Value) -> bool {
        existing_root
            .get("mcpServers")
            .and_then(|servers| servers.get(SERVER_NAME))
            == Some(desired)
    }

    fn hook_entry_command(entry: &Value) -> Option<&str> {
        entry
            .get("hooks")
            .and_then(Value::as_array)
            .and_then(|hooks| hooks.first())
            .and_then(|hook| hook.get("command"))
            .and_then(Value::as_str)
    }

    fn hook_entry_matches(existing_root: &Value, event_name: &str, desired: &Value) -> bool {
        existing_root
            .get("hooks")
            .and_then(Value::as_object)
            .and_then(|hooks| hooks.get(event_name))
            .and_then(Value::as_array)
            .is_some_and(|entries| entries.iter().any(|entry| entry == desired))
    }

    fn hook_command_present(existing_root: &Value, event_name: &str, command: &str) -> bool {
        existing_root
            .get("hooks")
            .and_then(Value::as_object)
            .and_then(|hooks| hooks.get(event_name))
            .and_then(Value::as_array)
            .is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| Self::hook_entry_command(entry) == Some(command))
            })
    }

    /// Value-merge `desired` into `existing_root`'s top-level
    /// `mcpServers` map, preserving every unrelated top-level key AND
    /// every unrelated `mcpServers` entry (e.g. `codebase-memory-mcp`).
    fn merge_mcp_server(existing_root: &mut Value, desired: Value) {
        if !existing_root.is_object() {
            *existing_root = Value::Object(Map::new());
        }
        let Some(root) = existing_root.as_object_mut() else {
            return;
        };
        let servers = root
            .entry("mcpServers")
            .or_insert_with(|| Value::Object(Map::new()));
        if !servers.is_object() {
            *servers = Value::Object(Map::new());
        }
        if let Some(servers_map) = servers.as_object_mut() {
            servers_map.insert(SERVER_NAME.to_owned(), desired);
        }
    }

    /// Remove `mcpServers[<SERVER_NAME>]` from `existing_root`, preserving
    /// every unrelated top-level key AND every unrelated `mcpServers`
    /// entry — the uninstall-direction mirror of [`Self::merge_mcp_server`].
    fn remove_mcp_server(existing_root: &mut Value) {
        if let Some(servers) = existing_root
            .get_mut("mcpServers")
            .and_then(Value::as_object_mut)
        {
            servers.remove(SERVER_NAME);
            if servers.is_empty() {
                if let Some(root) = existing_root.as_object_mut() {
                    root.remove("mcpServers");
                }
            }
        }
    }

    fn merge_hook_entry(existing_root: &mut Value, event_name: &str, desired: Value) {
        if !existing_root.is_object() {
            *existing_root = Value::Object(Map::new());
        }
        let Some(root) = existing_root.as_object_mut() else {
            return;
        };
        let hooks = root
            .entry("hooks")
            .or_insert_with(|| Value::Object(Map::new()));
        if !hooks.is_object() {
            *hooks = Value::Object(Map::new());
        }
        let Some(hooks_map) = hooks.as_object_mut() else {
            return;
        };
        let entries = hooks_map
            .entry(event_name.to_owned())
            .or_insert_with(|| Value::Array(Vec::new()));
        if !entries.is_array() {
            *entries = Value::Array(Vec::new());
        }
        let Some(entries) = entries.as_array_mut() else {
            return;
        };
        let desired_command = Self::hook_entry_command(&desired).map(str::to_owned);
        let mut exact_present = false;
        let mut retained = Vec::with_capacity(entries.len() + 1);
        for entry in entries.drain(..) {
            if entry == desired {
                if !exact_present {
                    exact_present = true;
                    retained.push(entry);
                }
                continue;
            }
            let same_command = desired_command
                .as_deref()
                .is_some_and(|command| Self::hook_entry_command(&entry) == Some(command));
            if same_command {
                continue;
            }
            retained.push(entry);
        }
        if !exact_present {
            retained.push(desired);
        }
        *entries = retained;
    }

    fn remove_hook_entry(existing_root: &mut Value, event_name: &str, command: &str) {
        let Some(root) = existing_root.as_object_mut() else {
            return;
        };
        let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
            return;
        };
        let mut remove_event = false;
        if let Some(entries) = hooks.get_mut(event_name).and_then(Value::as_array_mut) {
            entries.retain(|entry| Self::hook_entry_command(entry) != Some(command));
            remove_event = entries.is_empty();
        }
        if remove_event {
            hooks.remove(event_name);
        }
        if hooks.is_empty() {
            root.remove("hooks");
        }
    }

    fn verify_hook_check(
        existing_root: &Value,
        claude_json_path: &Path,
        event_name: &str,
        check_name: &str,
        desired: &Value,
    ) -> VerifyCheck {
        let expected_command = Self::hook_entry_command(desired).unwrap_or("<unknown>");
        let detail = if existing_root.get("hooks").is_none() {
            Some(format!(
                "`hooks` missing from `{}`",
                claude_json_path.display()
            ))
        } else if !existing_root.get("hooks").is_some_and(Value::is_object) {
            Some(format!(
                "`hooks` in `{}` is not an object",
                claude_json_path.display()
            ))
        } else {
            let entries = existing_root
                .get("hooks")
                .and_then(Value::as_object)
                .and_then(|hooks| hooks.get(event_name))
                .and_then(Value::as_array);
            match entries {
                None => Some(format!(
                    "hooks.{event_name} missing from `{}`",
                    claude_json_path.display()
                )),
                Some(entries) if entries.iter().any(|entry| entry == desired) => None,
                Some(entries)
                    if entries
                        .iter()
                        .any(|entry| Self::hook_entry_command(entry) == Some(expected_command)) =>
                {
                    Some(format!(
                        "hooks.{event_name} contains an enforcer command `{expected_command}` but \
                         the entry drifted from the expected config"
                    ))
                }
                Some(_) => Some(format!(
                    "hooks.{event_name} missing expected enforcer command `{expected_command}`"
                )),
            }
        };
        VerifyCheck {
            harness: HARNESS_KEY.to_owned(),
            name: check_name.to_owned(),
            passed: detail.is_none(),
            detail: detail.unwrap_or_default(),
        }
    }

    /// Render the `.claude/agents/<SERVER_NAME>.md` subagent descriptor:
    /// YAML frontmatter (`name`/`description`/tool-scope) + a body prompt
    /// mirroring Codex's `openai.yaml` implicit-invocation/agent setup.
    #[must_use]
    pub fn render_agent_descriptor() -> String {
        format!(
            "---\n\
             name: {SERVER_NAME}\n\
             description: Mechanical enforcement gate for this repository. Invoke \
             automatically before any commit/PR-readiness claim, and whenever a rule/\
             policy check, scan, or proof status is needed.\n\
             allow_implicit_invocation: true\n\
             tools:\n\
             - mcp__{SERVER_NAME}__*\n\
             ---\n\
             \n\
             # {SERVER_NAME} subagent\n\
             \n\
             You are the `{SERVER_NAME}` enforcement subagent. Use the \
             `mcp__{SERVER_NAME}__*` tools to run mechanical checks, scans, and proof \
             status against this repository. Never let a self-review substitute for a \
             mechanical gate: treat every claim of \"done\"/\"passing\"/\"ready\" as \
             unproven until the `{SERVER_NAME}` tools confirm it.\n\
             "
        )
    }

    /// Parse-and-validate `.claude/agents/<SERVER_NAME>.md`'s frontmatter
    /// far enough to confirm it is a genuine descriptor: begins with a
    /// `---` frontmatter block, and that block declares `name:` and
    /// `allow_implicit_invocation: true`. Returns the parse failure
    /// reason on anything else — never a silent pass on a corrupt/
    /// truncated file.
    fn validate_agent_descriptor(raw: &str) -> Result<(), String> {
        let mut lines = raw.lines();
        match lines.next() {
            Some("---") => {}
            _ => return Err("missing opening `---` frontmatter fence".to_owned()),
        }
        let mut frontmatter = String::new();
        let mut closed = false;
        for line in lines.by_ref() {
            if line == "---" {
                closed = true;
                break;
            }
            frontmatter.push_str(line);
            frontmatter.push('\n');
        }
        if !closed {
            return Err("missing closing `---` frontmatter fence".to_owned());
        }
        if !frontmatter
            .lines()
            .any(|l| l.trim_start().starts_with("name:"))
        {
            return Err("frontmatter missing `name:` field".to_owned());
        }
        if !frontmatter
            .lines()
            .any(|l| l.trim() == "allow_implicit_invocation: true")
        {
            return Err("frontmatter missing `allow_implicit_invocation: true`".to_owned());
        }
        Ok(())
    }

    /// The `CLAUDE.md` managed-block content, naming the enforcer
    /// subagent + MCP tools.
    fn claude_md_block_content() -> String {
        format!(
            "This repository has the `{SERVER_NAME}` mechanical-enforcement subagent \
             installed (see `.claude/agents/{SERVER_NAME}.md`) and its MCP tools are \
             available as `mcp__{SERVER_NAME}__*`. Invoke the `{SERVER_NAME}` subagent \
             (or its MCP tools directly) before any commit/PR-readiness claim — a \
             self-review is never a substitute for the mechanical gate.\n\n\
             {}",
            crate::core::PARALLEL_EXECUTION_DOCTRINE
        )
    }

    /// The enforcer skill body dropped at
    /// `~/.claude/skills/<SERVER_NAME>/SKILL.md`.
    fn skill_md_content() -> String {
        format!(
            "---\n\
             name: {SERVER_NAME}\n\
             description: Run the {SERVER_NAME} mechanical-enforcement checks (rule \
             checks, scans, proof status) against this repository via its MCP tools \
             (mcp__{SERVER_NAME}__*).\n\
             ---\n\
             \n\
             # {SERVER_NAME}\n\
             \n\
             Use the `mcp__{SERVER_NAME}__*` MCP tools for rule checks, scans, and \
             proof status. See `.claude/agents/{SERVER_NAME}.md` for the full subagent \
             descriptor.\n\
             "
        )
    }

    /// Whether `ctx.binary_path` matches this adapter's own configured
    /// binary path — a mismatch means the caller built `RequestContext`
    /// inconsistently with how this adapter was constructed, which would
    /// otherwise silently plan/verify against the wrong target.
    fn check_ctx_consistency(&self, ctx: &RequestContext) -> InstallResult<()> {
        if ctx.binary_path != self.binary_path {
            return Err(InstallError::MalformedConfig {
                path: self.claude_json_path().display().to_string(),
                reason: format!(
                    "RequestContext.binary_path `{}` does not match the path `{}` this \
                     ClaudeAdapter was constructed with",
                    ctx.binary_path.display(),
                    self.binary_path.display()
                ),
            });
        }
        Ok(())
    }
}

impl HarnessAdapter for ClaudeAdapter {
    fn harness_key(&self) -> &'static str {
        HARNESS_KEY
    }

    fn plan(&self, ctx: &RequestContext) -> InstallResult<InstallReport> {
        self.check_ctx_consistency(ctx)?;
        let existing = self.read_claude_json()?;
        let desired = Self::desired_entry(&self.binary_path);
        let desired_sessionstart = Self::desired_sessionstart_hook(&self.binary_path);
        let desired_pretooluse = Self::desired_pretooluse_hook(&self.binary_path);
        let mut planned_changes = Vec::new();
        let mut warnings = Vec::new();

        let claude_json_is_update = self.claude_json_path().is_file();
        if !Self::entry_matches(&existing, &desired) {
            planned_changes.push(PlannedChange {
                harness: HARNESS_KEY.to_owned(),
                kind: ArtifactKind::McpRegistration,
                path: Self::repo_root(&self.claude_json_path())?,
                description: format!(
                    "upsert mcpServers[\"{SERVER_NAME}\"] in ~/.claude.json (user/global scope)"
                ),
                is_update: claude_json_is_update,
            });
        }
        if !Self::hook_entry_matches(&existing, SESSION_START_EVENT, &desired_sessionstart)
            || !Self::hook_entry_matches(&existing, PRE_TOOL_USE_EVENT, &desired_pretooluse)
        {
            planned_changes.push(PlannedChange {
                harness: HARNESS_KEY.to_owned(),
                kind: ArtifactKind::HarnessSpecific,
                path: Self::repo_root(&self.claude_json_path())?,
                description:
                    "upsert Claude SessionStart and PreToolUse hook entries in ~/.claude.json"
                        .to_owned(),
                is_update: claude_json_is_update,
            });
        }

        let skill_path = self.skill_md_path();
        let desired_skill = Self::skill_md_content();
        let skill_is_update = skill_path.is_file();
        let skill_matches = skill_is_update
            && std::fs::read_to_string(&skill_path)
                .map(|s| s == desired_skill)
                .unwrap_or(false);
        if !skill_matches {
            planned_changes.push(PlannedChange {
                harness: HARNESS_KEY.to_owned(),
                kind: ArtifactKind::HarnessSpecific,
                path: Self::repo_root(&skill_path)?,
                description: format!("install {SERVER_NAME} skill under ~/.claude/skills"),
                is_update: skill_is_update,
            });
        }

        let descriptor_path = self.agent_descriptor_path();
        let desired_descriptor = Self::render_agent_descriptor();
        let descriptor_is_update = descriptor_path.is_file();
        let descriptor_matches = descriptor_is_update
            && std::fs::read_to_string(&descriptor_path)
                .map(|s| s == desired_descriptor)
                .unwrap_or(false);
        if !descriptor_matches {
            planned_changes.push(PlannedChange {
                harness: HARNESS_KEY.to_owned(),
                kind: ArtifactKind::HarnessSpecific,
                path: Self::repo_root(&descriptor_path)?,
                description: format!("emit .claude/agents/{SERVER_NAME}.md subagent descriptor"),
                is_update: descriptor_is_update,
            });
        }

        let claude_md_path = self.claude_md_path();
        let claude_md_is_update = claude_md_path.is_file();
        let existing_claude_md = if claude_md_is_update {
            std::fs::read_to_string(&claude_md_path)
                .map_err(|e| Self::io_err(&claude_md_path, e))?
        } else {
            String::new()
        };
        let rendered_claude_md = upsert_block(
            &existing_claude_md,
            CLAUDE_MD_BLOCK_NAME,
            &Self::claude_md_block_content(),
            &claude_md_path.display().to_string(),
        )?;
        if rendered_claude_md != existing_claude_md {
            planned_changes.push(PlannedChange {
                harness: HARNESS_KEY.to_owned(),
                kind: ArtifactKind::DoctrineReference,
                path: Self::repo_root(&claude_md_path)?,
                description: "upsert CLAUDE.md managed doctrine block".to_owned(),
                is_update: claude_md_is_update,
            });
        }

        if !self.home.is_dir() {
            warnings.push(format!(
                "Claude home `{}` does not exist yet; apply will create it",
                self.home.display()
            ));
        }

        Ok(InstallReport {
            planned_changes,
            warnings,
        })
    }

    fn apply(&self, report: &InstallReport) -> InstallResult<ApplyResult> {
        let mut applied = Vec::with_capacity(report.planned_changes.len());

        for change in &report.planned_changes {
            let target = PathBuf::from(change.path.as_str());
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| Self::io_err(parent, e))?;
            }

            let backup_path = backup_before_write(&target)?;

            match change.kind {
                ArtifactKind::McpRegistration => {
                    let mut existing = self.read_claude_json()?;
                    Self::merge_mcp_server(&mut existing, Self::desired_entry(&self.binary_path));
                    self.write_claude_json(&existing)?;
                }
                ArtifactKind::HarnessSpecific if target == self.claude_json_path() => {
                    let mut existing = self.read_claude_json()?;
                    Self::merge_hook_entry(
                        &mut existing,
                        SESSION_START_EVENT,
                        Self::desired_sessionstart_hook(&self.binary_path),
                    );
                    Self::merge_hook_entry(
                        &mut existing,
                        PRE_TOOL_USE_EVENT,
                        Self::desired_pretooluse_hook(&self.binary_path),
                    );
                    self.write_claude_json(&existing)?;
                }
                ArtifactKind::HarnessSpecific if target == self.skill_md_path() => {
                    std::fs::write(&target, Self::skill_md_content())
                        .map_err(|e| Self::io_err(&target, e))?;
                }
                ArtifactKind::HarnessSpecific if target == self.agent_descriptor_path() => {
                    std::fs::write(&target, Self::render_agent_descriptor())
                        .map_err(|e| Self::io_err(&target, e))?;
                }
                ArtifactKind::DoctrineReference => {
                    let existing = if target.is_file() {
                        std::fs::read_to_string(&target).map_err(|e| Self::io_err(&target, e))?
                    } else {
                        String::new()
                    };
                    let rendered = upsert_block(
                        &existing,
                        CLAUDE_MD_BLOCK_NAME,
                        &Self::claude_md_block_content(),
                        &target.display().to_string(),
                    )?;
                    std::fs::write(&target, rendered).map_err(|e| Self::io_err(&target, e))?;
                }
                _ => {
                    return Err(InstallError::MalformedConfig {
                        path: target.display().to_string(),
                        reason: format!(
                            "ClaudeAdapter::apply received an unrecognized planned change kind/path pair: {:?} at `{}`",
                            change.kind,
                            target.display()
                        ),
                    });
                }
            }

            applied.push(AppliedChange {
                change: change.clone(),
                succeeded: true,
                backup_path: backup_path.map(|p| Self::repo_root(&p)).transpose()?,
            });
        }

        Ok(ApplyResult { applied })
    }

    fn verify(&self, ctx: &RequestContext) -> InstallResult<VerifyReport> {
        self.check_ctx_consistency(ctx)?;
        let mut checks = Vec::new();

        let claude_json_path = self.claude_json_path();
        // A malformed `~/.claude.json` is FAIL-CLOSED here: `verify`
        // returns the typed error directly rather than downgrading it to
        // a `passed: false` check, per the workpack's binding contract
        // ("Fail-closed on malformed JSON or a missing/corrupt
        // descriptor" — distinct from a check that runs and finds a real
        // mismatch, which DOES render as `passed: false`).
        let root = self.read_claude_json()?;
        let entry = root.get("mcpServers").and_then(|s| s.get(SERVER_NAME));
        let mcp_check = match entry {
            None => VerifyCheck {
                harness: HARNESS_KEY.to_owned(),
                name: "mcp-registration-present".to_owned(),
                passed: false,
                detail: format!(
                    "mcpServers.{SERVER_NAME} missing from `{}`",
                    claude_json_path.display()
                ),
            },
            Some(entry) => {
                let command = entry.get("command").and_then(Value::as_str);
                let expected = ctx.binary_path.display().to_string();
                match command {
                    Some(c) if c == expected => VerifyCheck {
                        harness: HARNESS_KEY.to_owned(),
                        name: "mcp-registration-present".to_owned(),
                        passed: true,
                        detail: String::new(),
                    },
                    Some(c) => VerifyCheck {
                        harness: HARNESS_KEY.to_owned(),
                        name: "mcp-registration-present".to_owned(),
                        passed: false,
                        detail: format!(
                            "mcpServers.{SERVER_NAME}.command = `{c}`, expected `{expected}`"
                        ),
                    },
                    None => VerifyCheck {
                        harness: HARNESS_KEY.to_owned(),
                        name: "mcp-registration-present".to_owned(),
                        passed: false,
                        detail: format!("mcpServers.{SERVER_NAME} has no `command` field"),
                    },
                }
            }
        };
        checks.push(mcp_check);
        checks.push(Self::verify_hook_check(
            &root,
            &claude_json_path,
            SESSION_START_EVENT,
            "sessionstart-hook-present",
            &Self::desired_sessionstart_hook(&ctx.binary_path),
        ));
        checks.push(Self::verify_hook_check(
            &root,
            &claude_json_path,
            PRE_TOOL_USE_EVENT,
            "pretooluse-hook-present",
            &Self::desired_pretooluse_hook(&ctx.binary_path),
        ));

        let descriptor_path = self.agent_descriptor_path();
        let descriptor_check = if !descriptor_path.is_file() {
            VerifyCheck {
                harness: HARNESS_KEY.to_owned(),
                name: "agent-descriptor-present".to_owned(),
                passed: false,
                detail: format!(
                    "missing subagent descriptor at `{}`",
                    descriptor_path.display()
                ),
            }
        } else {
            match std::fs::read_to_string(&descriptor_path) {
                Err(e) => return Err(Self::io_err(&descriptor_path, e)),
                Ok(raw) => match Self::validate_agent_descriptor(&raw) {
                    Ok(()) => VerifyCheck {
                        harness: HARNESS_KEY.to_owned(),
                        name: "agent-descriptor-present".to_owned(),
                        passed: true,
                        detail: String::new(),
                    },
                    Err(reason) => VerifyCheck {
                        harness: HARNESS_KEY.to_owned(),
                        name: "agent-descriptor-present".to_owned(),
                        passed: false,
                        detail: format!(
                            "descriptor at `{}` is corrupt: {reason}",
                            descriptor_path.display()
                        ),
                    },
                },
            }
        };
        checks.push(descriptor_check);

        Ok(VerifyReport { checks })
    }
}

/// Plan the UNINSTALL direction for this adapter: remove the
/// `mcpServers[<SERVER_NAME>]` entry, the skill dir, and the agent
/// descriptor. Kept as a free function (rather than a second
/// [`HarnessAdapter`] impl) since the trait's `plan`/`apply` are
/// direction-agnostic per `crate::core` doc comments — the CALLER (c01's
/// `uninstall` orchestration) is expected to select install-vs-uninstall
/// planning; this module exposes the uninstall-specific planner
/// `ClaudeAdapter` callers use for that path.
impl ClaudeAdapter {
    /// Compute the uninstall plan: every artifact this adapter would
    /// remove, if present.
    ///
    /// # Errors
    /// Returns [`InstallError::MalformedConfig`] if `~/.claude.json`
    /// exists but is not valid JSON.
    pub fn plan_uninstall(&self, ctx: &RequestContext) -> InstallResult<InstallReport> {
        self.check_ctx_consistency(ctx)?;
        let existing = self.read_claude_json()?;
        let mut planned_changes = Vec::new();
        let sessionstart_command =
            Self::hook_entry_command(&Self::desired_sessionstart_hook(&self.binary_path))
                .unwrap_or_default()
                .to_owned();
        let pretooluse_command =
            Self::hook_entry_command(&Self::desired_pretooluse_hook(&self.binary_path))
                .unwrap_or_default()
                .to_owned();

        let has_entry = existing
            .get("mcpServers")
            .and_then(|s| s.get(SERVER_NAME))
            .is_some();
        if has_entry {
            planned_changes.push(PlannedChange {
                harness: HARNESS_KEY.to_owned(),
                kind: ArtifactKind::McpRegistration,
                path: Self::repo_root(&self.claude_json_path())?,
                description: format!("remove mcpServers[\"{SERVER_NAME}\"] from ~/.claude.json"),
                is_update: true,
            });
        }
        if Self::hook_command_present(&existing, SESSION_START_EVENT, &sessionstart_command)
            || Self::hook_command_present(&existing, PRE_TOOL_USE_EVENT, &pretooluse_command)
        {
            planned_changes.push(PlannedChange {
                harness: HARNESS_KEY.to_owned(),
                kind: ArtifactKind::HarnessSpecific,
                path: Self::repo_root(&self.claude_json_path())?,
                description:
                    "remove enforcer SessionStart and PreToolUse hook entries from ~/.claude.json"
                        .to_owned(),
                is_update: true,
            });
        }

        if self.skill_md_path().is_file() {
            planned_changes.push(PlannedChange {
                harness: HARNESS_KEY.to_owned(),
                kind: ArtifactKind::HarnessSpecific,
                path: Self::repo_root(&self.skill_md_path())?,
                description: format!("remove {SERVER_NAME} skill under ~/.claude/skills"),
                is_update: true,
            });
        }

        if self.agent_descriptor_path().is_file() {
            planned_changes.push(PlannedChange {
                harness: HARNESS_KEY.to_owned(),
                kind: ArtifactKind::HarnessSpecific,
                path: Self::repo_root(&self.agent_descriptor_path())?,
                description: format!("remove .claude/agents/{SERVER_NAME}.md subagent descriptor"),
                is_update: true,
            });
        }

        Ok(InstallReport {
            planned_changes,
            warnings: vec![],
        })
    }

    /// Apply a previously computed [`Self::plan_uninstall`] report: delete
    /// each targeted file, restoring `~/.claude.json` to a value-merge
    /// with the `enforcer` entry removed (preserving every unrelated key).
    ///
    /// # Errors
    /// Returns [`InstallError::Io`] if a removal fails.
    pub fn apply_uninstall(&self, report: &InstallReport) -> InstallResult<ApplyResult> {
        let mut applied = Vec::with_capacity(report.planned_changes.len());
        let sessionstart_command =
            Self::hook_entry_command(&Self::desired_sessionstart_hook(&self.binary_path))
                .unwrap_or_default()
                .to_owned();
        let pretooluse_command =
            Self::hook_entry_command(&Self::desired_pretooluse_hook(&self.binary_path))
                .unwrap_or_default()
                .to_owned();
        for change in &report.planned_changes {
            let target = PathBuf::from(change.path.as_str());
            let backup_path = backup_before_write(&target)?;

            match change.kind {
                ArtifactKind::McpRegistration => {
                    let mut existing = self.read_claude_json()?;
                    Self::remove_mcp_server(&mut existing);
                    self.write_claude_json(&existing)?;
                }
                ArtifactKind::HarnessSpecific if target == self.claude_json_path() => {
                    let mut existing = self.read_claude_json()?;
                    Self::remove_hook_entry(
                        &mut existing,
                        SESSION_START_EVENT,
                        &sessionstart_command,
                    );
                    Self::remove_hook_entry(&mut existing, PRE_TOOL_USE_EVENT, &pretooluse_command);
                    self.write_claude_json(&existing)?;
                }
                _ => {
                    if target.is_file() {
                        std::fs::remove_file(&target).map_err(|e| Self::io_err(&target, e))?;
                    }
                }
            }

            applied.push(AppliedChange {
                change: change.clone(),
                succeeded: true,
                backup_path: backup_path.map(|p| Self::repo_root(&p)).transpose()?,
            });
        }
        Ok(ApplyResult { applied })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_contract::RequestContext;
    use std::fs;

    fn ctx(binary: &Path) -> RequestContext {
        RequestContext::with_defaults(binary.to_path_buf())
    }

    fn fixture_home() -> Result<tempfile::TempDir, std::io::Error> {
        tempfile::tempdir()
    }

    #[test]
    fn plan_on_fresh_home_proposes_every_artifact() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        let adapter = ClaudeAdapter::new(home.path(), &binary);
        let report = adapter.plan(&ctx(&binary))?;
        assert!(!report.is_noop());
        assert_eq!(report.planned_changes.len(), 5);
        Ok(())
    }

    #[test]
    fn full_install_apply_then_verify_all_green() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        fs::create_dir_all(binary.parent().ok_or("expected a parent dir")?)?;
        fs::write(&binary, b"fake-binary")?;
        let adapter = ClaudeAdapter::new(home.path(), &binary);

        let plan = adapter.plan(&ctx(&binary))?;
        assert!(!plan.is_noop());
        let applied = adapter.apply(&plan)?;
        assert!(applied.all_succeeded());

        let verify = adapter.verify(&ctx(&binary))?;
        assert!(
            verify.all_passed(),
            "expected all checks to pass, got {verify:?}"
        );

        // Idempotent re-install: second plan is a no-op.
        let second_plan = adapter.plan(&ctx(&binary))?;
        assert!(second_plan.is_noop());
        Ok(())
    }

    #[test]
    fn install_then_uninstall_restores_pre_state_byte_for_byte(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        fs::create_dir_all(binary.parent().ok_or("expected a parent dir")?)?;
        fs::write(&binary, b"fake-binary")?;

        // Pre-existing unrelated state that must survive round-trip.
        let claude_json_path = home.path().join(".claude.json");
        let pre_state = serde_json::json!({
            "mcpServers": {
                "codebase-memory-mcp": { "command": "/usr/bin/codebase-memory-mcp" }
            },
            "unrelatedTopLevelKey": "keep-me"
        });
        fs::write(&claude_json_path, serde_json::to_string_pretty(&pre_state)?)?;
        let pre_bytes = fs::read(&claude_json_path)?;

        let adapter = ClaudeAdapter::new(home.path(), &binary);
        let install_plan = adapter.plan(&ctx(&binary))?;
        adapter.apply(&install_plan)?;

        // Confirm the unrelated entry survived the install merge.
        let mid: Value = serde_json::from_slice(&fs::read(&claude_json_path)?)?;
        assert_eq!(
            mid["mcpServers"]["codebase-memory-mcp"]["command"],
            "/usr/bin/codebase-memory-mcp"
        );
        assert_eq!(mid["unrelatedTopLevelKey"], "keep-me");
        assert_eq!(mid["hooks"]["SessionStart"][0]["matcher"], "");
        assert_eq!(
            mid["hooks"]["PreToolUse"][0]["matcher"],
            "Edit|Write|MultiEdit"
        );
        assert!(mid["mcpServers"][SERVER_NAME].is_object());

        let uninstall_plan = adapter.plan_uninstall(&ctx(&binary))?;
        assert!(!uninstall_plan.is_noop());
        adapter.apply_uninstall(&uninstall_plan)?;

        let post_bytes = fs::read(&claude_json_path)?;
        // Byte-for-byte requires re-serializing pre_state the same way
        // this adapter writes (to_string_pretty) so the comparison is not
        // sensitive to incidental whitespace differences between the
        // test's own initial write and the adapter's round-trip write.
        let restored: Value = serde_json::from_slice(&post_bytes)?;
        assert_eq!(restored, pre_state);
        let reserialized_pre = serde_json::to_vec_pretty(&pre_state)?;
        assert_eq!(post_bytes, reserialized_pre);
        let _ = pre_bytes;

        assert!(!adapter.skill_md_path().is_file());
        assert!(!adapter.agent_descriptor_path().is_file());
        Ok(())
    }

    #[test]
    fn empty_home_round_trip_drops_empty_json_scaffolding() -> Result<(), Box<dyn std::error::Error>>
    {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        fs::create_dir_all(binary.parent().ok_or("expected a parent dir")?)?;
        fs::write(&binary, b"fake-binary")?;
        fs::write(
            home.path().join(".claude.json"),
            serde_json::to_string_pretty(&serde_json::json!({}))?,
        )?;

        let adapter = ClaudeAdapter::new(home.path(), &binary);
        adapter.apply(&adapter.plan(&ctx(&binary))?)?;
        adapter.apply_uninstall(&adapter.plan_uninstall(&ctx(&binary))?)?;

        let restored: Value = serde_json::from_slice(&fs::read(home.path().join(".claude.json"))?)?;
        assert_eq!(restored, serde_json::json!({}));
        Ok(())
    }

    #[test]
    fn merge_preserves_unrelated_mcp_servers_and_top_level_keys() {
        let mut existing = serde_json::json!({
            "mcpServers": {
                "codebase-memory-mcp": { "command": "/usr/bin/codebase-memory-mcp" }
            },
            "someOtherTopLevelKey": 42
        });
        let desired = ClaudeAdapter::desired_entry(Path::new("/abs/enforcer"));
        ClaudeAdapter::merge_mcp_server(&mut existing, desired.clone());

        assert_eq!(existing["someOtherTopLevelKey"], 42);
        assert_eq!(
            existing["mcpServers"]["codebase-memory-mcp"]["command"],
            "/usr/bin/codebase-memory-mcp"
        );
        assert_eq!(existing["mcpServers"][SERVER_NAME], desired);
    }

    #[test]
    fn entry_matches_detects_idempotent_noop() {
        let desired = ClaudeAdapter::desired_entry(Path::new("/abs/enforcer"));
        let mut existing = serde_json::json!({});
        ClaudeAdapter::merge_mcp_server(&mut existing, desired.clone());
        assert!(ClaudeAdapter::entry_matches(&existing, &desired));

        let other = ClaudeAdapter::desired_entry(Path::new("/other/enforcer"));
        assert!(!ClaudeAdapter::entry_matches(&existing, &other));
    }

    #[test]
    fn malformed_claude_json_is_a_detected_plan_failure() -> Result<(), Box<dyn std::error::Error>>
    {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        fs::write(home.path().join(".claude.json"), "{ not valid json")?;
        let adapter = ClaudeAdapter::new(home.path(), &binary);
        let result = adapter.plan(&ctx(&binary));
        assert!(matches!(result, Err(InstallError::MalformedConfig { .. })));
        Ok(())
    }

    #[test]
    fn verify_fails_closed_on_malformed_claude_json() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        fs::write(home.path().join(".claude.json"), "{ not valid json")?;
        let adapter = ClaudeAdapter::new(home.path(), &binary);
        let result = adapter.verify(&ctx(&binary));
        assert!(matches!(result, Err(InstallError::MalformedConfig { .. })));
        Ok(())
    }

    #[test]
    fn verify_fails_when_descriptor_missing() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        let mut root = serde_json::json!({});
        ClaudeAdapter::merge_mcp_server(&mut root, ClaudeAdapter::desired_entry(&binary));
        ClaudeAdapter::merge_hook_entry(
            &mut root,
            SESSION_START_EVENT,
            ClaudeAdapter::desired_sessionstart_hook(&binary),
        );
        ClaudeAdapter::merge_hook_entry(
            &mut root,
            PRE_TOOL_USE_EVENT,
            ClaudeAdapter::desired_pretooluse_hook(&binary),
        );
        fs::write(
            home.path().join(".claude.json"),
            serde_json::to_string(&root)?,
        )?;
        let adapter = ClaudeAdapter::new(home.path(), &binary);
        let report = adapter.verify(&ctx(&binary))?;
        assert!(!report.all_passed());
        let descriptor_check = report
            .checks
            .iter()
            .find(|c| c.name == "agent-descriptor-present")
            .ok_or("expected an agent-descriptor-present check")?;
        assert!(!descriptor_check.passed);
        Ok(())
    }

    #[test]
    fn verify_fails_when_descriptor_corrupt() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        let mut root = serde_json::json!({});
        ClaudeAdapter::merge_mcp_server(&mut root, ClaudeAdapter::desired_entry(&binary));
        ClaudeAdapter::merge_hook_entry(
            &mut root,
            SESSION_START_EVENT,
            ClaudeAdapter::desired_sessionstart_hook(&binary),
        );
        ClaudeAdapter::merge_hook_entry(
            &mut root,
            PRE_TOOL_USE_EVENT,
            ClaudeAdapter::desired_pretooluse_hook(&binary),
        );
        fs::write(
            home.path().join(".claude.json"),
            serde_json::to_string(&root)?,
        )?;
        let agents_dir = home.path().join(".claude").join("agents");
        fs::create_dir_all(&agents_dir)?;
        fs::write(
            agents_dir.join(format!("{SERVER_NAME}.md")),
            "not a valid frontmatter file at all",
        )?;
        let adapter = ClaudeAdapter::new(home.path(), &binary);
        let report = adapter.verify(&ctx(&binary))?;
        assert!(!report.all_passed());
        Ok(())
    }

    #[test]
    fn verify_fails_when_hook_entries_are_missing() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        let mut root = serde_json::json!({});
        ClaudeAdapter::merge_mcp_server(&mut root, ClaudeAdapter::desired_entry(&binary));
        fs::write(
            home.path().join(".claude.json"),
            serde_json::to_string(&root)?,
        )?;
        let agents_dir = home.path().join(".claude").join("agents");
        fs::create_dir_all(&agents_dir)?;
        fs::write(
            agents_dir.join(format!("{SERVER_NAME}.md")),
            ClaudeAdapter::render_agent_descriptor(),
        )?;
        let adapter = ClaudeAdapter::new(home.path(), &binary);
        let report = adapter.verify(&ctx(&binary))?;
        assert!(!report.all_passed());
        let sessionstart = report
            .checks
            .iter()
            .find(|c| c.name == "sessionstart-hook-present")
            .ok_or("expected a sessionstart-hook-present check")?;
        assert!(!sessionstart.passed);
        let pretooluse = report
            .checks
            .iter()
            .find(|c| c.name == "pretooluse-hook-present")
            .ok_or("expected a pretooluse-hook-present check")?;
        assert!(!pretooluse.passed);
        Ok(())
    }

    #[test]
    fn hook_merge_preserves_unrelated_entries_and_replaces_stale_enforcer_entries(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut root = serde_json::json!({
            "hooks": {
                "SessionStart": [
                    {
                        "matcher": "resume",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "/usr/bin/other-session",
                                "additionalContext": "keep me"
                            }
                        ]
                    },
                    {
                        "matcher": "",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "/abs/enforcer hooks sessionstart",
                                "additionalContext": "stale"
                            }
                        ]
                    }
                ],
                "PreToolUse": [
                    {
                        "matcher": "Read",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "/usr/bin/other-pretooluse",
                                "timeout": 5
                            }
                        ]
                    }
                ]
            }
        });
        ClaudeAdapter::merge_hook_entry(
            &mut root,
            SESSION_START_EVENT,
            ClaudeAdapter::desired_sessionstart_hook(Path::new("/abs/enforcer")),
        );
        ClaudeAdapter::merge_hook_entry(
            &mut root,
            PRE_TOOL_USE_EVENT,
            ClaudeAdapter::desired_pretooluse_hook(Path::new("/abs/enforcer")),
        );
        let session_entries = root["hooks"]["SessionStart"]
            .as_array()
            .ok_or("expected SessionStart array")?;
        assert_eq!(session_entries.len(), 2);
        assert_eq!(
            session_entries[0]["hooks"][0]["command"],
            "/usr/bin/other-session"
        );
        assert_eq!(
            session_entries[1]["hooks"][0]["additionalContext"],
            crate::hooks::sessionstart::reminder_body(Path::new("/abs/enforcer"))
        );
        let pretool_entries = root["hooks"]["PreToolUse"]
            .as_array()
            .ok_or("expected PreToolUse array")?;
        assert_eq!(pretool_entries.len(), 2);
        assert_eq!(
            pretool_entries[0]["hooks"][0]["command"],
            "/usr/bin/other-pretooluse"
        );
        assert_eq!(
            pretool_entries[1]["hooks"][0]["command"],
            "/abs/enforcer check --hook-mode=pretooluse"
        );
        Ok(())
    }

    #[test]
    fn agent_descriptor_renders_valid_frontmatter() {
        let rendered = ClaudeAdapter::render_agent_descriptor();
        assert!(ClaudeAdapter::validate_agent_descriptor(&rendered).is_ok());
        assert!(rendered.contains(&format!("name: {SERVER_NAME}")));
        assert!(rendered.contains("allow_implicit_invocation: true"));
    }

    #[test]
    fn validate_agent_descriptor_rejects_missing_fence() {
        let result = ClaudeAdapter::validate_agent_descriptor("no frontmatter here");
        assert!(result.is_err());
    }

    #[test]
    fn validate_agent_descriptor_rejects_unclosed_fence() {
        let result = ClaudeAdapter::validate_agent_descriptor("---\nname: enforcer\n");
        assert!(result.is_err());
    }

    #[test]
    fn validate_agent_descriptor_rejects_missing_implicit_invocation() {
        let result = ClaudeAdapter::validate_agent_descriptor("---\nname: enforcer\n---\nbody\n");
        assert!(result.is_err());
    }

    #[test]
    fn ctx_binary_path_mismatch_is_a_detected_error() -> Result<(), Box<dyn std::error::Error>> {
        let home = fixture_home()?;
        let binary = home.path().join("bin").join("enforcer");
        let other_binary = home.path().join("bin").join("other-enforcer");
        let adapter = ClaudeAdapter::new(home.path(), &binary);
        let result = adapter.plan(&ctx(&other_binary));
        assert!(matches!(result, Err(InstallError::MalformedConfig { .. })));
        Ok(())
    }
}
