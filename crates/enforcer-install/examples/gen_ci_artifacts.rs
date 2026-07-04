//! Dev-only generator: renders the c10 CI artifacts (action.yml,
//! install.sh, install.ps1, npm wrapper package.json) from this crate's
//! own `ci` module so the checked-in files can never drift from the Rust
//! source of truth. Run via `cargo run -p enforcer-install --example gen_ci_artifacts`.
use enforcer_install::ci::{github_action, installer_scripts, npm_wrapper};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let version = "0.1.0";

    let action_dir = root.join(".github/actions/enforcer-scan");
    fs::create_dir_all(&action_dir)?;
    fs::write(
        action_dir.join("action.yml"),
        github_action::render_action_yml(),
    )?;

    fs::write(
        root.join("install.sh"),
        installer_scripts::render_install_sh(version),
    )?;
    fs::write(
        root.join("install.ps1"),
        installer_scripts::render_install_ps1(version),
    )?;

    let npm_dir = root.join("packages/enforcer-cli");
    fs::create_dir_all(&npm_dir)?;
    fs::write(
        npm_dir.join("package.json"),
        npm_wrapper::render_wrapper_package_json(version),
    )?;

    Ok(())
}
