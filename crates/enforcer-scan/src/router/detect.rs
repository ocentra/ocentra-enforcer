//! Language detection (f05, stage 1): reuse the arc-13 literal-scan
//! ext->language registry (~65 langs) plus manifest sniffing to produce a
//! deterministic, ordered set of detected [`DetectedLanguage`]s for a
//! repo-relative file list.
//!
//! Deterministic and offline: no network, no filesystem access beyond the
//! caller-supplied path list (the caller is [`crate::walk`]'s output, or a
//! test fixture's file list), so the same input always detects the same
//! set, in the same order.

use std::collections::BTreeSet;

use enforcer_domain::paths::RelPath;
use serde::{Deserialize, Serialize};

/// A detected implementation language, keyed to the language-family
/// partition the landed `enforcer-lang-*` crates use. Distinct from
/// [`super::LanguageFamily`] (the arc-15 skeleton's coarse by-extension
/// classifier): this enum is f05's own, richer detection surface, driven by
/// the full arc-13 ~65-language registry plus manifest sniffing, not just a
/// 6-way extension match.
#[doc = "SERDE-TAG-JUSTIFICATION: detected languages are closed camelCase tokens carried in `RoutePlanDto.languages`; adding a tag would change that stable array contract without adding discrimination beyond the enum value itself."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DetectedLanguage {
    /// `Cargo.toml` present, or any `.rs` file.
    Rust,
    /// `package.json`/`tsconfig.json` present, or any `.ts`/`.tsx`/`.js`/
    /// `.jsx`/`.mjs`/`.cjs` file.
    TypeScript,
    /// `pyproject.toml`/`setup.py` present, or any `.py`/`.pyw` file.
    Python,
    /// `pubspec.yaml` present, or any `.dart` file.
    Dart,
    /// `go.mod` present, or any `.go` file.
    Go,
    /// `box.json` present (BoxLang), or any `.cfc`/`.cfm` file (ColdFusion/
    /// CFML — arc-13's `coldfusion` registry entry covers the extensions;
    /// `box.json` is the manifest signal).
    Cfml,
    /// Any other extensioned file with no dedicated `enforcer-lang-*` pack
    /// (whether or not the arc-13 registry itself recognizes the
    /// extension) — routes to the literal-scan universal floor only, never
    /// a T1 blocker.
    Other,
}

/// A manifest file whose mere presence signals a language, independent of
/// any single file's extension (e.g. an empty `Cargo.toml`-only repo with
/// no `.rs` files yet still detects as Rust).
struct ManifestSignal {
    /// Repo-relative basename to match exactly.
    basename: &'static str,
    /// Language this manifest implies.
    language: DetectedLanguage,
}

const MANIFEST_SIGNALS: &[ManifestSignal] = &[
    ManifestSignal {
        basename: "Cargo.toml",
        language: DetectedLanguage::Rust,
    },
    ManifestSignal {
        basename: "package.json",
        language: DetectedLanguage::TypeScript,
    },
    ManifestSignal {
        basename: "tsconfig.json",
        language: DetectedLanguage::TypeScript,
    },
    ManifestSignal {
        basename: "pyproject.toml",
        language: DetectedLanguage::Python,
    },
    ManifestSignal {
        basename: "setup.py",
        language: DetectedLanguage::Python,
    },
    ManifestSignal {
        basename: "pubspec.yaml",
        language: DetectedLanguage::Dart,
    },
    ManifestSignal {
        basename: "go.mod",
        language: DetectedLanguage::Go,
    },
    ManifestSignal {
        basename: "box.json",
        language: DetectedLanguage::Cfml,
    },
];

/// Map an arc-13 [`enforcer_literal_scan::LanguageFamily`] + extension to a
/// [`DetectedLanguage`]. Extensions not covered by a dedicated
/// `enforcer-lang-*` pack fall to [`DetectedLanguage::Other`] — recognized
/// by the literal-scan registry, but with no T1 pack to route to yet.
fn language_for_extension(ext: &str) -> Option<DetectedLanguage> {
    let lower = ext.to_ascii_lowercase();
    match lower.as_str() {
        "rs" => Some(DetectedLanguage::Rust),
        "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" => {
            Some(DetectedLanguage::TypeScript)
        }
        "py" | "pyw" => Some(DetectedLanguage::Python),
        "dart" => Some(DetectedLanguage::Dart),
        "go" => Some(DetectedLanguage::Go),
        "cfc" | "cfm" => Some(DetectedLanguage::Cfml),
        _ => None,
    }
}

