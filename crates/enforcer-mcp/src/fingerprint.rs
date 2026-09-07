//! MCP build/artifact fingerprint assembly.
//!
//! Canonical fingerprint value types live in `enforcer-domain`; this module
//! resolves process-local artifact locations and injects this crate's
//! compile-time package version. JSON ingress/egress is isolated in
//! [`crate::boundary::fingerprint`].

use enforcer_domain::mcp_types::{
    FingerprintError, McpFingerprint, McpFreshness, PackageVersion, Staleness, StalenessReport,
};
use std::path::Path;

/// Build a canonical fingerprint from explicit artifact locations.
pub fn build_mcp_fingerprint(
    binary_path: &Path,
    package_version: PackageVersion,
    ruleset_path: Option<&Path>,
) -> McpFingerprint {
    enforcer_domain::mcp_types::build_mcp_fingerprint(binary_path, package_version, ruleset_path)
}

/// Compare two canonical MCP fingerprints.
pub fn compare_freshness(startup: &McpFingerprint, current: &McpFingerprint) -> StalenessReport {
    enforcer_domain::mcp_types::compare_freshness(startup, current)
}

/// Recompute one startup fingerprint and collapse its typed staleness verdict
/// into the MCP routing freshness gate.
pub fn current_mcp_freshness(startup: &McpFingerprint) -> McpFreshness {
    match compare_freshness(startup, &startup.recompute()).verdict {
        Staleness::Fresh => McpFreshness::Fresh,
        Staleness::Stale => McpFreshness::Stale,
    }
}

/// Build a fingerprint over the running executable and this MCP crate's
/// compile-time package version.
pub fn build_running_mcp_fingerprint(
    ruleset_path: Option<&Path>,
) -> Result<McpFingerprint, FingerprintError> {
    let exe = std::env::current_exe()
        .map_err(|source| FingerprintError::CurrentExeUnresolvable { source })?;
    let package_version = PackageVersion::try_new(env!("CARGO_PKG_VERSION")).map_err(|_error| {
        FingerprintError::CurrentExeUnresolvable {
            source: std::io::Error::other("compile-time package version was empty"),
        }
    })?;
    Ok(build_mcp_fingerprint(&exe, package_version, ruleset_path))
}

#[cfg(test)]
mod tests {
    use super::{
        build_mcp_fingerprint, build_running_mcp_fingerprint, compare_freshness, FingerprintError,
        PackageVersion,
    };
    use crate::boundary as transport_boundary;
    use enforcer_domain::mcp_types::{
        ArtifactEntry, ArtifactPath, ArtifactSlot, ArtifactState, ByteCount, ChangedArtifact,
        Staleness,
    };
    use std::path::Path;
    use transport_boundary::fingerprint::{
        decode_fingerprint_json, encode_fingerprint_json, FingerprintWireError,
    };
    use transport_boundary::staleness_report::{
        decode_staleness_report_json, encode_staleness_report_json,
    };

    fn version_zero_one() -> Result<PackageVersion, std::io::Error> {
        PackageVersion::try_new("0.1.0")
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    }

    fn version_zero_two() -> Result<PackageVersion, std::io::Error> {
        PackageVersion::try_new("0.2.0")
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    }

    #[test]
    fn missing_artifact_reports_explicit_missing_state() {
        let absent = Path::new("/definitely/does/not/exist/enforcer-binary");
        let entry = ArtifactEntry::of_file(absent);
        assert_eq!(entry.state, ArtifactState::Missing);
    }

