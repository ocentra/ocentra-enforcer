//! f02 onboarding (`enforcer onboard <repo>`): the ratchet-first first-run
//! step that binds the enforcer to a repository. Before this runs, there is
//! no `.enforce/`, no project profile, no baseline, and no registration --
//! the engine has nothing to compare a scan against. [`onboard`] scaffolds
//! all of that in one idempotent call.
//!
//! # Ratchet-first (product-thesis-endorsed deviation)
//! Per `docs/PRODUCT_THESIS.md` ("Ratchet-first onboarding (f02)") and this
//! workpack's own "PRODUCT DIRECTION" note, onboarding a repo captures
//! every violation the arc-15 engine finds on the FIRST run into the
//! baseline (via [`crate::rules::baseline_ratchet`]) rather than requiring
//! a clean tree before onboarding can succeed. Existing findings are
//! grandfathered; only violations introduced AFTER onboarding fail a later
//! `enforcer check --baseline` run. The f02.md checklist itself does not
//! spell out this grandfather behavior -- this is an intentional,
//! owner-endorsed deviation toward "any repo starts green in minutes", not
//! an oversight.
//!
//! # What onboarding writes under `.enforce/`
//! - [`PROJECT_CONFIG_FILE`] (`.enforce/config`): the f03
//!   `enforcer_config::project_tie::ProjectConfig` serde shape (native-tool
//!   ties + declarative policy) -- written ONLY if absent. An existing
//!   config (and any waivers/native ties it carries) is loaded just far
//!   enough to validate it, never rewritten; a malformed or otherwise
//!   invalid pre-existing config fails onboarding rather than being
//!   silently replaced. This is the idempotency contract the proof row
//!   requires ("preserved waivers").
//! - [`BASELINE_FILE`] (`.enforce/baseline.json`): the d02
//!   [`crate::rules::baseline_ratchet::BaselineRecord`], capturing every
//!   current violation as a grandfathered baseline entry (the ratchet's
//!   prior baseline is always empty at onboard time -- this is the
//!   "capture", not the "check", step; d02's own gate is what ratchets it
//!   down/up on later runs).
//! - [`REGISTRATION_FILE`] (`.enforce/project.json`): a small,
//!   deterministic [`ProjectRegistration`] identifying the project by
//!   [`project_id`], so later commands can confirm a repo is known. This
//!   module only ever serializes the registration; the decode side belongs
//!   to consumer boundary modules (per the "deserialize at the boundary"
//!   doctrine), and the round-trip proof lives in `tests/onboard.rs`.
//!
//! # Project id (a recorded deviation)
//! The workpack calls for "a deterministic project id, an `enforcer-domain`
//! branded newtype". This module reuses [`enforcer_domain::hashes::Sha256`]
//! (already a branded `enforcer-domain` newtype) as that id -- the SHA-256
//! digest of the canonicalized [`RepoRoot`] string -- rather than minting a
//! dedicated `ProjectId` brand. This workpack's file grant is scoped to
//! `enforcer-scan/src/onboard.rs` and `enforcer-cli/src/onboard.rs` plus
//! additive one-line registrations elsewhere; adding a new branded type to
//! `enforcer-domain::ids` would be a substantive edit to a file outside
//! that grant. `Sha256` is deterministic, collision-resistant, and already
//! serde/`ts_rs`-wired, so it stands in cleanly. Recorded here as a named
//! deviation per this workpack's own proof-rule requirement.
//!
//! # MCP tool surface (a recorded deviation)
//! The workpack also calls for an MCP `onboard` tool calling this same
//! module. Wiring that tool lives in `enforcer-mcp`, a crate outside this
//! workpack's file grant (registering a new MCP tool is a substantive edit
//! to that crate's registry, not a one-line mod/lib/command addition).
//! [`onboard`] is written as a plain, CLI-agnostic function specifically so
//! that MCP wiring is a thin follow-up adapter over the same core, not a
//! redesign. Recorded here as a named deviation.

use std::path::Path;

use enforcer_config::project_tie::ProjectConfig;
use enforcer_core::error::DecodeError;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::paths::RepoRoot;

use crate::engine;
use crate::rules::baseline_ratchet::{self, Baseline};
use crate::scope::{self, ScopeRequest};
use crate::walk::{self, IgnoreRules};

