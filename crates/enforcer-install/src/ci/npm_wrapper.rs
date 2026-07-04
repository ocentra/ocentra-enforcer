//! c10 — the OPTIONAL npm wrapper package (`packages/enforcer-cli/` or
//! similar): a thin JS shim `package.json` with per-platform
//! `optionalDependencies`, the same pattern biome/esbuild/swc use, so a
//! consumer already wired the old Node-centric way (`npm install` +
//! `npx enforcer ...`, per `TARGET_REPO_WIRING.md`) keeps working with
//! ZERO wiring changes even though the underlying package now ships a
//! compiled binary.
//!
//! # Charter
//!
//! This module computes the per-platform optional-dependency package
//! name npm's own platform resolution (`os`/`cpu` fields in each
//! per-platform package's `package.json`) picks automatically, and
//! renders the wrapper package's own `package.json`. It does NOT publish
//! to npm or shell out to `npm` itself — that is a CI-time publish step,
//! out of scope for this pure-rendering module (mirrors every other
//! emitter in this crate).

use crate::ci::release_pipeline::BinaryVariant;
use crate::distribution::TargetPlatform;

/// The per-platform optional-dependency package name npm resolves for
/// `platform` (e.g. `"enforcer-cli-linux-x64"`,
/// `"enforcer-cli-darwin-arm64"`) — mirrors the Orhun-blog/cargo-npm
/// pattern: one tiny package per platform, each declaring `os`/`cpu` in
/// its own `package.json` so `npm install` only ever pulls the ONE
/// matching platform package, never all of them.
#[must_use]
pub fn optional_dependency_name(platform: TargetPlatform) -> &'static str {
    match platform {
        TargetPlatform::WindowsX86_64 => "enforcer-cli-win32-x64",
        TargetPlatform::MacX86_64 => "enforcer-cli-darwin-x64",
        TargetPlatform::MacAarch64 => "enforcer-cli-darwin-arm64",
        TargetPlatform::LinuxX86_64Gnu => "enforcer-cli-linux-x64",
        TargetPlatform::LinuxX86_64Musl => "enforcer-cli-linux-x64-musl",
        TargetPlatform::LinuxAarch64Gnu => "enforcer-cli-linux-arm64",
    }
}

/// The npm `os` field value(s) for `platform`'s per-platform package
/// `package.json`, so npm's own install-time platform check (not this
/// module) skips every non-matching platform package.
#[must_use]
pub fn npm_os(platform: TargetPlatform) -> &'static str {
    match platform {
        TargetPlatform::WindowsX86_64 => "win32",
        TargetPlatform::MacX86_64 | TargetPlatform::MacAarch64 => "darwin",
        TargetPlatform::LinuxX86_64Gnu
        | TargetPlatform::LinuxX86_64Musl
        | TargetPlatform::LinuxAarch64Gnu => "linux",
    }
}

/// The npm `cpu` field value for `platform`.
#[must_use]
pub fn npm_cpu(platform: TargetPlatform) -> &'static str {
    match platform {
        TargetPlatform::WindowsX86_64
        | TargetPlatform::MacX86_64
        | TargetPlatform::LinuxX86_64Gnu
        | TargetPlatform::LinuxX86_64Musl => "x64",
        TargetPlatform::MacAarch64 | TargetPlatform::LinuxAarch64Gnu => "arm64",
    }
}

/// Resolve the exact optional-dependency package name npm's platform
/// resolution would pick for a given `(os, arch, libc)` combination —
/// the logic a fixture asserts against without spinning up a real `npm
/// install`.
///
/// # Errors
/// Returns [`crate::error::InstallError::UnsupportedTarget`] when the
/// `(os, arch, libc)` combination has no entry in
/// [`TargetPlatform::all`].
pub fn resolve_optional_dependency(
    os: &str,
    arch: &str,
    libc: crate::ci::installer_scripts::Libc,
) -> crate::error::InstallResult<&'static str> {
    use crate::ci::installer_scripts::Libc;
    let platform = match (os, arch, libc) {
        ("win32", "x64", _) => TargetPlatform::WindowsX86_64,
        ("darwin", "x64", _) => TargetPlatform::MacX86_64,
        ("darwin", "arm64", _) => TargetPlatform::MacAarch64,
        ("linux", "x64", Libc::Gnu) => TargetPlatform::LinuxX86_64Gnu,
        ("linux", "x64", Libc::Musl) => TargetPlatform::LinuxX86_64Musl,
        ("linux", "arm64", _) => TargetPlatform::LinuxAarch64Gnu,
        _ => {
            return Err(crate::error::InstallError::UnsupportedTarget {
                target: format!("{os}-{arch}-{libc:?}"),
            })
        }
    };
    Ok(optional_dependency_name(platform))
}

