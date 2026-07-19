//! `enforcer-config` â€” typed config load, parse-at-boundary, 3-layer
//! resolution (arc-03).
//!
//! # Charter
//!
//! Three real layers, PRESERVED from the legacy `.mjs` model, not
//! flattened:
//! 1. Global/shared **profiles** (`strict`, `ocentra-enforcer`,
//!    `ocentra-parent`, `default`) â€” each a COMPLETE config shape, embedded
//!    into the binary at compile time so the engine is self-contained (no
//!    external `profiles/` directory required for baseline operation).
//! 2. The **per-project config** (`ocentra-enforcer.config.json`), which
//!    declares `profileName` (which global profile it layers on top of)
//!    plus local overrides. Mechanically self-checked: missing
//!    `schemaVersion`/`profileName`, or an unknown `profileName`, fails to
//!    LOAD with a typed [`error::ConfigLoadError`] (mirrors legacy
//!    `CFG-1.10`/`CFG-1.11`, promoted from runtime finding to load-time
//!    error).
//! 3. **Per-run OUTPUT** under `.enforce/runs/<id>/**` â€” NOT config, owned
//!    by arc-17/arc-18. This crate only resolves the `harness` sub-config
//!    those crates consume; it never writes to `.enforce/` itself.
//!
//! Downstream crates (scan/harness/proof/lang-*) consume only the resolved
//! [`model::EffectiveConfig`] â€” never raw files or environment variables
//! directly.

#[path = "boundary/serde/env.rs"]
pub mod env;
pub mod error;
pub mod model;
pub mod policy;
pub mod profiles;
pub mod project_tie;
pub mod resolve;
#[path = "boundary/serde.rs"]
pub mod serde;
pub mod shape;

use crate::env::ConfigEnv;
use crate::error::ConfigResult;
use enforcer_domain::config_types::EffectiveConfig;

/// Load the effective config for a project: reads `config_path` if it
/// exists, otherwise resolves the `default` profile alone. This is the one
/// entry point downstream crates should call; it performs the read (I/O)
/// and delegates parsing/merging to [`resolve::resolve`].
///
/// # Errors
/// Returns [`ConfigLoadError`] if the file exists but is unreadable, is
/// malformed JSON, is missing `schemaVersion`/`profileName`, or names an
/// unknown profile.
pub fn load_project_config(config_path: &std::path::Path) -> ConfigResult<EffectiveConfig> {
    match serde::read_config_json(config_path)? {
        Some((raw, source)) => resolve::resolve(Some(&raw), &source),
        None => {
            let source = serde::absent_project_config_source();
            resolve::resolve(None, &source)
        }
    }
}

/// Load the effective config the same way [`load_project_config`] does, but
/// first apply [`ConfigEnv`] overrides (a07 boundary requirement: env-var
/// overrides are decoded once, here, never read ad hoc downstream):
/// `ENFORCER_CONFIG_PATH` replaces `default_config_path` when set, and
/// `ENFORCER_PROFILE` forces the profile layer regardless of what the
/// project config's `profileName` declares.
///
/// # Errors
/// Returns [`ConfigLoadError::InvalidEnvVar`] if `ENFORCER_PROFILE` names an
/// unknown profile, or any [`load_project_config`] error for the resolved
/// path.
pub fn load_project_config_with_env(
    default_config_path: &std::path::Path,
) -> ConfigResult<EffectiveConfig> {
    let config_env = ConfigEnv::read()?;
    let config_path = config_env
        .config_path
        .as_deref()
        .unwrap_or(default_config_path);
    let mut effective = load_project_config(config_path)?;
    if let Some(profile_override) = config_env.profile_name {
        effective = resolve::resolve_profile_only(&profile_override)?;
    }
    Ok(effective)
}