/// Directory onboarding scaffolds under the repo root.
pub const ENFORCE_DIR: &str = ".enforce";
/// The f03 `ProjectConfig` file name under `.enforce/`.
pub const PROJECT_CONFIG_FILE: &str = "config";
/// The d02 baseline record file name under `.enforce/`.
pub const BASELINE_FILE: &str = "baseline.json";
/// The project registration file name under `.enforce/`.
pub const REGISTRATION_FILE: &str = "project.json";
/// Schema version of [`ProjectRegistration`]'s on-disk wire form.
pub const REGISTRATION_VERSION: u32 = 1;

/// Derive the deterministic project id for `repo_root`: the SHA-256 digest
/// of its canonical (normalized, absolute) string form. See the module docs
/// "Project id" section for why this reuses [`Sha256`] rather than a new
/// brand.
pub fn project_id(repo_root: &RepoRoot) -> Sha256 {
    Sha256::of(repo_root.as_str().as_bytes())
}

/// The `.enforce/project.json` registration record: what later commands
/// (`check --baseline`, MCP `scan`) read -- at their own decode boundary --
/// to confirm a repo is a known, onboarded project.
///
/// SERIALIZATION-DOC: `.enforce/project.json` wire form -- camelCase keys
/// (`version`, `projectId`, `repoRoot`), written pretty-printed by
/// `register_project`. Serialize-only in this module: decoding happens at
/// consumer boundary modules, never here (round-trip proven in
/// `tests/onboard.rs` against a boundary-side wire mirror).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[doc = "Registration record; see the serialization note above."]
pub struct ProjectRegistration {
    /// Schema version; see [`REGISTRATION_VERSION`].
    // BRAND-INVARIANT: a plain schema-version counter, always minted from
    // `REGISTRATION_VERSION` by `register_project` (the only constructor
    // path); private so no other value can ever populate it.
    version: u32,
    /// The deterministic project id (see [`project_id`]).
    pub project_id: Sha256,
    /// The repo root this registration was computed for.
    pub repo_root: RepoRoot,
}

/// Typed onboarding failure. Every variant fails closed -- there is no
/// silent-default path anywhere in [`onboard`]/[`require_onboarded`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Debug + Error are both implemented: Debug via the derive above, std::error::Error via the manual impl below (this crate deliberately carries no thiserror dependency; only enforcer-core does)."]
pub enum OnboardError {
    /// A filesystem or (de)serialization operation on `.enforce/` failed
    /// (create/read/write of the config, baseline, or registration file).
    Io {
        /// Display-form location of the failing path.
        // BRAND-INVARIANT: `location` and `reason` are rendered
        // error-report text only (a path's display form plus the underlying
        // failure), produced solely by `io_err` and never re-parsed back
        // into a path or error -- a raw String is the honest carrier here.
        location: String,
        /// The underlying failure, rendered.
        reason: String,
    },
    /// `.enforce/config` exists but failed to load/parse as a
    /// [`ProjectConfig`] (see [`enforcer_config::project_tie`]). Onboarding
    /// refuses to proceed rather than silently replacing an invalid or
    /// malformed config.
    ConfigLoad(enforcer_config::ConfigLoadError),
    /// A scope resolution or validator-registry decode failed during the
    /// baseline scan.
    Decode(DecodeError),
    /// The repo has not been onboarded: no baseline exists to compare
    /// against. Returned by [`require_onboarded`], never by [`onboard`]
    /// itself (which always creates the baseline it needs).
    NotOnboarded {
        /// The repo root that has no `.enforce/baseline.json`.
        repo_root: RepoRoot,
    },
}

impl std::fmt::Display for OnboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { location, reason } => {
                write!(f, "onboard: io failure at `{location}`: {reason}")
            }
            Self::ConfigLoad(inner) => {
                write!(f, "onboard: `.enforce/config` failed to load: {inner}")
            }
            Self::Decode(inner) => write!(f, "onboard: decode/validation failed: {inner}"),
            Self::NotOnboarded { repo_root } => write!(
                f,
                "onboard: `{repo_root}` has not been onboarded (no `.enforce/baseline.json`); \
                 run `enforcer onboard` first"
            ),
        }
    }
}

impl std::error::Error for OnboardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ConfigLoad(inner) => Some(inner),
            Self::Decode(inner) => Some(inner),
            Self::Io { .. } | Self::NotOnboarded { .. } => None,
        }
    }
}

impl From<DecodeError> for OnboardError {
    fn from(inner: DecodeError) -> Self {
        Self::Decode(inner)
    }
}

