//! MCP build/artifact fingerprint (a02).
//!
//! # Repoint from the legacy `.mjs` fingerprint
//! The legacy `mcp/rust-rules-mcp-fingerprint.mjs` hashed a hardcoded list
//! of ~10 `.mjs` SOURCE paths (`MCP_FINGERPRINT_FILES`) to power the
//! restart/freshness signal for a Node process that loaded those files at
//! startup. In the Rust engine there is no such list: the "MCP server" IS
//! the compiled `enforcer` binary running on stdio ([`crate`] docs, "One
//! binary IS the engine"). A frozen list of `.mjs` paths would describe
//! neither what is executing nor what changed, so this module fingerprints
//! the BUILT ARTIFACT instead:
//!
//! - the running binary's own bytes, read from an explicit location
//!   (typically [`std::env::current_exe`]'s result);
//! - the crate's compile-time package version (the workpack's "fold in the
//!   build fingerprint so a rebuilt-but-unmoved binary is still
//!   detectable" item);
//! - an optional ruleset artifact (content digest over the shipped rule
//!   pack the caller names), folded in only when the caller supplies a
//!   location — never fabricated.
//!
//! The folded digest is a branded [`Sha256`] (a05's brand from
//! `enforcer-domain`), per the workpack's "`build_mcp_fingerprint` returns
//! a `Sha256`" requirement.
//!
//! # Missing/unresolvable artifacts never silently pass
//! [`ArtifactEntry::of_file`] never panics and never treats a missing
//! location as "unchanged": a nonexistent or unreadable artifact yields an
//! explicit [`ArtifactState::Missing`] (the typed replacement for the
//! legacy `exists: false` entry), and that state still participates in the
//! digest, so a missing artifact is observably different from a present
//! one rather than silently ignored.
//!
//! # Staleness: running vs on-disk
//! [`McpFingerprint::compare_to_current`] re-reads the same artifact
//! location(s) captured at startup and compares the freshly computed
//! digest against the captured one, mirroring the legacy
//! `changedFingerprintFiles` staleness concept but over the artifact
//! (binary + version + ruleset) rather than a source-file list.
//!
//! # Scope note (this file's charter)
//! This module OWNS the fingerprint computation and staleness comparison
//! only. Wiring the result into [`crate::gate::Freshness`] (the write-gate
//! predicate) or into the `ocentra_enforcer_mcp_status` router handler is
//! a sibling pack's job — `gate.rs`'s own "a02 seam" module doc already
//! states it only consumes an already-resolved `Freshness`, never
//! computing one itself.

use enforcer_domain::hashes::Sha256;
use std::path::Path;

/// Branded artifact location, captured in forward-slash form.
///
/// SERIALIZATION-DOC: `#[serde(transparent)]` — serializes as the inner
/// forward-slash string (a plain JSON string), matching the legacy
/// fingerprint entry's forward-slash label shape on the wire.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
#[doc = "Branded artifact location; see the serialization note above."]
pub struct ArtifactPath {
    // BRAND-INVARIANT: always the forward-slash rendering of the
    // constructing `&Path` (see `from_path`); may be absolute (the
    // running binary) or caller-relative (a named ruleset), so the
    // relative-only `enforcer_domain::paths::RelPath` brand deliberately
    // does not apply. Only ever rendered, or re-viewed via `as_path`.
    rendered: String,
}

impl ArtifactPath {
    /// Capture `path` in the branded forward-slash form. Total: every
    /// `&Path` has exactly one such rendering, so no `try_` constructor
    /// is needed — there is no rejectable input.
    fn from_path(path: &Path) -> Self {
        let rendered = path.to_string_lossy().replace('\\', "/");
        Self { rendered }
    }

    /// Re-view the captured location as a filesystem path (forward
    /// slashes are valid separators on every supported platform).
    fn as_path(&self) -> &Path {
        Path::new(&self.rendered)
    }
}

/// Branded compile-time package version of this crate's build.
///
/// SERIALIZATION-DOC: `#[serde(transparent)]` — serializes as the plain
/// semver string (a JSON string), matching the legacy `packageVersion`
/// field shape.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
#[doc = "Branded build version; see the serialization note above."]
pub struct PackageVersion {
    // BRAND-INVARIANT: exactly the compile-time `CARGO_PKG_VERSION` this
    // crate was built as (see `current`); display/digest input only,
    // never re-interpreted as a version range or requirement.
    semver: String,
}