/// Detect the set of languages present in `paths` (already walked,
/// repo-relative). Deterministic — the returned set is a [`BTreeSet`], so
/// iteration order is stable regardless of input order.
///
/// Detection is a union of two signals:
/// - manifest sniffing: an exact-basename match against
///   [`MANIFEST_SIGNALS`] (covers a repo with a manifest but no source
///   files yet).
/// - extension sniffing: any file whose extension maps to one of the
///   [`DetectedLanguage`] variants with a dedicated `enforcer-lang-*` pack.
///
/// Every other file with an extension (mapped or not, arc-13-registered or
/// not) contributes [`DetectedLanguage::Other`] — literal-scan (arc-13) is
/// the universal floor and scans every file regardless of whether its
/// extension is in its own ~65-language registry (`include_unknown`), so
/// the router's `Other` bucket mirrors that: any such file is "detected"
/// only for the universal literal-scan floor, never masquerading as a
/// T1-blocking language pack. A file with NO extension and no manifest
/// match (e.g. a bare `Dockerfile`-shaped name not in
/// [`MANIFEST_SIGNALS`]) contributes nothing — extensionless files are rare
/// enough that literal-scan's own basename-driven role classification,
/// not this router, is the right place to special-case them.
pub fn detect_languages(paths: &[RelPath]) -> BTreeSet<DetectedLanguage> {
    let mut found = BTreeSet::new();

    for path in paths {
        let as_str = path.as_str();
        let basename = as_str.rsplit('/').next().unwrap_or(as_str);
        let is_manifest = MANIFEST_SIGNALS.iter().any(|signal| {
            if basename == signal.basename {
                found.insert(signal.language);
                true
            } else {
                false
            }
        });
        if is_manifest {
            // A manifest file (`Cargo.toml`, `package.json`, ...) is a
            // build/config artifact, not a generic source file — its own
            // extension (e.g. `.toml`, `.json`) must not also register a
            // spurious `Other` language on top of the manifest signal.
            continue;
        }

        let ext = as_str.rsplit('.').next().unwrap_or("");
        if ext == as_str {
            // No extension (e.g. `Dockerfile`, `Makefile`) — manifest
            // sniffing above already covers the basenames we care about.
            continue;
        }
        match language_for_extension(ext) {
            Some(lang) => found.insert(lang),
            None => found.insert(DetectedLanguage::Other),
        };
    }

    found
}

#[cfg(test)]
mod tests {
    use super::{detect_languages, DetectedLanguage};
    use std::str::FromStr;

    fn rel(path: &str) -> Result<enforcer_domain::paths::RelPath, Box<dyn std::error::Error>> {
        Ok(enforcer_domain::paths::RelPath::from_str(path)?)
    }

    #[test]
    fn detects_rust_by_manifest_with_no_source_files() -> Result<(), Box<dyn std::error::Error>> {
        let paths = vec![rel("Cargo.toml")?];
        let detected = detect_languages(&paths);
        assert!(detected.contains(&DetectedLanguage::Rust));
        assert_eq!(detected.len(), 1);
        Ok(())
    }

    #[test]
    fn detects_rust_and_typescript_in_a_mixed_repo() -> Result<(), Box<dyn std::error::Error>> {
        let paths = vec![
            rel("Cargo.toml")?,
            rel("src/lib.rs")?,
            rel("package.json")?,
            rel("web/index.ts")?,
        ];
        let detected = detect_languages(&paths);
        assert!(detected.contains(&DetectedLanguage::Rust));
        assert!(detected.contains(&DetectedLanguage::TypeScript));
        assert_eq!(detected.len(), 2);
        Ok(())
    }

    #[test]
    fn detects_python_only_in_a_python_folder() -> Result<(), Box<dyn std::error::Error>> {
        let paths = vec![rel("pyproject.toml")?, rel("app/main.py")?];
        let detected = detect_languages(&paths);
        assert_eq!(detected, [DetectedLanguage::Python].into_iter().collect());
        Ok(())
    }

    #[test]
    fn unknown_extension_with_no_manifest_detects_other_only(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // `.qux` is not even in the arc-13 registry, but literal-scan's
        // universal floor still scans it (`include_unknown`), so the
        // router still detects `Other` — never a T1 language, and never
        // "nothing" (which would falsely suggest the file is unscannable).
        let paths = vec![rel("notes.qux")?];
        let detected = detect_languages(&paths);
        assert_eq!(detected, [DetectedLanguage::Other].into_iter().collect());
        Ok(())
    }

    #[test]
    fn extensionless_file_with_no_manifest_match_detects_nothing(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let paths = vec![rel("NOTICE")?];
        let detected = detect_languages(&paths);
        assert!(detected.is_empty());
        Ok(())
    }

    #[test]
    fn recognized_but_unmapped_extension_detects_other() -> Result<(), Box<dyn std::error::Error>> {
        // `.rb` is in the arc-13 registry but has no dedicated
        // `enforcer-lang-*` pack.
        let paths = vec![rel("script.rb")?];
        let detected = detect_languages(&paths);
        assert_eq!(detected, [DetectedLanguage::Other].into_iter().collect());
        Ok(())
    }
}