/// Whether [`onboard`] wrote a fresh default `.enforce/config` or found an
/// existing one and preserved it untouched (waivers included) -- the f02
/// idempotency contract, expressed as a domain enum rather than a bare
/// boolean flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Provisioning verdict; Debug is derived above intentionally."]
pub enum ConfigProvisioning {
    /// No config existed; the [`ProjectConfig::default`] shape was written.
    WroteDefault,
    /// A config already existed; it was validated but never rewritten.
    PreservedExisting,
}

/// The result of a successful [`onboard`] run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Onboard outcome record; Debug is derived above intentionally."]
pub struct OnboardOutcome {
    /// The deterministic project id registered.
    pub project_id: Sha256,
    /// The ratchet-first baseline captured: every violation present at
    /// onboard time, grandfathered (see the module docs).
    pub baseline: Baseline,
    /// Whether this run wrote a fresh default `.enforce/config` or
    /// preserved an existing one untouched.
    pub config: ConfigProvisioning,
}

fn io_err(path: &Path, reason: impl std::fmt::Display) -> OnboardError {
    OnboardError::Io {
        // ALLOC-JUSTIFICATION: error construction on the cold failure path
        // only -- the failing path and underlying reason are rendered once
        // into owned report text carried by the error.
        location: path.display().to_string(),
        reason: reason.to_string(),
    }
}

/// Write `.enforce/config` with [`ProjectConfig::default`] iff `config_path`
/// does not already exist. Idempotency contract: an existing config (and
/// any waivers/native ties it carries) is loaded ONLY to validate it (fail
/// closed on a malformed or invalid config), never rewritten.
fn write_default_config_if_absent(config_path: &Path) -> Result<ConfigProvisioning, OnboardError> {
    if config_path.exists() {
        // Validate, never rewrite: a malformed pre-existing config must
        // fail onboarding rather than being silently replaced.
        enforcer_config::project_tie::load_project_tie(config_path)
            .map_err(OnboardError::ConfigLoad)?;
        return Ok(ConfigProvisioning::PreservedExisting);
    }
    let default_config = ProjectConfig::default();
    let payload = serde_json::to_vec_pretty(&default_config).map_err(|e| io_err(config_path, e))?;
    std::fs::write(config_path, payload).map_err(|e| io_err(config_path, e))?;
    Ok(ConfigProvisioning::WroteDefault)
}

/// Run the ratchet-first baseline scan: every current violation across the
/// whole repo becomes a grandfathered baseline entry (the ratchet's prior
/// is always empty here -- see module docs). Writes the resulting
/// `BaselineRecord` to `baseline_path` and returns the captured
/// [`Baseline`].
fn capture_ratchet_first_baseline(
    repo_root: &RepoRoot,
    baseline_path: &Path,
) -> Result<Baseline, OnboardError> {
    let resolved = scope::resolve(&ScopeRequest::All, repo_root)?;
    let root_path = Path::new(repo_root.as_str());
    let files = walk::walk(root_path, &IgnoreRules::default()).map_err(|e| io_err(root_path, e))?;
    let validators = engine::build_family_validators()?;
    let report = engine::run(&resolved, &files, &validators);

    let outcome = baseline_ratchet::ratchet(&Baseline::default(), &report.violations);
    baseline_ratchet::write_baseline(baseline_path, &outcome.ratcheted_baseline)
        .map_err(|e| io_err(baseline_path, e))?;
    Ok(outcome.ratcheted_baseline)
}

/// Register the project deterministically: always (re)writes
/// `.enforce/project.json`. Idempotent by construction -- [`project_id`]
/// and the record shape are pure functions of `repo_root`, so re-running
/// never changes the file's bytes for an unchanged repo root.
fn register_project(
    repo_root: &RepoRoot,
    registration_path: &Path,
) -> Result<Sha256, OnboardError> {
    let registration = ProjectRegistration {
        version: REGISTRATION_VERSION,
        project_id: project_id(repo_root),
        // CLONE-JUSTIFICATION: the registration record owns its branded
        // repo root (it outlives this call as a serialized file); the
        // caller's borrow lives on unchanged.
        repo_root: repo_root.clone(),
    };
    let payload =
        serde_json::to_vec_pretty(&registration).map_err(|e| io_err(registration_path, e))?;
    std::fs::write(registration_path, payload).map_err(|e| io_err(registration_path, e))?;
    Ok(registration.project_id)
}