impl PackageVersion {
    /// The version this crate was compiled as (`CARGO_PKG_VERSION`).
    pub fn current() -> Self {
        // ALLOC-JUSTIFICATION: `env!` yields a `&'static str`; one owned
        // copy is taken so the brand can hold and serialize the value
        // without a lifetime parameter on every fingerprint struct.
        let semver = env!("CARGO_PKG_VERSION").to_owned();
        Self { semver }
    }
}

/// Branded on-disk artifact size observed at read time.
///
/// SERIALIZATION-DOC: `#[serde(transparent)]` — serializes as a plain
/// JSON number, matching the legacy `byteLength` field shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
#[doc = "Branded observed artifact size; see the serialization note above."]
pub struct ByteCount {
    // BRAND-INVARIANT: the exact byte length of the artifact content
    // that was hashed; zero only co-occurs with a genuinely empty file
    // (a missing artifact carries no ByteCount at all — see
    // `ArtifactState::Missing`).
    observed: u64,
}

/// Present-with-digest or explicitly missing: the typed replacement for
/// the legacy `exists: true/false` + nullable `sha256` field pair, so a
/// missing artifact is structurally unmistakable for a hashed one.
///
/// SERIALIZATION-DOC: internally tagged (`{"kind": "present", ...}` /
/// `{"kind": "missing"}`), camelCase, keeping the two states distinct on
/// the wire for `mcp_status` consumers.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[doc = "Observed artifact state; see the serialization note above."]
pub enum ArtifactState {
    /// The artifact existed and its content was hashed.
    #[serde(rename_all = "camelCase")]
    Present {
        /// Content digest of the artifact bytes.
        sha256: Sha256,
        /// On-disk size observed when the content was read.
        byte_length: ByteCount,
    },
    /// The artifact was missing or unreadable — an explicit signal
    /// surfaced to `mcp_status`, never a silent pass.
    Missing,
}

/// One artifact's fingerprint entry: where it was read from plus its
/// observed state.
///
/// SERIALIZATION-DOC: serializes as a camelCase object `{path, state}`;
/// see each field type's own serialization contract.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[doc = "One artifact's fingerprint entry; see the serialization note above."]
pub struct ArtifactEntry {
    /// Where the artifact was read from.
    pub path: ArtifactPath,
    /// Present-with-digest or explicitly missing.
    pub state: ArtifactState,
}

impl ArtifactEntry {
    /// Fingerprint the file at `path`. A missing or unreadable artifact
    /// yields an explicit [`ArtifactState::Missing`] — never a panic and
    /// never a silent pass.
    pub fn of_file(path: &Path) -> Self {
        let state = match std::fs::read(path) {
            Ok(bytes) => {
                // CAST-JUSTIFICATION: usize -> u64 is lossless on every
                // supported platform (usize is at most 64 bits wide).
                let observed = bytes.len() as u64;
                let byte_length = ByteCount { observed };
                ArtifactState::Present {
                    sha256: Sha256::of(&bytes),
                    byte_length,
                }
            }
            Err(_) => ArtifactState::Missing,
        };
        Self {
            path: ArtifactPath::from_path(path),
            state,
        }
    }
}

impl std::fmt::Display for ArtifactEntry {
    /// Digest-preimage rendering: `<path> present <sha256> <bytes>` for a
    /// hashed artifact, `<path> missing` otherwise. This rendering IS the
    /// digest input contract — changing it changes every folded digest.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.state {
            ArtifactState::Present {
                sha256,
                byte_length,
            } => {
                write!(
                    f,
                    "{} present {} {}",
                    self.path.rendered, sha256, byte_length.observed
                )
            }
            ArtifactState::Missing => write!(f, "{} missing", self.path.rendered),
        }
    }
}

