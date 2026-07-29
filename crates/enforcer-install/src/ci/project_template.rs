//! Reusable, deterministic CI contract for Enforcer consumer repositories.
//!
//! The workflow is intentionally self-contained: it installs a pinned release
//! through the checksum-verifying installer before it invokes Enforcer.  The
//! memory command is written for the full CLI transport currently being wired;
//! it does not silently fall back to the legacy `codebase-memory-mcp` binary.

use crate::error::{InstallError, InstallResult};
use enforcer_domain::install_types::{
    CheckStatus, CheckSubject, InstallReportText, InstallRootPath, InstallVerifyCheck,
    OverwriteMode,
};
use std::path::Path;

const WORKFLOW_PATH: &str = ".github/workflows/ocentra-enforcer-ci.yml";
const ENFORCER_VERSION: &str = "0.1.0";
const CHECKOUT_ACTION: &str = "actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5";
const CACHE_ACTION: &str = "actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830";

/// A single-line command that is safe to place in a YAML `run: |` block.
///
/// The template is deliberately not an arbitrary shell-script transport.
/// Commands may contain ordinary executable/argument text and `&&`, but not
/// control characters, expansion syntax, redirections, comments, or command
/// separators. This keeps a caller supplied value from changing YAML shape or
/// executing an unintended second shell program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCiCommand(InstallReportText);

impl ProjectCiCommand {
    /// Borrow the validated command domain value.
    #[must_use]
    pub fn text(&self) -> &InstallReportText {
        &self.0
    }
}

impl TryFrom<InstallReportText> for ProjectCiCommand {
    type Error = InstallError;

    fn try_from(value: InstallReportText) -> Result<Self, Self::Error> {
        let text = value.as_str();
        if text.is_empty()
            || text.chars().any(char::is_control)
            || text.chars().any(|character| {
                !character.is_ascii_alphanumeric()
                    && !matches!(character, ' ' | '.' | '_' | '-' | '/' | ':' | '=' | '&')
            })
            || text
                .split("&&")
                .any(|part| part.is_empty() || part.contains('&'))
        {
            return Err(InstallError::InvalidCiCommand {
                path: WORKFLOW_PATH,
                reason: "CI commands must be non-empty single-line argument text; only alphanumeric characters, spaces, . _ - / : =, and paired && are allowed",
            });
        }
        Ok(Self(value))
    }
}

/// Supported repository stacks. Stack selection changes only the default
/// local-parity command; the mechanical Enforcer gates are identical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    /// Cargo workspace or Rust package.
    Rust,
    /// Node/npm workspace or package.
    Node,
    /// Repository containing both Rust and Node surfaces.
    Hybrid,
}

/// Typed commands injected at the project's real tool boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCiCommands {
    local_parity: ProjectCiCommand,
    release_rehearsal: Option<ProjectCiCommand>,
}

/// Fully rendered workflow bytes ready to cross the filesystem boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCiWorkflow(InstallReportText);

impl ProjectCiWorkflow {
    /// Return the validated workflow source ready for filesystem emission.
    #[must_use]
    pub fn content(&self) -> &InstallReportText {
        &self.0
    }
}

impl ProjectCiCommands {
    /// Create a profile from already validated command values.
    pub fn new(
        local_parity: ProjectCiCommand,
        release_rehearsal: Option<ProjectCiCommand>,
    ) -> Self {
        Self {
            local_parity,
            release_rehearsal,
        }
    }

    /// Select the stack-specific local parity command.
    pub fn for_kind(kind: ProjectKind) -> InstallResult<Self> {
        let local = match kind {
            ProjectKind::Rust => "cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-targets --all-features",
            ProjectKind::Node => "npm run ci:local",
            ProjectKind::Hybrid => "npm run ci:local && cargo test --workspace --all-targets --all-features",
        };
        Ok(Self::new(
            // ALLOC-JUSTIFICATION: the selected static profile must become an owned domain value.
            ProjectCiCommand::try_from(InstallReportText::try_from(local.to_owned())?)?,
            None,
        ))
    }
}

/// Render the repeated clean-runner Enforcer bootstrap step.
fn install_step() -> InstallResult<InstallReportText> {
    // ALLOC-JUSTIFICATION: each rendered workflow owns its reusable setup block.
    Ok(InstallReportText::try_from(format!(
        r#"      - name: Install pinned Enforcer with checksum verification
        shell: bash
        env:
          ENFORCER_VERSION: "{ENFORCER_VERSION}"
          ENFORCER_VARIANT: lite
          ENFORCER_INSTALL_DIR: ${{{{ runner.temp }}}}/enforcer-bin
        run: |
          curl --fail --location --silent --show-error "https://raw.githubusercontent.com/ocentra/enforcer/v${{ENFORCER_VERSION}}/install.sh" -o "$RUNNER_TEMP/install-enforcer.sh"
          sh "$RUNNER_TEMP/install-enforcer.sh"
          echo "$ENFORCER_INSTALL_DIR" >> "$GITHUB_PATH"
          "$ENFORCER_INSTALL_DIR/enforcer" --version
"#
    ))?)
}