    #[test]
    fn present_artifact_hashes_its_real_bytes() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"binary-bytes-v1")?;
        let entry = ArtifactEntry::of_file(&artifact);
        let expected_byte_count = std::num::NonZeroU64::new(15).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "fixture byte count was zero",
            )
        })?;
        assert_eq!(
            entry.state,
            ArtifactState::Present {
                sha256: enforcer_domain::boundary::hash::validate(b"binary-bytes-v1"),
                byte_length: ByteCount::try_new(expected_byte_count),
            }
        );
        Ok(())
    }

    #[test]
    fn digest_is_a_wellformed_sha256_and_deterministic() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"artifact-content")?;
        let first = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        let second = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        assert_eq!(first.digest.hex().len(), 64);
        assert!(first.digest.hex().chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(first.digest, second.digest);
        Ok(())
    }

    #[test]
    fn digest_changes_when_binary_bytes_change() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"v1-bytes")?;
        let before = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        std::fs::write(&artifact, b"v2-bytes-different")?;
        let after = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        assert_ne!(before.digest, after.digest);
        Ok(())
    }

    #[test]
    fn digest_changes_when_version_changes_even_with_identical_bytes() -> Result<(), std::io::Error>
    {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"same-bytes")?;
        let v1 = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        let v2 = build_mcp_fingerprint(&artifact, version_zero_two()?, None);
        assert_ne!(v1.digest, v2.digest);
        Ok(())
    }

    #[test]
    fn unrelated_source_adjacent_files_never_perturb_the_digest() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"stable-binary-bytes")?;
        let before = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        std::fs::write(dir.path().join("unrelated.txt"), b"noise")?;
        let after = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        assert_eq!(before.digest, after.digest);
        Ok(())
    }

    #[test]
    fn ruleset_tracking_and_content_both_fold_into_the_digest() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        let ruleset = dir.path().join("ruleset.json");
        std::fs::write(&artifact, b"binary-bytes")?;
        std::fs::write(&ruleset, b"{\"rules\":1}")?;
        let untracked = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        let tracked = build_mcp_fingerprint(&artifact, version_zero_one()?, Some(&ruleset));
        assert_ne!(untracked.digest, tracked.digest);
        std::fs::write(&ruleset, b"{\"rules\":2}")?;
        let tracked_changed = build_mcp_fingerprint(&artifact, version_zero_one()?, Some(&ruleset));
        assert_ne!(tracked.digest, tracked_changed.digest);
        Ok(())
    }

    #[test]
    fn staleness_report_is_quiet_when_nothing_changed_on_disk() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"unchanged-bytes")?;
        let startup = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
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
        let startup = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        std::fs::write(&artifact, b"freshly-rebuilt-bytes")?;
        let report = startup.compare_to_current();
        assert_eq!(report.verdict, Staleness::Stale);
        assert_eq!(report.changed.len(), 1);
        assert!(matches!(
            report.changed.first(),
            Some(ChangedArtifact {
                slot: ArtifactSlot::Binary,
                ..
            })
        ));
        Ok(())
    }

    #[test]
    fn staleness_report_detects_the_artifact_vanishing() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"present-at-startup")?;
        let startup = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        std::fs::remove_file(&artifact)?;
        let report = startup.compare_to_current();
        assert_eq!(report.verdict, Staleness::Stale);
        assert_eq!(
            report
                .changed
                .first()
                .and_then(|change| change.current.as_ref())
                .map(|entry| entry.state == ArtifactState::Missing),
            Some(true)
        );
        Ok(())
    }

    #[test]
    fn compare_freshness_free_function_matches_the_method() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"x")?;
        let a = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        let b = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        let via_method = a.compare_to_current();
        let via_function = compare_freshness(&a, &b);
        assert_eq!(via_method.verdict, via_function.verdict);
        assert_eq!(via_method.changed.len(), via_function.changed.len());
        Ok(())
    }

    #[test]
    fn running_fingerprint_resolves_the_real_test_binary() -> Result<(), FingerprintError> {
        let fingerprint = build_running_mcp_fingerprint(None)?;
        assert!(matches!(
            fingerprint.binary.state,
            ArtifactState::Present { .. }
        ));
        assert!(!fingerprint.package_version.as_str().is_empty());
        assert_eq!(fingerprint.digest.hex().len(), 64);
        Ok(())
    }

    #[test]
    fn artifact_path_capture_properties_hold_across_generated_inputs() {
        let segments = ["crates", "enforcer-mcp", "with space", "unicode", "a.b"];
        let separators = ["/", "\\"];
        for first in segments {
            for second in segments {
                for sep in separators {
                    let raw = format!("{first}{sep}{second}");
                    let captured = ArtifactPath::from_path(Path::new(&raw));
                    assert!(!captured.as_str().contains('\\'));
                    assert_eq!(captured, ArtifactPath::from_path(captured.as_path()));
                }
            }
        }
    }

    #[test]
    fn fingerprint_json_round_trip_crosses_the_mcp_boundary(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"boundary-bytes")?;
        let canonical = build_mcp_fingerprint(&artifact, version_zero_one()?, None);

        let json = encode_fingerprint_json(&canonical)?;
        let decoded = decode_fingerprint_json(&json)?;

        assert_eq!(decoded, canonical);
        assert_eq!(decoded.package_version.as_str(), "0.1.0");
        Ok(())
    }

    #[test]
    fn fingerprint_json_rejects_an_invalid_domain_digest() {
        let malformed = r#"{
            "digest":"not-a-sha256",
            "packageVersion":"0.1.0",
            "binary":{"path":"bin/enforcer","state":{"kind":"missing"}},
            "ruleset":null
        }"#;
        assert!(matches!(
            decode_fingerprint_json(malformed),
            Err(FingerprintWireError::Decode(error)) if error.path == "fingerprint.digest"
        ));
    }

    #[test]
    fn staleness_report_json_round_trip_crosses_the_mcp_boundary(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let artifact = dir.path().join("enforcer-binary");
        std::fs::write(&artifact, b"startup")?;
        let startup = build_mcp_fingerprint(&artifact, version_zero_one()?, None);
        std::fs::write(&artifact, b"current")?;
        let report = startup.compare_to_current();

        let json = encode_staleness_report_json(&report)?;
        assert_eq!(decode_staleness_report_json(&json)?, report);
        Ok(())
    }
}
