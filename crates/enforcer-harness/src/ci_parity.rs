//! d11 — CI parity validator: proves the local dev-loop step set and the
//! CI workflow job's step set are the same set, and that pinned tool/
//! toolchain versions agree between the two sources of truth.
//!
//! # Where this fits
//!
//! ADBP's "local == CI" is a guideline, not a check, until this module
//! exists. `enforcer-harness` (arc-18) already runs native tools but had
//! no module proving the LOCAL steps (the installer-emitted pre-commit /
//! cargo-alias steps a contributor runs before pushing) and the CI
//! workflow's steps are the same set, with the same pinned versions. This
//! module is that fail-closed diff, callable identically from a local
//! `enforcer` check and from a CI job (self-referential parity — CI runs
//! the same function against the same two files a contributor's machine
//! would).
//!
//! # Manifests, not files
//!
//! [`parse_local_manifest`] and [`parse_ci_manifest`] both take raw text
//! (never a path) and return a normalized [`StepManifest`] — the actual
//! file I/O lives at the call site (a harness run-adapter, a CLI command,
//! or a test). This keeps the diff itself pure and lets tests inject
//! drift (an extra local-only step, a version skew) via fixture text
//! without touching real repo files.
//!
//! - [`parse_local_manifest`] reads the small JSON array this repo's own
//!   local step declaration uses (`[{"name": "...", "version": "..."}]`,
//!   `version` optional) — the mechanical, greppable source of truth for
//!   "what a contributor runs locally before pushing."
//! - [`parse_ci_manifest`] reads a GitHub Actions workflow YAML's
//!   `steps:` list with a minimal, purpose-built line scanner (no general
//!   YAML semantics: this workspace carries no YAML crate, and the
//!   surface this module needs — `- name: "..."` step names plus
//!   `uses: owner/action@ref` / inline pinned-version hints — is narrow
//!   enough that hand-parsing it is more auditable than pulling in a full
//!   YAML parser for one file shape).
//!
//! # Fail-closed diff
//!
//! [`check_parity`] is the whole check: normalized step-NAME set equality
//! (any local-only or CI-only step is a [`Finding`] naming that exact
//! step) AND, independently, pinned-version agreement for every component
//! name both manifests declare a version for (a mismatch names the
//! component, the local value, and the CI value). An empty return means
//! full parity — never a boolean, matching the [`crate`]-wide "return
//! findings, don't `println!`/exit" contract (workspace lint policy: no
//! `unwrap`/`expect`/`panic`/`print_*`).

use std::collections::{BTreeMap, BTreeSet};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;

/// Synthetic rule id this check's findings carry.
pub const CI_PARITY_RULE_ID: &str = "CI-PARITY.1";

/// One step (local or CI), after normalization: a name and an optional
/// pinned version. Two steps with the same `name` but different
/// `version` are a version-skew finding, not a missing-step finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StepRecord {
    /// Normalized step name (trimmed; the comparison key).
    pub name: String,
    /// Pinned version string for this step's tool/action/toolchain, if
    /// the manifest declared one.
    pub version: Option<String>,
}

/// A normalized set of steps parsed from one source (local or CI).
/// Deliberately a `Vec` (not a `BTreeSet`) at this layer so a manifest
/// with an accidental duplicate step name is itself observable (the
/// duplicate collapses only when [`check_parity`] builds its comparison
/// sets); order is preserved from the source text but not load-bearing
/// for the diff.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StepManifest {
    pub steps: Vec<StepRecord>,
}

impl StepManifest {
    fn name_set(&self) -> BTreeSet<&str> {
        self.steps.iter().map(|s| s.name.as_str()).collect()
    }

    /// First declared version per step name (a manifest declaring the
    /// same name twice with different versions is a manifest-authoring
    /// bug outside this check's scope; the first occurrence wins
    /// deterministically).
    fn version_map(&self) -> BTreeMap<&str, &str> {
        let mut map = BTreeMap::new();
        for step in &self.steps {
            if let Some(version) = step.version.as_deref() {
                map.entry(step.name.as_str()).or_insert(version);
            }
        }
        map
    }
}

