//! c02 — deterministic harness autodetect + per-harness capability
//! manifest.
//!
//! # Charter
//!
//! The legacy `.mjs` install scripts only ever knew Codex: they derived
//! `CODEX_HOME` from the environment and never probed for any other
//! harness. Installing across "any harness" (the global-install thesis,
//! RUST_ARCHITECTURE.md) first requires knowing which harnesses are
//! actually present on this machine. This module is that detection layer:
//! it probes a fixed set of candidate harness home directories (env
//! override first, then the conventional default), and returns one
//! [`DetectedHarness`] record per known harness — present or not, always
//! WITH evidence, never a guess.
//!
//! Beyond "is it installed", this module also DETECTS/DECLARES each
//! present harness's **capability manifest**
//! ([`HarnessCapabilities`]): the machine-readable answer to "what
//! agentic primitives does this harness actually have" (max concurrent
//! agents, sub-agent nesting depth, background tasks, scheduled tasks,
//! cross-session messaging, implicit invocation). The orchestrator
//! (EXECUTION_MODEL.md §3b) reads this manifest to ADAPT / gracefully
//! degrade instead of assuming the enforcer's full target agentic model
//! exists everywhere. This module only DETECTS and DECLARES; the
//! adapt/degrade logic itself is the orchestrator's, not this crate's.
//!
//! # Purity (unit-testable with temp fixtures)
//!
//! The probe core never reads `std::env`/the real filesystem directly —
//! every accessor is injected via [`EnvSource`]/[`FsSource`], so a test
//! can seed an isolated temp-dir "home" and an isolated env map and get a
//! fully deterministic, hermetic result. [`RealEnv`]/[`RealFs`] are the
//! only place this module touches ambient globals, and they are trivial
//! pass-throughs a caller wires up outside the probe core (`enforcer
//! install`/`doctor`, c01).
//!
//! # Fail-closed
//!
//! An unprovable/ambiguous state is reported as `present: false` (for
//! detection) or `Cap::Unknown`/`Support::Unknown` (for a capability
//! field) WITH the [`Evidence`] that led to that conclusion — never
//! silently guessed `true`/`Yes`.
//!
//! # Deviation note (workpack c02, ownership)
//!
//! The workpack floats [`HarnessId`] as "a branded `enforcer-domain`
//! newtype" but this workpack's `owns:` line grants only
//! `crates/enforcer-install/src/detect.rs` (+ its fixtures) — it does NOT
//! grant `crates/enforcer-domain/src/ids.rs`. Per the workpack's own
//! fallback instruction ("otherwise define crate-locally and flag"),
//! [`HarnessId`] is defined here, crate-local, using the exact same
//! parse-at-boundary/no-bare-string-constructor shape as the
//! `enforcer_domain::ids::branded_string!` family (`RuleId`, `HubName`,
//! `LaneId`, ...) so a future promotion into `enforcer-domain` is a pure
//! move, not a redesign. Flagged for the orchestrator/hub: promote
//! `HarnessId` (and `Cap`/`Support` if desired workspace-wide) into
//! `enforcer-domain::ids` in a follow-up that owns that file.

use enforcer_core::error::DecodeError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------
// HarnessId — crate-local branded newtype (see module doc deviation note)
// ---------------------------------------------------------------------

/// Branded harness identifier (e.g. `"claude"`, `"codex"`, `"gemini"`).
/// Validates on construction; no bare-string constructor, matching the
/// `enforcer_domain::ids` brand shape (parse-at-boundary, camelCase-safe
/// on the wire via `serde(try_from = "String", into = "String")`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct HarnessId(String);

impl HarnessId {
    /// View the validated inner value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for HarnessId {
    type Error = DecodeError;

    fn try_from(raw: String) -> Result<Self, DecodeError> {
        validate_harness_id(&raw)?;
        Ok(Self(raw))
    }
}

impl std::str::FromStr for HarnessId {
    type Err = DecodeError;

    fn from_str(raw: &str) -> Result<Self, DecodeError> {
        Self::try_from(raw.to_owned())
    }
}

impl From<HarnessId> for String {
    fn from(value: HarnessId) -> String {
        value.0
    }
}