/// The full MCP build/artifact fingerprint: the running binary, the
/// crate's build version, and an optional ruleset artifact, folded into
/// one branded [`Sha256`] digest.
///
/// SERIALIZATION-DOC: serializes as a camelCase object `{digest,
/// packageVersion, binary, ruleset}`; `ruleset: null` means "not
/// tracked", while a tracked-but-absent ruleset serializes as an entry in
/// the `missing` state (the distinction is deliberate and load-bearing).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[doc = "The full MCP fingerprint; see the serialization note above."]
pub struct McpFingerprint {
    /// Folded digest over `binary` + `package_version` + `ruleset`; it
    /// changes iff one of those inputs changes, and unrelated
    /// source-adjacent files can never perturb it because they are never
    /// part of the preimage.
    pub digest: Sha256,
    /// Build version folded into the digest.
    pub package_version: PackageVersion,
    /// The fingerprinted binary/build artifact.
    pub binary: ArtifactEntry,
    /// The fingerprinted ruleset artifact, when the caller tracks one.
    pub ruleset: Option<ArtifactEntry>,
}

/// Build a fingerprint over an explicit binary location, package version,
/// and optional ruleset location. This is the seam
/// [`build_running_mcp_fingerprint`] calls with the real
/// `current_exe()`; exposed directly so tests (and any caller that
/// already knows the artifact location) need no running process.
pub fn build_mcp_fingerprint(
    binary_path: &Path,
    package_version: PackageVersion,
    ruleset_path: Option<&Path>,
) -> McpFingerprint {
    let binary = ArtifactEntry::of_file(binary_path);
    let ruleset = ruleset_path.map(ArtifactEntry::of_file);
    let digest = fold_digest(&binary, &package_version, ruleset.as_ref());
    McpFingerprint {
        digest,
        package_version,
        binary,
        ruleset,
    }
}

/// Typed failure surface for [`build_running_mcp_fingerprint`]. A missing
/// ARTIFACT is not an error (it is [`ArtifactState::Missing`]); this enum
/// covers only the rarer case where the OS cannot even name the running
/// executable, which must surface loudly rather than substitute a guess.
#[derive(Debug, thiserror::Error)]
#[doc = "Typed fingerprint failure surface; see the note above."]
pub enum FingerprintError {
    /// `std::env::current_exe()` could not resolve the running
    /// executable's own location.
    #[error("could not resolve the running executable path: {source}")]
    CurrentExeUnresolvable {
        /// Underlying OS failure, preserved as the error source chain.
        #[source]
        source: std::io::Error,
    },
}

/// Build a fingerprint over the ACTUAL running process's own executable
/// (`std::env::current_exe()`) and this crate's compile-time version —
/// the real-artifact entry point the workpack names.
pub fn build_running_mcp_fingerprint(
    ruleset_path: Option<&Path>,
) -> Result<McpFingerprint, FingerprintError> {
    let exe = std::env::current_exe()
        .map_err(|source| FingerprintError::CurrentExeUnresolvable { source })?;
    Ok(build_mcp_fingerprint(
        &exe,
        PackageVersion::current(),
        ruleset_path,
    ))
}

/// Fold the three fingerprint inputs into one branded digest. The
/// preimage is a fixed, versioned, newline-separated text layout (see
/// [`ArtifactEntry`]'s `Display` contract), so the digest is reproducible
/// across runs and platforms for the same logical inputs.
fn fold_digest(
    binary: &ArtifactEntry,
    package_version: &PackageVersion,
    ruleset: Option<&ArtifactEntry>,
) -> Sha256 {
    let ruleset_render = match ruleset {
        // ALLOC-JUSTIFICATION: the Display rendering is a digest-preimage
        // component; an owned copy is required to splice it into the
        // preimage text below.
        Some(entry) => entry.to_string(),
        None => String::from("untracked"),
    };
    let preimage = format!(
        "enforcer-mcp-fingerprint-v1\nbinary={}\nversion={}\nruleset={}",
        binary, package_version.semver, ruleset_render,
    );
    Sha256::of(preimage.as_bytes())
}

/// Which fingerprint slot a staleness diff refers to.
///
/// SERIALIZATION-DOC: serializes as a plain camelCase string
/// (`"binary"` / `"ruleset"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[doc = "SERDE-TAG-JUSTIFICATION: unit-only enum; serializes as a plain camelCase string, and a tag would wrap it in a needless single-key object."]
pub enum ArtifactSlot {
    /// The running binary artifact changed.
    Binary,
    /// The tracked ruleset artifact changed.
    Ruleset,
}