/// Onboard `repo_root`: create `.enforce/`, write (or preserve) the f03
/// project profile, capture a ratchet-first baseline over every current
/// violation, and register the project. Explicit (only ever invoked by a
/// caller, never auto-triggered by a scan) and idempotent -- see module
/// docs.
///
/// # Errors
/// Returns [`OnboardError`] if `.enforce/` cannot be created, an existing
/// `.enforce/config` fails to parse (malformed/invalid input never gets a
/// silent default), the baseline scan's scope fails to resolve, or any
/// write fails.
pub fn onboard(repo_root: &RepoRoot) -> Result<OnboardOutcome, OnboardError> {
    let enforce_dir = Path::new(repo_root.as_str()).join(ENFORCE_DIR);
    std::fs::create_dir_all(&enforce_dir).map_err(|e| io_err(&enforce_dir, e))?;

    let config = write_default_config_if_absent(&enforce_dir.join(PROJECT_CONFIG_FILE))?;
    let baseline = capture_ratchet_first_baseline(repo_root, &enforce_dir.join(BASELINE_FILE))?;
    let project_id = register_project(repo_root, &enforce_dir.join(REGISTRATION_FILE))?;

    Ok(OnboardOutcome {
        project_id,
        baseline,
        config,
    })
}

/// Require that `repo_root` has already been onboarded, returning the
/// loaded [`Baseline`] to compare a fresh scan against. Fails closed with
/// [`OnboardError::NotOnboarded`] when `.enforce/baseline.json` is absent --
/// there is no silent "treat as empty baseline" fallback for a repo that
/// was never onboarded (an onboarded repo's genuinely-empty baseline is a
/// different, present-file case, written explicitly by [`onboard`]).
///
/// # Errors
/// Returns [`OnboardError::NotOnboarded`] if no baseline file exists, or an
/// [`OnboardError::Io`] if the file exists but fails to load
/// (corrupt/tampered -- see [`baseline_ratchet::load_baseline`]).
pub fn require_onboarded(repo_root: &RepoRoot) -> Result<Baseline, OnboardError> {
    let baseline_path = Path::new(repo_root.as_str())
        .join(ENFORCE_DIR)
        .join(BASELINE_FILE);
    if !baseline_path.exists() {
        return Err(OnboardError::NotOnboarded {
            // CLONE-JUSTIFICATION: the fail-closed error owns the branded
            // root it names (it outlives this call); the caller's borrow
            // lives on unchanged.
            repo_root: repo_root.clone(),
        });
    }
    baseline_ratchet::load_baseline(&baseline_path).map_err(|e| io_err(&baseline_path, e))
}

#[cfg(test)]
mod tests {
    use super::{project_id, OnboardError};
    use enforcer_domain::paths::RepoRoot;

    // Behavioral coverage (scaffold on a fresh repo, "not onboarded"
    // fail-closed error on a repo with no `.enforce/`, idempotent re-run
    // preserving a hand-edited config with waivers, rejection of a
    // malformed pre-existing config, serde round-trip of the registration)
    // lives in `tests/onboard.rs` over `tests/fixtures/onboard/**` -- the
    // f02 proof row. These unit tests pin the pure `project_id` function.

    #[test]
    fn project_id_is_deterministic_for_the_same_root() -> Result<(), OnboardError> {
        let root: RepoRoot = "C:/Projects/enforcer".parse()?;
        assert_eq!(project_id(&root), project_id(&root));
        Ok(())
    }

    #[test]
    fn project_id_differs_for_different_roots() -> Result<(), OnboardError> {
        let first: RepoRoot = "C:/Projects/enforcer-a".parse()?;
        let second: RepoRoot = "C:/Projects/enforcer-b".parse()?;
        assert_ne!(project_id(&first), project_id(&second));
        Ok(())
    }

    // PROPERTY-TEST: `project_id` is a pure function of the canonical
    // RepoRoot string -- for ANY generated root, hashing twice agrees
    // (determinism property) and distinct roots never collide (injectivity
    // property over the generated set). Hand-rolled generator loop rather
    // than a proptest/quickcheck harness because neither crate is vendored
    // in this workspace.
    #[test]
    fn property_project_id_is_pure_and_collision_free_over_generated_roots(
    ) -> Result<(), OnboardError> {
        let mut seen = std::collections::BTreeSet::new();
        for raw in (1..=64_u32).map(|index| format!("C:/Projects/generated-{index}")) {
            let root: RepoRoot = raw.parse()?;
            assert_eq!(
                project_id(&root),
                project_id(&root),
                "hashing the same root twice must agree"
            );
            seen.insert(project_id(&root));
        }
        assert_eq!(
            seen.len(),
            64,
            "distinct roots must map to distinct project ids"
        );
        Ok(())
    }
}
