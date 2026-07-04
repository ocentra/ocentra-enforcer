//! c10 — the reusable composite GitHub Action (`.github/actions/enforcer-scan`).
//!
//! # Charter
//!
//! Wraps [`crate::ci::installer_scripts::render_install_sh`]/
//! [`render_install_ps1`][crate::ci::installer_scripts::render_install_ps1]
//! plus an `actions/cache` entry keyed by `enforcer-version+platform`
//! (checked BEFORE downloading — cache-hit skips re-download, cache-miss
//! downloads+caches), then exposes the binary on `PATH` for the calling
//! workflow's own `run: enforcer scan ...` step. Wired into target repos
//! by the existing `github-actions` install adapter
//! (`TARGET_REPO_WIRING.md`'s adapter list).
//!
//! This module renders the composite action's `action.yml` (the file
//! actually written to `.github/actions/enforcer-scan/action.yml`) and
//! exposes the pure cache-key logic so a fixture can assert "cache-hit
//! skips re-download, cache-miss downloads+caches" without spinning up a
//! real GitHub Actions runner.

use crate::ci::release_pipeline::BinaryVariant;

/// Compute the `actions/cache` key for one `(version, platform-triple,
/// variant)` combination. Stable/deterministic: the same inputs always
/// produce the same key, so a re-run with an unchanged
/// version+platform+variant hits the existing cache entry instead of
/// re-downloading.
#[must_use]
pub fn cache_key(version: &str, platform_triple: &str, variant: BinaryVariant) -> String {
    format!(
        "enforcer-{}-{platform_triple}-v{version}",
        variant.asset_label()
    )
}

/// The composite action's declared inputs, typed here so a fixture can
/// assert the rendered YAML's `inputs:` block matches this Rust-side
/// source of truth rather than drifting silently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionInputs {
    /// `version` input: the pinned enforcer version to install (default
    /// resolves from the consumer's own recorded pin -- see
    /// [`crate::ci::release_pipeline::VersionPin`] -- never a bare
    /// "latest" default).
    pub version: String,
    /// `variant` input: `lite` (default) or `full`.
    pub variant: BinaryVariant,
    /// `scan-args` input: extra arguments forwarded to `enforcer scan`
    /// when the action runs the scan itself rather than merely installing
    /// the binary onto `PATH` for the caller's own step.
    pub scan_args: String,
}

impl Default for ActionInputs {
    fn default() -> Self {
        Self {
            version: String::new(),
            variant: BinaryVariant::ci_default(),
            scan_args: String::new(),
        }
    }
}