/// One artifact whose observation differs between the startup snapshot
/// and a freshly recomputed current one.
///
/// SERIALIZATION-DOC: serializes as a camelCase object `{slot, startup,
/// current}` with full before/after entries for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[doc = "One changed artifact slot; see the serialization note above."]
pub struct ChangedArtifact {
    /// Which slot changed.
    pub slot: ArtifactSlot,
    /// The slot's entry as captured at startup (`None` = untracked then).
    pub startup: Option<ArtifactEntry>,
    /// The slot's entry as observed now (`None` = untracked now).
    pub current: Option<ArtifactEntry>,
}

/// Fresh-or-stale verdict, as an explicit two-state enum rather than a
/// raw bool so call sites read as domain language.
///
/// SERIALIZATION-DOC: serializes as a plain camelCase string
/// (`"fresh"` / `"stale"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[doc = "SERDE-TAG-JUSTIFICATION: unit-only enum; serializes as a plain camelCase string, and a tag would wrap it in a needless single-key object."]
pub enum Staleness {
    /// Running and on-disk fingerprints agree.
    Fresh,
    /// The on-disk artifact set no longer matches what is running.
    Stale,
}

/// The staleness verdict from comparing a startup-time fingerprint
/// against a freshly recomputed one over the SAME artifact locations —
/// the "running vs on-disk" comparison the workpack requires.
///
/// SERIALIZATION-DOC: serializes as a camelCase object `{verdict,
/// startupDigest, currentDigest, changed}`; `changed` is empty iff the
/// verdict is fresh.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[doc = "The running-vs-on-disk staleness verdict; see the note above."]
pub struct StalenessReport {
    /// Fresh iff the folded digests agree.
    pub verdict: Staleness,
    /// Digest captured at startup.
    pub startup_digest: Sha256,
    /// Digest recomputed from disk just now.
    pub current_digest: Sha256,
    /// Which specific slot(s) changed; empty when fresh.
    pub changed: Vec<ChangedArtifact>,
}

impl McpFingerprint {
    /// Re-read the same binary/ruleset location(s) this fingerprint was
    /// built from and produce a fresh "current" fingerprint. The version
    /// is carried over from the startup snapshot: it is a compile-time
    /// constant of the running process, not an on-disk observable, so a
    /// rebuilt binary at the same location is detected via its changed
    /// byte content (which IS re-read here).
    pub fn recompute(&self) -> Self {
        let ruleset_path = self.ruleset.as_ref().map(|entry| entry.path.as_path());
        build_mcp_fingerprint(
            self.binary.path.as_path(),
            // CLONE-JUSTIFICATION: the recomputed snapshot must carry the
            // startup snapshot's own version brand (see doc above); the
            // startup value stays alive for the comparison, so a copy is
            // required.
            self.package_version.clone(),
            ruleset_path,
        )
    }

    /// Recompute against disk and compare to `self` (the startup
    /// snapshot). Never mutates `self`.
    pub fn compare_to_current(&self) -> StalenessReport {
        let current = self.recompute();
        compare_freshness(self, &current)
    }
}

/// Compare two fingerprints (typically a startup snapshot and a freshly
/// recomputed current one) and report what changed. Free function
/// alongside the [`McpFingerprint::compare_to_current`] convenience so a
/// caller holding two independently built fingerprints (e.g. a live P3
/// MCP-tool test that swaps the binary out-of-process) can compare them
/// directly.
pub fn compare_freshness(startup: &McpFingerprint, current: &McpFingerprint) -> StalenessReport {
    let verdict = if startup.digest == current.digest {
        Staleness::Fresh
    } else {
        Staleness::Stale
    };
    StalenessReport {
        verdict,
        // CLONE-JUSTIFICATION: the report owns its digests and entry
        // snapshots so it can be serialized and outlive the two
        // fingerprints it was derived from (both stay borrowed here).
        startup_digest: startup.digest.clone(),
        current_digest: current.digest.clone(),
        changed: changed_slots(startup, current),
    }
}