impl std::fmt::Display for HarnessId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn validate_harness_id(raw: &str) -> Result<(), DecodeError> {
    let ok = !raw.is_empty()
        && raw.len() <= 64
        && raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "harnessId",
            "expected lowercase kebab-case (e.g. `claude`, `codex`, `kilocode`)",
        ))
    }
}

/// Every known harness this module probes for, in the fixed order the
/// workpack lists them. Adding a new harness (future workpack) is a
/// one-line addition here plus a `HarnessProbe` entry in
/// [`probe_specs`] — the detection core itself never special-cases a
/// harness name in control flow.
pub const KNOWN_HARNESS_IDS: &[&str] = &[
    "claude",
    "codex",
    "gemini",
    "antigravity",
    "cursor",
    "windsurf",
    "zed",
    "opencode",
    "aider",
    "kilocode",
    "kiro",
];

// ---------------------------------------------------------------------
// Evidence — how a detection/capability conclusion was reached
// ---------------------------------------------------------------------

/// One concrete observation backing a detection or capability
/// conclusion. Every [`DetectedHarness::present`] value and every
/// [`HarnessCapabilities`] field is paired with a list of these so a
/// human (or `enforcer doctor`) can see exactly WHY the module concluded
/// what it did — never an unexplained boolean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    /// The home-relative or absolute path inspected (env var name for an
    /// env-sourced observation, e.g. `"env:CODEX_HOME"`).
    pub source: String,
    /// What was observed there (e.g. `"directory exists"`,
    /// `"not found"`, `"allow_implicit_invocation: true"`).
    pub observation: String,
}

impl Evidence {
    /// Build an evidence entry.
    #[must_use]
    pub fn new(source: impl Into<String>, observation: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            observation: observation.into(),
        }
    }
}

// ---------------------------------------------------------------------
// Capability manifest primitives
// ---------------------------------------------------------------------

/// A bounded/unbounded/unknown numeric capacity (e.g. max concurrent
/// agents, max sub-agent nesting depth). `Unknown` is the fail-closed
/// default — an undetectable cap is NEVER guessed as `Unbounded` or a
/// specific bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Cap {
    /// A concrete, detected upper bound.
    Bounded(u32),
    /// Detected as having no enforced upper bound.
    Unbounded,
    /// Not detectable from available evidence — fail-closed default.
    #[default]
    Unknown,
}

/// Whether a harness supports a binary agentic primitive (background
/// tasks, scheduled tasks, cross-session messaging, implicit
/// invocation). `Unknown` is the fail-closed default — an undetectable
/// primitive is NEVER declared `Yes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Support {
    /// Detected as present.
    Yes,
    /// Detected as absent.
    No,
    /// Not detectable from available evidence — fail-closed default.
    #[default]
    Unknown,
}

/// A capability value paired with the [`Evidence`] that justifies it.
/// Every field of [`HarnessCapabilities`] is one of these so a value is
/// never reported without its provenance.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapValue {
    /// The detected/declared capacity.
    pub value: Cap,
    /// Evidence backing `value`. Empty only when `value` is the
    /// fail-closed [`Cap::Unknown`] default with no probe attempted.
    pub evidence: Vec<Evidence>,
}

impl CapValue {
    /// A [`Cap::Unknown`] value with the evidence explaining why nothing
    /// could be determined.
    #[must_use]
    pub fn unknown(evidence: Vec<Evidence>) -> Self {
        Self {
            value: Cap::Unknown,
            evidence,
        }
    }
}

/// A support value paired with the [`Evidence`] that justifies it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportValue {
    /// The detected/declared support state.
    pub value: Support,
    /// Evidence backing `value`. Empty only when `value` is the
    /// fail-closed [`Support::Unknown`] default with no probe attempted.
    pub evidence: Vec<Evidence>,
}

impl SupportValue {
    /// A [`Support::Unknown`] value with the evidence explaining why
    /// nothing could be determined.
    #[must_use]
    pub fn unknown(evidence: Vec<Evidence>) -> Self {
        Self {
            value: Support::Unknown,
            evidence,
        }
    }
}