/// Render the complete reusable workflow. Output is stable for equal input.
pub fn render(commands: &ProjectCiCommands) -> InstallResult<ProjectCiWorkflow> {
    // ALLOC-JUSTIFICATION: the rendered workflow owns the reusable setup block.
    let setup = install_step()?;
    let release_step = commands
        .release_rehearsal
        .as_ref()
        .map_or_else(String::new, |command| {
            // ALLOC-JUSTIFICATION: optional release text is materialized into the workflow bytes.
            format!(
                "\n      - name: Release rehearsal\n        run: |\n          {}\n",
                command.text().as_str()
            )
        });
    // ALLOC-JUSTIFICATION: formatting materializes the complete deterministic workflow artifact.
    Ok(ProjectCiWorkflow(InstallReportText::try_from(format!(
        r#"name: Ocentra Enforcer CI

on:
  pull_request:
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: ocentra-enforcer-${{{{ github.workflow }}}}-${{{{ github.ref }}}}
  cancel-in-progress: true

jobs:
  preflight:
    name: Enforcer Preflight
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: {CHECKOUT_ACTION}
        with:
          fetch-depth: 0
{setup}      - name: Index a fresh ephemeral memory store
        run: |
          rm -rf "$RUNNER_TEMP/enforcer-memory-store"
          enforcer memory cli --json index_repository --repo-path . --stores-dir "$RUNNER_TEMP/enforcer-memory-store" --mode fast
      - name: Graph exclusion proof capability
        run: |
          # The typed query exists, but the CLI has no typed expected-count/zero-result assertion yet.
          # Required future proof: query_graph over this storesDir for File nodes named target/, target-*, .tmp/, and .tmp-* with an expected count of zero.
          echo "BLOCKED: memory CLI lacks a typed zero-result assertion for graph exclusion proof"
          exit 1
      - name: Enforce source and architecture
        run: enforcer scan --all
      - name: Prove CI verification contract
        run: enforcer verify --mode ci --all

  validate:
    name: Platform Validation (${{{{ matrix.os }}}})
    needs: [preflight]
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{{{ matrix.os }}}}
    steps:
      - uses: {CHECKOUT_ACTION}
        with:
          fetch-depth: 0
{setup}      - name: Restore dependency cache
        uses: {CACHE_ACTION}
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            ~/.npm
          key: ${{{{ runner.os }}}}-enforcer-${{{{ hashFiles('**/Cargo.lock', '**/package-lock.json', 'ocentra-enforcer.config.json') }}}}
      - name: Run the exact local parity command
        run: |
          {local}

  policy:
    name: Security and Dependency Policy
    needs: [preflight]
    runs-on: ubuntu-latest
    steps:
      - uses: {CHECKOUT_ACTION}
        with:
          fetch-depth: 0
{setup}      - name: Enforce complete policy scan
        run: enforcer scan --all{release}

  required:
    name: Enforcer Required Gate
    if: always()
    needs: [preflight, validate, policy]
    runs-on: ubuntu-latest
    steps:
      - name: Require every gate
        shell: bash
        run: |
          test "${{{{ needs.preflight.result }}}}" = success
          test "${{{{ needs.validate.result }}}}" = success
          test "${{{{ needs.policy.result }}}}" = success
"#,
        setup = setup.as_str(),
        local = commands.local_parity.text().as_str(),
        release = release_step,
    ))?))
}

/// Install the generated workflow, preserving an existing file unless forced.
pub fn install(
    root: &InstallRootPath,
    commands: &ProjectCiCommands,
    overwrite: OverwriteMode,
) -> InstallResult<()> {
    let target = root.join_target(Path::new(WORKFLOW_PATH));
    if target.as_path().exists() && overwrite == OverwriteMode::PreserveExisting {
        return Ok(());
    }
    let parent = target.as_path().parent().ok_or_else(|| InstallError::Io {
        // ALLOC-JUSTIFICATION: diagnostics retain the filesystem path.
        path: target.as_path().display().to_string(),
        // ALLOC-JUSTIFICATION: diagnostics own the stable failure explanation.
        reason: "workflow target has no parent directory".to_owned(),
    })?;
    std::fs::create_dir_all(parent).map_err(|error| InstallError::Io {
        // ALLOC-JUSTIFICATION: diagnostics retain the failing path and I/O explanation.
        path: parent.display().to_string(),
        reason: error.to_string(),
    })?;
    let rendered = render(commands)?;
    std::fs::write(target.as_path(), rendered.content().as_str()).map_err(|error| {
        // ALLOC-JUSTIFICATION: the filesystem failure report owns the path and I/O explanation.
        InstallError::Io {
            path: target.as_path().display().to_string(),
            reason: error.to_string(),
        }
    })
}

/// Fail closed when the generated workflow is missing or byte-drifted.
pub fn verify(
    root: &InstallRootPath,
    commands: &ProjectCiCommands,
) -> InstallResult<InstallVerifyCheck> {
    let target = root.join_target(Path::new(WORKFLOW_PATH));
    let actual = match std::fs::read_to_string(target.as_path()) {
        Ok(value) => Some(value),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(InstallError::Io {
                // ALLOC-JUSTIFICATION: diagnostics retain the failing path and I/O explanation.
                path: target.as_path().display().to_string(),
                reason: error.to_string(),
            });
        }
    };
    let rendered = render(commands)?;
    let passed = actual.as_deref() == Some(rendered.content().as_str());
    Ok(InstallVerifyCheck {
        // ALLOC-JUSTIFICATION: the returned verification report owns its subject path.
        subject: CheckSubject::SkillAsset(InstallReportText::try_from(
            target.as_path().display().to_string(),
        )?),
        // ALLOC-JUSTIFICATION: the returned verification report owns its stable check name.
        name: InstallReportText::try_from("reusable-ci-template-parity".to_owned())?,
        status: if passed {
            CheckStatus::Passed
        } else {
            CheckStatus::Failed
        },
        // ALLOC-JUSTIFICATION: the returned verification report owns its outcome detail.
        detail: InstallReportText::try_from(if passed {
            "generated workflow matches the reusable CI contract".to_owned()
        } else {
            format!(
                "missing or drifted generated workflow at `{}`",
                target.as_path().display()
            )
        })?,
    })
}