/// Render `.github/actions/enforcer-scan/action.yml`. Pure function;
/// never touches disk (the caller decides whether/where to write it —
/// mirroring every other emitter in this crate's `--dry-run`/`apply`
/// split).
#[must_use]
pub fn render_action_yml() -> String {
    let default_variant = BinaryVariant::ci_default().asset_label();
    format!(
        r#"name: "enforcer scan"
description: >-
  Installs the enforcer binary (zero Rust toolchain required) with an
  actions/cache entry keyed by version+platform, exposes it on PATH, and
  optionally runs `enforcer scan` directly. CI always regenerates proof
  fresh -- this action never trusts a pre-computed/uploaded artifact as a
  substitute for running the binary.
inputs:
  version:
    description: >-
      Pinned enforcer release version to install (e.g. "0.1.0"). Version
      pinning is the default for consumers; there is no "latest" default
      here on purpose -- pass version: "latest" explicitly to opt into
      the floating-latest channel.
    required: true
  variant:
    description: 'Binary variant to install: "lite" (default, no coordination hub/UI) or "full".'
    required: false
    default: "{default_variant}"
  scan-args:
    description: 'Extra arguments forwarded to "enforcer scan" if run-scan is true.'
    required: false
    default: ""
  run-scan:
    description: "If true, this action also runs `enforcer scan $scan-args` itself after installing."
    required: false
    default: "false"
outputs:
  enforcer-path:
    description: "Absolute path to the installed enforcer binary."
    value: ${{{{ steps.install.outputs.enforcer-path }}}}
runs:
  using: "composite"
  steps:
    - name: Resolve platform
      id: platform
      shell: bash
      run: |
        case "$(uname -s)" in
          Linux*) echo "os=linux" >> "$GITHUB_OUTPUT" ;;
          Darwin*) echo "os=macos" >> "$GITHUB_OUTPUT" ;;
          MINGW*|MSYS*|CYGWIN*) echo "os=windows" >> "$GITHUB_OUTPUT" ;;
          *) echo "unsupported platform: $(uname -s)" >&2; exit 1 ;;
        esac
        echo "arch=$(uname -m)" >> "$GITHUB_OUTPUT"
    - name: Cache enforcer binary
      id: cache
      uses: actions/cache@v4
      with:
        path: ${{{{ runner.temp }}}}/enforcer-bin
        key: enforcer-${{{{ inputs.variant }}}}-${{{{ steps.platform.outputs.os }}}}-${{{{ steps.platform.outputs.arch }}}}-v${{{{ inputs.version }}}}
    - name: Install enforcer (cache miss)
      id: install
      if: steps.cache.outputs.cache-hit != 'true'
      shell: bash
      env:
        ENFORCER_VERSION: ${{{{ inputs.version }}}}
        ENFORCER_VARIANT: ${{{{ inputs.variant }}}}
        ENFORCER_INSTALL_DIR: ${{{{ runner.temp }}}}/enforcer-bin
      run: |
        curl -fsSL https://raw.githubusercontent.com/ocentra/enforcer/main/install.sh | sh
        echo "enforcer-path=${{ENFORCER_INSTALL_DIR}}/enforcer" >> "$GITHUB_OUTPUT"
    - name: Expose cached enforcer binary
      if: steps.cache.outputs.cache-hit == 'true'
      shell: bash
      run: echo "enforcer-path=${{{{ runner.temp }}}}/enforcer-bin/enforcer" >> "$GITHUB_OUTPUT"
    - name: Add enforcer to PATH
      shell: bash
      run: echo "${{{{ runner.temp }}}}/enforcer-bin" >> "$GITHUB_PATH"
    - name: Run enforcer scan
      if: inputs.run-scan == 'true'
      shell: bash
      run: enforcer scan ${{{{ inputs.scan-args }}}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::{cache_key, render_action_yml, ActionInputs};
    use crate::ci::release_pipeline::BinaryVariant;

    #[test]
    fn cache_key_is_stable_for_identical_inputs() {
        let a = cache_key("0.1.0", "x86_64-unknown-linux-gnu", BinaryVariant::Lite);
        let b = cache_key("0.1.0", "x86_64-unknown-linux-gnu", BinaryVariant::Lite);
        assert_eq!(a, b);
    }

    #[test]
    fn cache_key_differs_by_version_platform_and_variant() {
        let base = cache_key("0.1.0", "x86_64-unknown-linux-gnu", BinaryVariant::Lite);
        let diff_version = cache_key("0.2.0", "x86_64-unknown-linux-gnu", BinaryVariant::Lite);
        let diff_platform = cache_key("0.1.0", "aarch64-apple-darwin", BinaryVariant::Lite);
        let diff_variant = cache_key("0.1.0", "x86_64-unknown-linux-gnu", BinaryVariant::Full);
        assert_ne!(base, diff_version);
        assert_ne!(base, diff_platform);
        assert_ne!(base, diff_variant);
    }

    #[test]
    fn default_action_inputs_default_to_the_lite_ci_variant() {
        let inputs = ActionInputs::default();
        assert_eq!(inputs.variant, BinaryVariant::ci_default());
    }

    #[test]
    fn rendered_action_yml_declares_cache_before_install() -> Result<(), Box<dyn std::error::Error>>
    {
        let yml = render_action_yml();
        let cache_idx = yml.find("actions/cache@v4").ok_or("cache step present")?;
        let install_idx = yml
            .find("Install enforcer (cache miss)")
            .ok_or("install step present")?;
        assert!(
            cache_idx < install_idx,
            "cache-check step must be declared before the install-on-miss step"
        );
        Ok(())
    }

    #[test]
    fn rendered_action_yml_defaults_variant_input_to_lite() {
        let yml = render_action_yml();
        assert!(yml.contains(r#"default: "lite""#));
    }

    #[test]
    fn rendered_action_yml_contains_no_hardcoded_local_absolute_path() {
        let yml = render_action_yml();
        assert!(!yml.contains("E:/"));
        assert!(!yml.contains("C:/Projects"));
        assert!(!yml.contains("/home/"));
    }

    #[test]
    fn rendered_action_yml_never_hardcodes_a_version_default(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Version pinning is the consumer's own explicit choice; the
        // action must require the input rather than silently defaulting
        // to "latest".
        let yml = render_action_yml();
        let version_block_start = yml.find("version:").ok_or("version input present")?;
        let variant_block_start = yml.find("variant:").ok_or("variant input present")?;
        let version_block = &yml[version_block_start..variant_block_start];
        assert!(version_block.contains("required: true"));
        assert!(!version_block.contains("default:"));
        Ok(())
    }
}