/// A present harness's declared agentic primitives + limits — the
/// machine-readable "what can this harness actually do" record the
/// orchestrator (EXECUTION_MODEL.md §3b, arc-16) reads to decide how to
/// degrade gracefully instead of assuming the enforcer's full target
/// agentic model exists everywhere. Produced at install AND doctor time.
/// Every field defaults fail-closed to `Unknown`/`Support::Unknown` when
/// no probe evidence exists — this module never guesses `Yes`/a bound.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessCapabilities {
    /// Concurrency cap on simultaneously active agents/sub-agents.
    pub max_concurrent_agents: CapValue,
    /// Max sub-agent nesting depth (flat-only vs multi-tier).
    pub sub_agent_nesting_depth: CapValue,
    /// Background-task support.
    pub background_tasks: SupportValue,
    /// Scheduled-task / cron / automation support. When absent, the
    /// orchestrator polls for mail instead of relying on a scheduled
    /// mail-check (EXECUTION_MODEL.md §3b).
    pub scheduled_tasks: SupportValue,
    /// Cross-session / direct inter-agent messaging support (Codex
    /// strong; other harnesses weaker/none). When absent, the
    /// orchestrator falls back to manual/human-relayed handoff.
    pub cross_session_messaging: SupportValue,
    /// Implicit-invocation support (e.g. Codex `allow_implicit_invocation`).
    pub implicit_invocation: SupportValue,
}

// ---------------------------------------------------------------------
// Detected-harness record
// ---------------------------------------------------------------------

/// One normalized detection result — present or not, always with
/// evidence — for a single [`HarnessId`]. c01's orchestrators (`install`/
/// `doctor`) consume a `Vec<DetectedHarness>` to pick adapters when no
/// explicit `--only <harness>` list is pinned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedHarness {
    /// Which harness this record is for.
    pub id: HarnessId,
    /// Whether this harness was detected as installed on this machine.
    pub present: bool,
    /// The home directory probed (env override or the conventional
    /// default), regardless of whether it was found to exist. `None`
    /// only when a harness defines no home-dir convention at all (not
    /// currently the case for any [`KNOWN_HARNESS_IDS`] entry).
    pub home_path: Option<PathBuf>,
    /// Evidence backing the `present` conclusion.
    pub evidence: Vec<Evidence>,
    /// The capability manifest for this harness. `None` when `present`
    /// is `false` — an absent harness has no capabilities to declare
    /// (an empty-home fixture emits no manifests, per the workpack's
    /// acceptance row).
    pub capabilities: Option<HarnessCapabilities>,
}

// ---------------------------------------------------------------------
// Pure fs/env accessors — the injection seam that makes this testable
// ---------------------------------------------------------------------

/// Pure environment-variable accessor. The probe core never calls
/// `std::env::var` directly; every env read goes through this trait so a
/// test can inject a closed, deterministic env map instead of the
/// process's real (test-order-dependent, CI-polluted) environment.
pub trait EnvSource {
    /// Read one environment variable by name, or `None` if unset.
    fn get(&self, key: &str) -> Option<String>;
}

/// Pure filesystem accessor. The probe core never calls `std::fs`/
/// `Path::exists` directly; every filesystem read goes through this
/// trait so a test can inject an isolated temp-dir "home" instead of the
/// real one, and assert exact `Evidence` text without flakiness from
/// whatever happens to exist on the machine running the test.
pub trait FsSource {
    /// Whether `path` exists and is a directory.
    fn is_dir(&self, path: &Path) -> bool;
    /// Whether `path` exists and is a regular file.
    fn is_file(&self, path: &Path) -> bool;
    /// Read a file's contents as UTF-8 text, or `None` if it does not
    /// exist / is not valid UTF-8.
    fn read_to_string(&self, path: &Path) -> Option<String>;
}

/// The real, ambient-global-backed [`EnvSource`]. The ONLY place this
/// module touches `std::env` — callers (c01's `install`/`doctor` wiring)
/// construct this once outside the pure probe core.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealEnv;