/// Collect the per-slot diffs between two fingerprints (helper for
/// [`compare_freshness`]).
fn changed_slots(startup: &McpFingerprint, current: &McpFingerprint) -> Vec<ChangedArtifact> {
    let mut changed = Vec::new();
    if startup.binary != current.binary {
        changed.push(ChangedArtifact {
            slot: ArtifactSlot::Binary,
            // CLONE-JUSTIFICATION: the diff owns before/after snapshots
            // for serialization; the compared fingerprints stay borrowed.
            startup: Some(startup.binary.clone()),
            current: Some(current.binary.clone()),
        });
    }
    if startup.ruleset != current.ruleset {
        changed.push(ChangedArtifact {
            slot: ArtifactSlot::Ruleset,
            // CLONE-JUSTIFICATION: same owned-snapshot rationale as the
            // binary slot above.
            startup: startup.ruleset.clone(),
            current: current.ruleset.clone(),
        });
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::{
        build_mcp_fingerprint, build_running_mcp_fingerprint, compare_freshness, ArtifactEntry,
        ArtifactPath, ArtifactSlot, ArtifactState, ByteCount, ChangedArtifact, FingerprintError,
        PackageVersion, Staleness,
    };
    use enforcer_domain::hashes::Sha256;
    use std::path::Path;

    fn version_zero_one() -> PackageVersion {
        let semver = String::from("0.1.0");
        PackageVersion { semver }
    }

    fn version_zero_two() -> PackageVersion {
        let semver = String::from("0.2.0");
        PackageVersion { semver }
    }

    #[test]
    fn missing_artifact_reports_explicit_missing_state() {
        let absent = Path::new("/definitely/does/not/exist/enforcer-binary");
        let entry = ArtifactEntry::of_file(absent);
        assert_eq!(
            entry.state,
            ArtifactState::Missing,
            "a missing artifact must be an explicit Missing state, never a silent pass"
        );
    }

    #[test]
    fn present_artifact_hashes_its_real_bytes() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"binary-bytes-v1")?;
        let entry = ArtifactEntry::of_file(&artifact);
        assert_eq!(
            entry.state,
            ArtifactState::Present {
                sha256: Sha256::of(b"binary-bytes-v1"),
                byte_length: ByteCount { observed: 15 },
            },
        );
        Ok(())
    }

    #[test]
    fn digest_is_a_wellformed_sha256_and_deterministic() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"artifact-content")?;
        let first = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        let second = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        assert_eq!(first.digest.hex().len(), 64);
        assert!(first.digest.hex().chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(
            first.digest, second.digest,
            "identical inputs must fold to the identical digest"
        );
        Ok(())
    }

    #[test]
    fn digest_changes_when_binary_bytes_change() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"v1-bytes")?;
        let before = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        // Simulate a rebuild replacing the artifact bytes at the same
        // location.
        std::fs::write(&artifact, b"v2-bytes-different")?;
        let after = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        assert_ne!(
            before.digest, after.digest,
            "changed artifact bytes must change the folded digest"
        );
        Ok(())
    }

    #[test]
    fn digest_changes_when_version_changes_even_with_identical_bytes() -> Result<(), std::io::Error>
    {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"same-bytes")?;
        let v1 = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        let v2 = build_mcp_fingerprint(&artifact, version_zero_two(), None);
        assert_ne!(
            v1.digest, v2.digest,
            "a rebuilt-but-byte-identical binary under a new version must still be detectable"
        );
        Ok(())
    }

    #[test]
    fn unrelated_source_adjacent_files_never_perturb_the_digest() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"stable-binary-bytes")?;
        let before = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        // An unrelated file appears next to the binary; it must never be
        // part of the fingerprint preimage.
        std::fs::write(dir.path().join("unrelated.txt"), b"noise")?;
        let after = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        assert_eq!(
            before.digest, after.digest,
            "unrelated source-adjacent files must not perturb the artifact fingerprint"
        );
        Ok(())
    }

    #[test]
    fn ruleset_tracking_and_content_both_fold_into_the_digest() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"binary-bytes")?;
        let ruleset = dir.path().join("ruleset.json");
        std::fs::write(&ruleset, b"{\"rules\":1}")?;

        let untracked = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        let tracked = build_mcp_fingerprint(&artifact, version_zero_one(), Some(&ruleset));
        assert_ne!(
            untracked.digest, tracked.digest,
            "tracking a ruleset at all must change the digest vs not tracking one"
        );

        std::fs::write(&ruleset, b"{\"rules\":2}")?;
        let tracked_changed = build_mcp_fingerprint(&artifact, version_zero_one(), Some(&ruleset));
        assert_ne!(
            tracked.digest, tracked_changed.digest,
            "a changed ruleset artifact must change the digest"
        );
        Ok(())
    }

    #[test]
    fn staleness_report_is_quiet_when_nothing_changed_on_disk() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"unchanged-bytes")?;
        let startup = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        let report = startup.compare_to_current();
        assert_eq!(report.verdict, Staleness::Fresh);
        assert!(report.changed.is_empty());
        assert_eq!(report.startup_digest, report.current_digest);
        Ok(())
    }

    #[test]
    fn staleness_report_detects_a_binary_replaced_after_startup() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"running-version-bytes")?;
        let startup = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        // The process still holds `startup` in memory, but the on-disk
        // binary gets replaced by a rebuild.
        std::fs::write(&artifact, b"freshly-rebuilt-bytes")?;
        let report = startup.compare_to_current();
        assert_eq!(
            report.verdict,
            Staleness::Stale,
            "a replaced on-disk binary must be reported stale"
        );
        assert_eq!(report.changed.len(), 1);
        assert!(matches!(
            report.changed.first(),
            Some(ChangedArtifact {
                slot: ArtifactSlot::Binary,
                ..
            })
        ));
        assert_ne!(report.startup_digest, report.current_digest);
        Ok(())
    }

    #[test]
    fn staleness_report_detects_the_artifact_vanishing() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"present-at-startup")?;
        let startup = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        std::fs::remove_file(&artifact)?;
        let report = startup.compare_to_current();
        assert_eq!(
            report.verdict,
            Staleness::Stale,
            "a vanished artifact must be reported stale"
        );
        let current_is_missing = report
            .changed
            .first()
            .and_then(|change| change.current.as_ref())
            .map(|entry| entry.state == ArtifactState::Missing);
        assert_eq!(
            current_is_missing,
            Some(true),
            "the diff must show the current observation as explicitly missing"
        );
        Ok(())
    }

    #[test]
    fn compare_freshness_free_function_matches_the_method() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"x")?;
        let a = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        let b = build_mcp_fingerprint(&artifact, version_zero_one(), None);
        let via_method = a.compare_to_current();
        let via_function = compare_freshness(&a, &b);
        assert_eq!(via_method.verdict, via_function.verdict);
        assert_eq!(via_method.changed.len(), via_function.changed.len());
        Ok(())
    }

    /// Sanity check against the REAL running artifact: `current_exe()`
    /// resolves to the test binary itself, which always exists and is
    /// readable, proving the production entry point end-to-end rather
    /// than only the location-parameterized seam above.
    #[test]
    fn running_fingerprint_resolves_the_real_test_binary() -> Result<(), FingerprintError> {
        let fingerprint = build_running_mcp_fingerprint(None)?;
        assert!(matches!(
            fingerprint.binary.state,
            ArtifactState::Present { .. }
        ));
        assert!(!fingerprint.package_version.semver.is_empty());
        assert_eq!(fingerprint.digest.hex().len(), 64);
        Ok(())
    }

    /// PROPERTY-TEST: over a generated grid of separator/segment
    /// combinations, `ArtifactPath::from_path` (the brand's constructor)
    /// upholds two properties: (1) the captured form never contains a
    /// backslash, and (2) re-viewing via `as_path` and re-capturing is
    /// idempotent (`from_path(as_path(x)) == x`).
    #[test]
    fn artifact_path_capture_properties_hold_across_generated_inputs() {
        let segments = ["crates", "enforcer-mcp", "with space", "üñí-code", "a.b"];
        let separators = ["/", "\\"];
        for first in segments {
            for second in segments {
                for sep in separators {
                    let raw = format!("{first}{sep}{second}");
                    let captured = ArtifactPath::from_path(Path::new(&raw));
                    assert!(
                        !captured.rendered.contains('\\'),
                        "captured form must be forward-slash only for {raw:?}"
                    );
                    let recaptured = ArtifactPath::from_path(captured.as_path());
                    assert_eq!(
                        captured, recaptured,
                        "capture must be idempotent for {raw:?}"
                    );
                }
            }
        }
    }
}