/// Parse the local step manifest: a JSON array of
/// `{"name": "...", "version": "..." }` objects (`version` optional).
/// This is the installer-emitted / cargo-alias local declaration's
/// mechanical shape — the exact file this reads from is a call-site
/// concern (see module docs); this function only ever sees text.
///
/// # Errors
/// Returns a `String` describing the JSON error the moment the text does
/// not parse as the expected array-of-objects shape. Never panics.
pub fn parse_local_manifest(text: &str) -> Result<StepManifest, String> {
    #[derive(serde::Deserialize)]
    struct RawStep {
        name: String,
        #[serde(default)]
        version: Option<String>,
    }
    let raw: Vec<RawStep> = serde_json::from_str(text)
        .map_err(|source| format!("local manifest is not a valid JSON step array: {source}"))?;
    Ok(StepManifest {
        steps: raw
            .into_iter()
            .map(|r| StepRecord {
                name: r.name.trim().to_owned(),
                version: r.version.map(|v| v.trim().to_owned()),
            })
            .collect(),
    })
}

/// Parse a GitHub Actions workflow's `steps:` list out of raw YAML text.
///
/// Recognizes, per step block (a line starting with `- name:` opens a new
/// step; parsing continues until the next `- name:` line at the same
/// indent or the block ends):
/// - `name:` -> [`StepRecord::name`] (quotes stripped).
/// - `uses: owner/action@ref` -> version is the `@ref` suffix.
/// - `run: ... some-tool@X.Y.Z ...` or `... --version X.Y.Z` shaped
///   pins are NOT inferred (too ambiguous to parse safely); a CI step
///   that pins a tool version only inside a free-form `run:` shell
///   command must additionally declare it via a companion `# ci-parity:
///   <tool>=<version>` comment line for this parser to see it — this
///   keeps the extraction mechanical (comment grep) rather than a shell
///   command interpreter.
///
/// Never panics on malformed/partial YAML: a step block missing a `name:`
/// line is simply skipped (it cannot be compared by name), and stray text
/// outside any `steps:` block is ignored.
#[must_use]
pub fn parse_ci_manifest(text: &str) -> StepManifest {
    let mut steps = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_version: Option<String> = None;

    let flush =
        |steps: &mut Vec<StepRecord>, name: &mut Option<String>, version: &mut Option<String>| {
            if let Some(name) = name.take() {
                steps.push(StepRecord {
                    name,
                    version: version.take(),
                });
            } else {
                *version = None;
            }
        };

    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim_start();

        if let Some(rest) = trimmed.strip_prefix("- name:") {
            flush(&mut steps, &mut current_name, &mut current_version);
            current_name = Some(unquote(rest.trim()));
            continue;
        }

        if current_name.is_none() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("uses:") {
            let uses = unquote(rest.trim());
            if let Some((_, ref_part)) = uses.rsplit_once('@') {
                current_version = Some(ref_part.to_owned());
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("# ci-parity:") {
            if let Some((tool, version)) = rest.trim().split_once('=') {
                if tool.trim() == current_name.as_deref().unwrap_or_default()
                    || current_version.is_none()
                {
                    current_version = Some(version.trim().to_owned());
                }
            }
            continue;
        }
    }
    flush(&mut steps, &mut current_name, &mut current_version);

    StepManifest { steps }
}

/// Strip one layer of matching leading/trailing `'` or `"` quotes, if
/// present. Pure string trimming — not a YAML scalar decoder.
fn unquote(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let (Some(first), Some(last)) = (bytes.first(), bytes.last()) else {
        return raw.to_owned();
    };
    if bytes.len() >= 2
        && ((*first == b'"' && *last == b'"') || (*first == b'\'' && *last == b'\''))
    {
        return raw
            .get(1..bytes.len() - 1)
            .map_or_else(|| raw.to_owned(), str::to_owned);
    }
    raw.to_owned()
}

/// Run the fail-closed diff between `local` and `ci`. An empty result
/// means full parity. Every mismatch names the specific step or
/// component — never a bare "manifests differ."
#[must_use]
pub fn check_parity(local: &StepManifest, ci: &StepManifest) -> Vec<Finding> {
    let mut findings = Vec::new();

    let local_names = local.name_set();
    let ci_names = ci.name_set();

    for local_only in local_names.difference(&ci_names) {
        push_step_finding(
            &mut findings,
            "local-only step has no matching CI step",
            &format!(
                "step `{local_only}` runs locally but is not present in the CI workflow's step set"
            ),
        );
    }
    for ci_only in ci_names.difference(&local_names) {
        push_step_finding(
            &mut findings,
            "CI-only step has no matching local step",
            &format!("step `{ci_only}` runs in CI but is not present in the local step manifest"),
        );
    }

    let local_versions = local.version_map();
    let ci_versions = ci.version_map();
    for (component, local_version) in &local_versions {
        if let Some(ci_version) = ci_versions.get(component) {
            if local_version != ci_version {
                push_step_finding(
                    &mut findings,
                    "pinned version skew between local and CI",
                    &format!(
                        "component `{component}` is pinned to `{local_version}` locally but `{ci_version}` in CI"
                    ),
                );
            }
        }
    }

    findings
}

/// Compare the [`rust-toolchain.toml`] `channel` value both sources
/// expect. `local_channel`/`ci_channel` are each the caller-extracted
/// channel string (see [`extract_toolchain_channel`]); `None` means that
/// source made no explicit toolchain-channel claim (not itself a
/// mismatch — a source that does not pin a channel cannot skew from one
/// that does, it simply defers to whatever `rustup` resolves).
#[must_use]
pub fn check_toolchain_channel_parity(
    local_channel: Option<&str>,
    ci_channel: Option<&str>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let (Some(local), Some(ci)) = (local_channel, ci_channel) {
        if local != ci {
            push_step_finding(
                &mut findings,
                "pinned toolchain channel skew",
                &format!(
                    "`rust-toolchain.toml` channel is `{local}` but the CI-observed channel is `{ci}`"
                ),
            );
        }
    }
    findings
}

/// Extract the `channel = "..."` value from a `rust-toolchain.toml`-shaped
/// TOML text's `[toolchain]` table. Returns `None` (not an error) when no
/// such key is present — this is a best-effort extraction, not a full
/// TOML schema validator; malformed TOML also yields `None` rather than
/// panicking.
#[must_use]
pub fn extract_toolchain_channel(toolchain_toml_text: &str) -> Option<String> {
    let doc = toolchain_toml_text.parse::<toml_edit::DocumentMut>().ok()?;
    doc.get("toolchain")?
        .get("channel")?
        .as_str()
        .map(str::to_owned)
}

/// Build one [`Finding`] and push it onto `findings` — but only if both
/// branded fields ([`RuleId`], [`RelPath`]) actually construct from their
/// fixed literals. [`CI_PARITY_RULE_ID`] and the diagnostic path literal
/// are structurally guaranteed valid under their respective
/// `TryFrom<String>` charsets (see `crates/enforcer-domain/src/ids.rs` and
/// `.../paths.rs`) — covered by [`tests::ci_parity_rule_id_and_path_literals_are_valid`]
/// — so the `Err` arm below is unreachable in practice. It is still a
/// real (non-panicking) branch rather than `.unwrap()`/`.expect()`: this
/// workspace denies both lints workspace-wide with no `#[allow]` escape
/// hatch, so a silently-dropped finding on a theoretical future charset
/// change is the honest total-function alternative to a panic.
fn push_step_finding(findings: &mut Vec<Finding>, title: &str, detail: &str) {
    let rule_id: Result<RuleId, _> = CI_PARITY_RULE_ID.parse();
    let file: Result<RelPath, _> = "ci-parity/local-vs-ci-manifest".parse();
    if let (Ok(rule_id), Ok(file)) = (rule_id, file) {
        findings.push(Finding {
            rule_id,
            severity: Severity::Error,
            title: title.to_owned(),
            detail: detail.to_owned(),
            file,
            line: 1,
            snippet: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{
        check_parity, check_toolchain_channel_parity, extract_toolchain_channel, parse_ci_manifest,
        parse_local_manifest, StepManifest, StepRecord, CI_PARITY_RULE_ID,
    };
    use enforcer_domain::ids::RuleId;
    use enforcer_domain::paths::RelPath;

    /// Backs the "unreachable in practice" claim in `push_step_finding`'s
    /// doc comment: both fixed literals it parses actually construct
    /// successfully today. If a future charset change to `RuleId`/
    /// `RelPath` ever breaks either, this test (not a silent
    /// dropped-finding at runtime) is what catches it.
    #[test]
    fn ci_parity_rule_id_and_path_literals_are_valid() {
        assert!(CI_PARITY_RULE_ID.parse::<RuleId>().is_ok());
        assert!("ci-parity/local-vs-ci-manifest".parse::<RelPath>().is_ok());
    }

    #[test]
    fn parses_local_manifest_json_array() -> Result<(), Box<dyn std::error::Error>> {
        let text =
            r#"[{"name": "cargo fmt --check"}, {"name": "cargo-deny", "version": "0.14.0"}]"#;
        let manifest = parse_local_manifest(text)?;
        assert_eq!(manifest.steps.len(), 2);
        assert_eq!(manifest.steps[0].name, "cargo fmt --check");
        assert_eq!(manifest.steps[1].version.as_deref(), Some("0.14.0"));
        Ok(())
    }

    #[test]
    fn rejects_malformed_local_manifest_text() {
        let result = parse_local_manifest("not json");
        assert!(result.is_err());
        if let Err(detail) = result {
            assert!(detail.contains("not a valid JSON step array"));
        }
    }

    #[test]
    fn parses_ci_step_names_and_uses_pin() {
        let yaml = r#"
jobs:
  rust-ci:
    steps:
      - uses: actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5
      - name: cargo fmt --check
        run: cargo fmt --all --check
      - name: cargo-deny
        uses: some/cargo-deny-action@v1.2.3
"#;
        let manifest = parse_ci_manifest(yaml);
        let names: Vec<&str> = manifest.steps.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["cargo fmt --check", "cargo-deny"]);
        let deny = manifest.steps.iter().find(|s| s.name == "cargo-deny");
        assert_eq!(deny.and_then(|s| s.version.as_deref()), Some("v1.2.3"));
    }

    #[test]
    fn parses_ci_parity_comment_pin() {
        let yaml = r#"
      - name: cargo-audit
        # ci-parity: cargo-audit=0.18.0
        run: cargo install cargo-audit --locked
"#;
        let manifest = parse_ci_manifest(yaml);
        assert_eq!(manifest.steps.len(), 1);
        assert_eq!(manifest.steps[0].version.as_deref(), Some("0.18.0"));
    }

    #[test]
    fn matched_sets_pass_with_zero_findings() {
        let local = StepManifest {
            steps: vec![
                StepRecord {
                    name: "cargo fmt --check".to_owned(),
                    version: None,
                },
                StepRecord {
                    name: "cargo-deny".to_owned(),
                    version: Some("0.14.0".to_owned()),
                },
            ],
        };
        let ci = StepManifest {
            steps: vec![
                StepRecord {
                    name: "cargo fmt --check".to_owned(),
                    version: None,
                },
                StepRecord {
                    name: "cargo-deny".to_owned(),
                    version: Some("0.14.0".to_owned()),
                },
            ],
        };
        assert!(check_parity(&local, &ci).is_empty());
    }

    #[test]
    fn injected_local_only_step_fails_closed() {
        let local = StepManifest {
            steps: vec![
                StepRecord {
                    name: "cargo fmt --check".to_owned(),
                    version: None,
                },
                StepRecord {
                    name: "extra-local-only-step".to_owned(),
                    version: None,
                },
            ],
        };
        let ci = StepManifest {
            steps: vec![StepRecord {
                name: "cargo fmt --check".to_owned(),
                version: None,
            }],
        };
        let findings = check_parity(&local, &ci);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("extra-local-only-step"));
        assert!(findings[0]
            .detail
            .contains("is not present in the CI workflow's step set"));
    }

    #[test]
    fn injected_ci_only_step_fails_closed() {
        let local = StepManifest {
            steps: vec![StepRecord {
                name: "cargo fmt --check".to_owned(),
                version: None,
            }],
        };
        let ci = StepManifest {
            steps: vec![
                StepRecord {
                    name: "cargo fmt --check".to_owned(),
                    version: None,
                },
                StepRecord {
                    name: "extra-ci-only-step".to_owned(),
                    version: None,
                },
            ],
        };
        let findings = check_parity(&local, &ci);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("extra-ci-only-step"));
        assert!(findings[0]
            .detail
            .contains("is not present in the local step manifest"));
    }

    #[test]
    fn injected_version_skew_fails_closed() {
        let local = StepManifest {
            steps: vec![StepRecord {
                name: "cargo-deny".to_owned(),
                version: Some("0.14.0".to_owned()),
            }],
        };
        let ci = StepManifest {
            steps: vec![StepRecord {
                name: "cargo-deny".to_owned(),
                version: Some("0.15.2".to_owned()),
            }],
        };
        let findings = check_parity(&local, &ci);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("0.14.0"));
        assert!(findings[0].detail.contains("0.15.2"));
        assert!(findings[0].title.contains("version skew"));
    }

    #[test]
    fn matching_versions_do_not_double_report_alongside_name_diff() {
        let local = StepManifest {
            steps: vec![StepRecord {
                name: "cargo-deny".to_owned(),
                version: Some("0.14.0".to_owned()),
            }],
        };
        let ci = StepManifest {
            steps: vec![StepRecord {
                name: "cargo-deny".to_owned(),
                version: Some("0.14.0".to_owned()),
            }],
        };
        assert!(check_parity(&local, &ci).is_empty());
    }

    #[test]
    fn extracts_toolchain_channel_from_toml() {
        let toml = "[toolchain]\nchannel = \"1.95.0\"\ncomponents = [\"rustfmt\", \"clippy\"]\n";
        assert_eq!(extract_toolchain_channel(toml), Some("1.95.0".to_owned()));
    }

    #[test]
    fn missing_channel_key_yields_none_not_error() {
        let toml = "[toolchain]\ncomponents = [\"rustfmt\"]\n";
        assert_eq!(extract_toolchain_channel(toml), None);
    }

    #[test]
    fn malformed_toolchain_toml_yields_none_not_panic() {
        assert_eq!(extract_toolchain_channel("not valid [[[ toml"), None);
    }

    #[test]
    fn toolchain_channel_skew_is_flagged() {
        let findings = check_toolchain_channel_parity(Some("1.95.0"), Some("1.80.0"));
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("1.95.0"));
        assert!(findings[0].detail.contains("1.80.0"));
    }

    #[test]
    fn toolchain_channel_agreement_is_clean() {
        assert!(check_toolchain_channel_parity(Some("1.95.0"), Some("1.95.0")).is_empty());
    }

    #[test]
    fn one_sided_toolchain_claim_is_not_a_mismatch() {
        assert!(check_toolchain_channel_parity(Some("1.95.0"), None).is_empty());
        assert!(check_toolchain_channel_parity(None, Some("1.95.0")).is_empty());
    }
}