/// Render the thin JS shim wrapper package's `package.json`. Declares
/// every per-platform package as an `optionalDependencies` entry (npm
/// itself decides at install time which one actually resolves/downloads
/// based on the running host's `os`/`cpu`/`libc`), plus the `bin` entry
/// so `npx enforcer ...` keeps working unchanged.
#[must_use]
pub fn render_wrapper_package_json(version: &str) -> String {
    let mut optional_deps = String::new();
    for platform in TargetPlatform::all() {
        if !optional_deps.is_empty() {
            optional_deps.push_str(",\n");
        }
        optional_deps.push_str(&format!(
            "    \"{}\": \"{version}\"",
            optional_dependency_name(*platform)
        ));
    }

    format!(
        r#"{{
  "name": "enforcer-cli",
  "version": "{version}",
  "description": "Thin JS shim for the enforcer binary (npx enforcer ...). Per-platform optionalDependencies each bundle the matching release binary; the default CI-facing variant is lite ({default_variant}). cargo install from source remains a documented fallback for platforms without a prebuilt asset.",
  "bin": {{
    "enforcer": "./bin/enforcer.js"
  }},
  "optionalDependencies": {{
{optional_deps}
  }},
  "os": ["win32", "darwin", "linux"],
  "cpu": ["x64", "arm64"]
}}
"#,
        default_variant = BinaryVariant::ci_default().asset_label(),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        npm_cpu, npm_os, optional_dependency_name, render_wrapper_package_json,
        resolve_optional_dependency,
    };
    use crate::ci::installer_scripts::Libc;
    use crate::distribution::TargetPlatform;
    use crate::error::InstallError;

    #[test]
    fn every_platform_has_a_unique_optional_dependency_name() {
        let mut names: Vec<&str> = TargetPlatform::all()
            .iter()
            .map(|p| optional_dependency_name(*p))
            .collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            names.len(),
            total,
            "optional-dependency names must be unique per platform"
        );
    }

    #[test]
    fn npm_os_cpu_matches_node_platform_conventions() {
        assert_eq!(npm_os(TargetPlatform::WindowsX86_64), "win32");
        assert_eq!(npm_cpu(TargetPlatform::WindowsX86_64), "x64");
        assert_eq!(npm_os(TargetPlatform::MacAarch64), "darwin");
        assert_eq!(npm_cpu(TargetPlatform::MacAarch64), "arm64");
        assert_eq!(npm_os(TargetPlatform::LinuxAarch64Gnu), "linux");
        assert_eq!(npm_cpu(TargetPlatform::LinuxAarch64Gnu), "arm64");
    }

    #[test]
    fn resolve_optional_dependency_matches_every_declared_platform(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cases: &[(&str, &str, Libc, &str)] = &[
            ("win32", "x64", Libc::Gnu, "enforcer-cli-win32-x64"),
            ("darwin", "x64", Libc::Gnu, "enforcer-cli-darwin-x64"),
            ("darwin", "arm64", Libc::Gnu, "enforcer-cli-darwin-arm64"),
            ("linux", "x64", Libc::Gnu, "enforcer-cli-linux-x64"),
            ("linux", "x64", Libc::Musl, "enforcer-cli-linux-x64-musl"),
            ("linux", "arm64", Libc::Gnu, "enforcer-cli-linux-arm64"),
        ];
        for (os, arch, libc, expected) in cases {
            let resolved = resolve_optional_dependency(os, arch, *libc)?;
            assert_eq!(resolved, *expected);
        }
        Ok(())
    }

    #[test]
    fn resolve_optional_dependency_rejects_an_unreleased_combination() {
        let result = resolve_optional_dependency("plan9", "x64", Libc::Gnu);
        assert!(matches!(
            result,
            Err(InstallError::UnsupportedTarget { .. })
        ));
    }

    #[test]
    fn rendered_wrapper_package_json_lists_every_platform_as_optional_dependency(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json = render_wrapper_package_json("0.1.0");
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        let optional_deps = parsed
            .get("optionalDependencies")
            .and_then(|v| v.as_object())
            .ok_or("optionalDependencies object present")?;
        assert_eq!(optional_deps.len(), TargetPlatform::all().len());
        for platform in TargetPlatform::all() {
            assert!(
                optional_deps.contains_key(optional_dependency_name(*platform)),
                "missing optionalDependencies entry for {:?}",
                platform
            );
        }
        Ok(())
    }

    #[test]
    fn rendered_wrapper_package_json_is_valid_json_with_a_bin_entry(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json = render_wrapper_package_json("0.1.0");
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(
            parsed["bin"]["enforcer"].as_str(),
            Some("./bin/enforcer.js")
        );
        assert_eq!(parsed["version"].as_str(), Some("0.1.0"));
        Ok(())
    }

    #[test]
    fn rendered_wrapper_package_json_contains_no_hardcoded_local_absolute_path() {
        let json = render_wrapper_package_json("0.1.0");
        assert!(!json.contains("E:/"));
        assert!(!json.contains("C:/Projects"));
        assert!(!json.contains("/home/"));
    }
}