impl EnvSource for RealEnv {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

/// The real, ambient-global-backed [`FsSource`]. The ONLY place this
/// module touches `std::fs`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealFs;

impl FsSource for RealFs {
    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn read_to_string(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
}

// ---------------------------------------------------------------------
// Probe specification — per-harness env override + default-home rule
// ---------------------------------------------------------------------

/// How to locate one harness's home directory: an env-var override
/// (checked first, highest precedence) and a fallback rule against the
/// user's home directory (`USERPROFILE`/`HOME`, whichever `EnvSource`
/// supplies — Windows sets `USERPROFILE`, POSIX sets `HOME`; a caller's
/// `EnvSource` may supply either or both).
struct HarnessProbe {
    id: &'static str,
    /// Env var that, if set, is used verbatim as the home path (e.g.
    /// `CODEX_HOME`, `CLAUDE_HOME`) — highest precedence, overrides the
    /// conventional default.
    env_override: &'static str,
    /// Directory name under the user's home dir used when no env
    /// override is set (e.g. `.codex`, `.claude`, `.gemini`).
    default_dir_name: &'static str,
}

/// The fixed probe table, in [`KNOWN_HARNESS_IDS`] order. `CODEX_HOME`/
/// `CLAUDE_HOME` are honored explicitly per the workpack; every other
/// harness follows the same env-override-then-dotdir convention with its
/// own `<HARNESS>_HOME`-shaped variable name and dot-directory.
fn probe_specs() -> Vec<HarnessProbe> {
    vec![
        HarnessProbe {
            id: "claude",
            env_override: "CLAUDE_HOME",
            default_dir_name: ".claude",
        },
        HarnessProbe {
            id: "codex",
            env_override: "CODEX_HOME",
            default_dir_name: ".codex",
        },
        HarnessProbe {
            id: "gemini",
            env_override: "GEMINI_HOME",
            default_dir_name: ".gemini",
        },
        HarnessProbe {
            id: "antigravity",
            env_override: "ANTIGRAVITY_HOME",
            default_dir_name: ".antigravity",
        },
        HarnessProbe {
            id: "cursor",
            env_override: "CURSOR_HOME",
            default_dir_name: ".cursor",
        },
        HarnessProbe {
            id: "windsurf",
            env_override: "WINDSURF_HOME",
            default_dir_name: ".windsurf",
        },
        HarnessProbe {
            id: "zed",
            env_override: "ZED_HOME",
            default_dir_name: ".zed",
        },
        HarnessProbe {
            id: "opencode",
            env_override: "OPENCODE_HOME",
            default_dir_name: ".opencode",
        },
        HarnessProbe {
            id: "aider",
            env_override: "AIDER_HOME",
            default_dir_name: ".aider",
        },
        HarnessProbe {
            id: "kilocode",
            env_override: "KILOCODE_HOME",
            default_dir_name: ".kilocode",
        },
        HarnessProbe {
            id: "kiro",
            env_override: "KIRO_HOME",
            default_dir_name: ".kiro",
        },
    ]
}

/// Resolve the user's home directory from `env`, preferring `HOME`
/// (POSIX) then falling back to `USERPROFILE` (Windows) — either source
/// may be absent depending on what the caller's [`EnvSource`] carries.
fn resolve_user_home(env: &dyn EnvSource) -> Option<PathBuf> {
    env.get("HOME")
        .or_else(|| env.get("USERPROFILE"))
        .map(PathBuf::from)
}

// ---------------------------------------------------------------------
// Detection core
// ---------------------------------------------------------------------

/// Probe every [`KNOWN_HARNESS_IDS`] entry against `env`/`fs` and return
/// one [`DetectedHarness`] per harness, in [`KNOWN_HARNESS_IDS`] order.
/// Pure over the injected accessors — no ambient `std::env`/`std::fs`
/// reads happen inside this function itself.
///
/// # Errors
/// Returns a [`DecodeError`] only if a [`KNOWN_HARNESS_IDS`] entry itself
/// fails [`HarnessId`] validation (a defensive check against this
/// module's own constant table drifting out of sync with
/// [`validate_harness_id`] — not reachable via any external input).
pub fn detect_harnesses(
    env: &dyn EnvSource,
    fs: &dyn FsSource,
) -> Result<Vec<DetectedHarness>, DecodeError> {
    let user_home = resolve_user_home(env);
    let mut out = Vec::with_capacity(probe_specs().len());
    for probe in probe_specs() {
        out.push(detect_one(&probe, env, fs, user_home.as_deref())?);
    }
    Ok(out)
}

/// Detect a single harness per `probe`'s env-override-then-default-dir
/// rule, then (when present) build its capability manifest.
fn detect_one(
    probe: &HarnessProbe,
    env: &dyn EnvSource,
    fs: &dyn FsSource,
    user_home: Option<&Path>,
) -> Result<DetectedHarness, DecodeError> {
    let id: HarnessId = probe.id.parse()?;

    let (home_path, mut evidence) = match env.get(probe.env_override) {
        Some(overridden) if overridden.trim().is_empty() => {
            let ev = Evidence::new(
                format!("env:{}", probe.env_override),
                "override is blank; refusing to resolve an ambiguous harness home".to_owned(),
            );
            (None, vec![ev])
        }
        Some(overridden) => {
            let path = PathBuf::from(&overridden);
            let ev = Evidence::new(
                format!("env:{}", probe.env_override),
                format!("override set to `{overridden}`"),
            );
            (Some(path), vec![ev])
        }
        None => match user_home {
            Some(home) => {
                let path = home.join(probe.default_dir_name);
                let ev = Evidence::new(
                    format!("env:{}", probe.env_override),
                    "not set; falling back to default home-relative directory".to_owned(),
                );
                (Some(path), vec![ev])
            }
            None => {
                let ev = Evidence::new(
                    "env:HOME|USERPROFILE",
                    "neither HOME nor USERPROFILE is set; cannot resolve a default home directory"
                        .to_owned(),
                );
                (None, vec![ev])
            }
        },
    };

    let present = match &home_path {
        Some(path) => {
            let exists = fs.is_dir(path);
            evidence.push(Evidence::new(
                path.display().to_string(),
                if exists {
                    "directory exists".to_owned()
                } else {
                    "directory not found".to_owned()
                },
            ));
            exists
        }
        None => false,
    };

    let capabilities = if present {
        Some(capability_manifest_for(probe.id, home_path.as_deref(), fs))
    } else {
        None
    };

    Ok(DetectedHarness {
        id,
        present,
        home_path,
        evidence,
        capabilities,
    })
}

// ---------------------------------------------------------------------
// Capability manifest detection
// ---------------------------------------------------------------------

/// Build the capability manifest for a present harness identified by
/// `harness_id`, probing `home` (already known to exist) via `fs`.
/// Every field starts `Unknown`/`Support::Unknown` and is only upgraded
/// away from that default when concrete on-disk evidence is found —
/// fail-closed by construction, never a guessed `Yes`.
fn capability_manifest_for(
    harness_id: &str,
    home: Option<&Path>,
    fs: &dyn FsSource,
) -> HarnessCapabilities {
    match harness_id {
        "codex" => codex_capability_manifest(home, fs),
        _ => HarnessCapabilities::default(),
    }
}

/// Codex's manifest: the only currently-known on-disk signal is
/// `agents/openai.yaml` under the Codex home, whose
/// `allow_implicit_invocation: true` line is direct evidence of implicit
/// invocation support, and (per the workpack) also justifies declaring
/// strong cross-session messaging (Codex's mail/thread primitives are a
/// first-class feature of the same agent config surface). Every other
/// field has no on-disk detector yet, so it stays the fail-closed
/// `Unknown` default with evidence recording that no probe exists.
fn codex_capability_manifest(home: Option<&Path>, fs: &dyn FsSource) -> HarnessCapabilities {
    let Some(home) = home else {
        return HarnessCapabilities::default();
    };
    let marker_path = home.join("agents").join("openai.yaml");
    let marker_path_str = marker_path.display().to_string();

    let Some(contents) = fs.read_to_string(&marker_path) else {
        let no_marker_observation = "file not found; no probe available";
        return HarnessCapabilities {
            implicit_invocation: SupportValue::unknown(vec![Evidence::new(
                marker_path_str.as_str(),
                no_marker_observation,
            )]),
            cross_session_messaging: SupportValue::unknown(vec![Evidence::new(
                marker_path_str,
                no_marker_observation,
            )]),
            ..HarnessCapabilities::default()
        };
    };

    let allows_implicit = contents
        .lines()
        .map(str::trim)
        .any(|line| line == "allow_implicit_invocation: true");

    if allows_implicit {
        let ev = Evidence::new(&marker_path_str, "allow_implicit_invocation: true");
        HarnessCapabilities {
            implicit_invocation: SupportValue {
                value: Support::Yes,
                evidence: vec![ev],
            },
            cross_session_messaging: SupportValue {
                value: Support::Yes,
                evidence: vec![Evidence::new(
                    &marker_path_str,
                    "allow_implicit_invocation marker present; Codex's mail/thread primitives \
                     are a first-class part of the same agent-config surface",
                )],
            },
            ..HarnessCapabilities::default()
        }
    } else {
        let ev = Evidence::new(
            &marker_path_str,
            "present but does not set allow_implicit_invocation: true",
        );
        HarnessCapabilities {
            implicit_invocation: SupportValue {
                value: Support::No,
                evidence: vec![ev],
            },
            ..HarnessCapabilities::default()
        }
    }
}

// ---------------------------------------------------------------------
// Test-only in-memory accessors
// ---------------------------------------------------------------------

/// An in-memory [`EnvSource`] for unit tests — a closed, deterministic
/// env map with no dependency on the real process environment.
#[derive(Debug, Clone, Default)]
pub struct MapEnv(BTreeMap<String, String>);

impl MapEnv {
    /// An empty env map (models "nothing set").
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set one env var in this map, builder-style.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.0.insert(key.into(), value.into());
        self
    }
}

impl EnvSource for MapEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
}

/// A real-filesystem-backed [`FsSource`] rooted nowhere in particular —
/// used by tests that seed an actual temp directory (`tempfile::tempdir`)
/// and want real `is_dir`/`read_to_string` semantics against it without
/// touching the ambient environment for the HOME/env half of the probe.
/// This is [`RealFs`] under another name kept local to tests for clarity
/// at call sites (`TempHomeFs::default()` reads oddly; `RealFs` reads
/// misleadingly next to a temp-dir fixture) — same impl, re-exported.
pub type TempHomeFs = RealFs;

#[cfg(test)]
mod tests {
    use super::{
        detect_harnesses, Cap, DetectedHarness, EnvSource, HarnessId, MapEnv, RealFs, Support,
        KNOWN_HARNESS_IDS,
    };
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Build a temp dir standing in for a user's home, with the given
    /// dot-directories created inside it (each optionally carrying an
    /// `agents/openai.yaml` body for the Codex capability-manifest
    /// fixtures).
    struct TempHome {
        dir: tempfile::TempDir,
    }

    impl TempHome {
        fn new() -> Result<Self, std::io::Error> {
            Ok(Self {
                dir: tempfile::tempdir()?,
            })
        }

        fn path(&self) -> PathBuf {
            self.dir.path().to_path_buf()
        }

        fn seed_dir(&self, name: &str) -> Result<(), std::io::Error> {
            fs::create_dir_all(self.dir.path().join(name))
        }

        fn seed_codex_agents_yaml(&self, body: &str) -> Result<(), std::io::Error> {
            let agents_dir = self.dir.path().join(".codex").join("agents");
            fs::create_dir_all(&agents_dir)?;
            fs::write(agents_dir.join("openai.yaml"), body)
        }
    }

    fn env_with_home(home: &Path) -> MapEnv {
        MapEnv::new().with("HOME", home.display().to_string())
    }

    fn find<'a>(
        records: &'a [DetectedHarness],
        id: &str,
    ) -> Result<&'a DetectedHarness, Box<dyn std::error::Error>> {
        records
            .iter()
            .find(|r| r.id.as_str() == id)
            .ok_or_else(|| format!("harness id {id:?} must be in KNOWN_HARNESS_IDS").into())
    }

    #[test]
    fn known_harness_ids_are_all_valid_harness_ids() {
        for raw in KNOWN_HARNESS_IDS {
            let parsed: Result<HarnessId, _> = raw.parse();
            assert!(parsed.is_ok(), "expected {raw:?} to be a valid HarnessId");
        }
    }

    #[test]
    fn empty_home_yields_no_false_positives() -> Result<(), Box<dyn std::error::Error>> {
        let home = TempHome::new()?;
        let env = env_with_home(&home.path());
        let fs = RealFs;

        let records = detect_harnesses(&env, &fs)?;
        assert_eq!(records.len(), KNOWN_HARNESS_IDS.len());
        for record in &records {
            assert!(
                !record.present,
                "expected {} absent on an empty home, got present",
                record.id
            );
            assert!(
                !record.evidence.is_empty(),
                "expected evidence even for an absent harness"
            );
            assert!(
                record.capabilities.is_none(),
                "an absent harness must emit no capability manifest"
            );
        }
        Ok(())
    }

    #[test]
    fn seeded_claude_and_codex_dirs_are_detected_present() -> Result<(), Box<dyn std::error::Error>>
    {
        let home = TempHome::new()?;
        home.seed_dir(".claude")?;
        home.seed_dir(".codex")?;
        let env = env_with_home(&home.path());
        let fs = RealFs;

        let records = detect_harnesses(&env, &fs)?;
        assert!(find(&records, "claude")?.present);
        assert!(find(&records, "codex")?.present);

        // Every other known harness must remain absent -- the exact
        // detected-adapter set, not a superset.
        for id in KNOWN_HARNESS_IDS {
            if *id == "claude" || *id == "codex" {
                continue;
            }
            assert!(
                !find(&records, id)?.present,
                "expected {id} absent when only .claude/.codex were seeded"
            );
        }
        Ok(())
    }

    #[test]
    fn seeded_dir_reports_the_resolved_home_path() -> Result<(), Box<dyn std::error::Error>> {
        let home = TempHome::new()?;
        home.seed_dir(".gemini")?;
        let env = env_with_home(&home.path());
        let fs = RealFs;

        let records = detect_harnesses(&env, &fs)?;
        let gemini = find(&records, "gemini")?;
        assert!(gemini.present);
        assert_eq!(
            gemini.home_path.as_deref(),
            Some(home.path().join(".gemini").as_path())
        );
        Ok(())
    }

    #[test]
    fn env_override_takes_precedence_over_the_default_dir() -> Result<(), Box<dyn std::error::Error>>
    {
        let home = TempHome::new()?;
        // Seed the CONVENTIONAL default (~/.codex) but point CODEX_HOME at
        // a DIFFERENT directory that does not exist -- if precedence were
        // wrong (default winning over the override), this would incorrectly
        // report `present: true`.
        home.seed_dir(".codex")?;
        let overridden_missing = home.path().join("codex-elsewhere");
        let env = env_with_home(&home.path())
            .with("CODEX_HOME", overridden_missing.display().to_string());
        let fs = RealFs;

        let records = detect_harnesses(&env, &fs)?;
        let codex = find(&records, "codex")?;
        assert!(
            !codex.present,
            "env override must take precedence over the default dir even when the default exists"
        );
        assert_eq!(
            codex.home_path.as_deref(),
            Some(overridden_missing.as_path())
        );
        Ok(())
    }

    #[test]
    fn env_override_pointing_at_a_real_dir_is_detected_present(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let home = TempHome::new()?;
        home.seed_dir("codex-elsewhere")?;
        let overridden = home.path().join("codex-elsewhere");
        let env = env_with_home(&home.path()).with("CODEX_HOME", overridden.display().to_string());
        let fs = RealFs;

        let records = detect_harnesses(&env, &fs)?;
        let codex = find(&records, "codex")?;
        assert!(codex.present);
        assert_eq!(codex.home_path.as_deref(), Some(overridden.as_path()));
        Ok(())
    }

    #[test]
    fn missing_home_env_yields_absent_with_evidence_not_a_panic(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let env = MapEnv::new(); // neither HOME nor USERPROFILE set
        let fs = RealFs;

        let records = detect_harnesses(&env, &fs)?;
        for record in &records {
            assert!(!record.present);
            assert!(record
                .evidence
                .iter()
                .any(|e| e.observation.contains("neither HOME nor USERPROFILE")));
        }
        Ok(())
    }

    #[test]
    fn caps_codex_fixture_declares_implicit_invocation_and_cross_session_messaging(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let home = TempHome::new()?;
        home.seed_codex_agents_yaml("name: openai\nallow_implicit_invocation: true\n")?;
        let env = env_with_home(&home.path());
        let fs = RealFs;

        let records = detect_harnesses(&env, &fs)?;
        let codex = find(&records, "codex")?;
        assert!(codex.present);
        let caps = codex
            .capabilities
            .as_ref()
            .ok_or("present harness must carry a capability manifest")?;

        assert_eq!(caps.implicit_invocation.value, Support::Yes);
        assert!(!caps.implicit_invocation.evidence.is_empty());
        assert_eq!(caps.cross_session_messaging.value, Support::Yes);
        assert!(!caps.cross_session_messaging.evidence.is_empty());

        // Fields with no detector yet must stay fail-closed Unknown, never
        // guessed Yes just because implicit invocation was detected.
        assert_eq!(caps.max_concurrent_agents.value, Cap::Unknown);
        assert_eq!(caps.sub_agent_nesting_depth.value, Cap::Unknown);
        assert_eq!(caps.background_tasks.value, Support::Unknown);
        assert_eq!(caps.scheduled_tasks.value, Support::Unknown);
        Ok(())
    }

    #[test]
    fn caps_claude_bare_fixture_declares_unprovable_fields_unknown(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let home = TempHome::new()?;
        home.seed_dir(".claude")?; // bare -- no capability markers at all
        let env = env_with_home(&home.path());
        let fs = RealFs;

        let records = detect_harnesses(&env, &fs)?;
        let claude = find(&records, "claude")?;
        assert!(claude.present);
        let caps = claude
            .capabilities
            .as_ref()
            .ok_or("present harness must carry a capability manifest")?;

        assert_eq!(caps.max_concurrent_agents.value, Cap::Unknown);
        assert_eq!(caps.sub_agent_nesting_depth.value, Cap::Unknown);
        assert_eq!(caps.background_tasks.value, Support::Unknown);
        assert_eq!(caps.scheduled_tasks.value, Support::Unknown);
        assert_eq!(caps.cross_session_messaging.value, Support::Unknown);
        assert_eq!(caps.implicit_invocation.value, Support::Unknown);
        Ok(())
    }

    #[test]
    fn codex_marker_without_the_true_flag_is_support_no_not_yes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let home = TempHome::new()?;
        home.seed_codex_agents_yaml("name: openai\nallow_implicit_invocation: false\n")?;
        let env = env_with_home(&home.path());
        let fs = RealFs;

        let records = detect_harnesses(&env, &fs)?;
        let codex = find(&records, "codex")?;
        let caps = codex
            .capabilities
            .as_ref()
            .ok_or("present harness must carry a capability manifest")?;
        assert_eq!(caps.implicit_invocation.value, Support::No);
        // A false marker is not evidence for cross-session messaging
        // either -- must not silently inherit Yes from a sibling field.
        assert_eq!(caps.cross_session_messaging.value, Support::Unknown);
        Ok(())
    }

    #[test]
    fn detection_is_pure_same_inputs_yield_identical_output(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let home = TempHome::new()?;
        home.seed_dir(".claude")?;
        let env = env_with_home(&home.path());
        let fs = RealFs;

        let first = detect_harnesses(&env, &fs)?;
        let second = detect_harnesses(&env, &fs)?;
        assert_eq!(first, second);
        Ok(())
    }

    #[test]
    fn detected_harness_round_trips_through_json_camel_case(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let home = TempHome::new()?;
        home.seed_codex_agents_yaml("allow_implicit_invocation: true\n")?;
        let env = env_with_home(&home.path());
        let fs = RealFs;

        let records = detect_harnesses(&env, &fs)?;
        let codex = find(&records, "codex")?;
        let wire = serde_json::to_string(codex)?;
        assert!(wire.contains("\"homePath\""));
        assert!(wire.contains("\"implicitInvocation\""));
        assert!(wire.contains("\"crossSessionMessaging\""));
        let back: DetectedHarness = serde_json::from_str(&wire)?;
        assert_eq!(back, *codex);
        Ok(())
    }

    #[test]
    fn harness_id_serde_rejects_malformed_ids() {
        let outcome = serde_json::from_str::<HarnessId>("\"Not Valid\"");
        assert!(outcome.is_err());
    }

    #[allow(dead_code)]
    fn assert_env_source_object_safe(_: &dyn EnvSource) {}
}
