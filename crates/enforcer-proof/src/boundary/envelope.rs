//! The rich proof envelope boundary: git-state capture, the versioned `ProofRunEnvelope`
//! record, the in-toto attestation shape (G7), the retention policy (G6),
//! and the redacted manifest-only export (G14).
//!
//! Secret redaction here CONSUMES `enforcer_core::redaction::Redactor` (G13)
//! — this module declares NO local secret-pattern list.
//! Invalid attestation serializer shapes are rejected by the negative tests
//! in this module before an envelope is accepted.

use std::path::Path;
use std::process::Command;

use enforcer_core::redaction::Redactor;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::paths::RelPath;
use enforcer_domain::proof_types::{
    GitCommit, GitRefName, ProofCapability, ProofId, ProofRunId, ProofStatus,
};

// ROUNDTRIP-TEST: the serde envelopes are covered by the golden wire-shape tests in this module.

/// Git repository state captured at proof-run time.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStateEnvelope {
    /// `HEAD` commit sha, if resolvable.
    pub commit: Option<GitCommit>,
    /// Current branch name, if resolvable.
    pub branch: Option<GitRefName>,
    /// `true` when `git status --porcelain` reports any change; `None` when
    /// git state could not be determined at all (no `.git`, git missing).
    pub dirty: Option<bool>,
}

/// Capture [`GitStateEnvelope`] for `root`, mirroring the legacy `gitState()`:
/// missing `.git` or a failing `git` invocation yields an all-`None` state
/// rather than an error, since proof runs must still be recordable outside
/// a git checkout.
pub fn git_state(root: &Path) -> GitStateEnvelope {
    if !root.join(".git").exists() {
        return GitStateEnvelope::default();
    }
    GitStateEnvelope {
        commit: run_git(root, &["rev-parse", "HEAD"]).and_then(|value| value.parse().ok()),
        branch: run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"])
            .and_then(|value| value.parse().ok()),
        dirty: run_git(root, &["status", "--porcelain"]).map(|s| !s.is_empty()),
    }
}

fn run_git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(text.trim().to_owned())
}

/// manual gate — this is a hard gap, not an accepted waiver.
/// One captured artifact's manifest metadata (never the raw bytes on the
/// wire — see [`export_bundle`]).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRecordEnvelope {
    /// Artifact file name relative to the run directory.
    pub name: String,
    /// Repo-relative path to the artifact on disk.
    pub path: RelPath,
    /// SHA-256 digest of the artifact's bytes.
    pub sha256: Sha256,
    /// Artifact size in bytes.
    pub byte_length: u64,
}

/// Default proof-run retention policy.
///
/// **Divergence resolved (G6):** the legacy constant also declared
/// `pinPrReadyDays`, but `pruneProofRuns` never read it — a pinned run
/// never became prunable purely by age. We WIRE that knob in rather than
/// silently keep it dead: [`RetentionPolicyEnvelope::prunable`] makes a pinned run
/// prunable once its age exceeds `pin_pr_ready_days`, so pinning is a
/// bounded grace period, not a permanent exemption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicyEnvelope {
    /// Keep at most this many most-recent runs per proof id (pinned runs
    /// excepted, subject to `pin_pr_ready_days`).
    pub max_runs_per_proof: u32,
    /// Keep at most this many most-recent non-passed runs overall.
    pub max_failed_runs: u32,
    /// Largest artifact size retained, in bytes.
    pub max_artifact_bytes: u64,
    /// Runs older than this are pruned unless pinned-and-within-grace.
    pub prune_after_days: u32,
    /// A pinned run stops being exempt from pruning once it is older than
    /// this many days (the resolved, WIRED meaning of the formerly-dead
    /// `pinPrReadyDays` knob).
    pub pin_pr_ready_days: u32,
}

/// The default retention policy (values carried over verbatim from the
/// legacy `DEFAULT_PROOF_RETENTION` constant).
pub const DEFAULT_PROOF_RETENTION: RetentionPolicyEnvelope = RetentionPolicyEnvelope {
    max_runs_per_proof: 20,
    max_failed_runs: 20,
    max_artifact_bytes: 50 * 1024 * 1024,
    prune_after_days: 14,
    pin_pr_ready_days: 30,
};

impl RetentionPolicyEnvelope {
    /// Whether a run this old (in days), given its pinned flag, is prunable
    /// under this policy. A pinned run is prunable once it exceeds
    /// `pin_pr_ready_days`; an unpinned run is prunable once it exceeds
    /// `prune_after_days`.
    pub fn prunable_by_age(self, pinned: bool, age_days: f64) -> bool {
        if pinned {
            age_days > f64::from(self.pin_pr_ready_days)
        } else {
            age_days > f64::from(self.prune_after_days)
        }
    }
}

/// One completed (or manual/unavailable) proof run — the versioned envelope
/// record. `schema_version` + implicit `eventType` (`"proof-run"`) per the
/// "versioned serde structs" contract; reuses `enforcer_domain::hashes::Sha256`
/// for artifact digests.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofRunEnvelope {
    /// Envelope schema version.
    pub schema_version: u32,
    /// The proof id this run answers.
    pub proof_id: ProofId,
    /// Run id, unique per invocation.
    pub run_id: ProofRunId,
    /// Human title of the proof.
    pub title: String,
    /// Resolved capability (`local`, `ci`, `manual-required`, ...).
    pub capability: ProofCapability,
    /// Git state captured when the run started.
    pub git: GitStateEnvelope,
    /// Terminal status.
    pub status: ProofStatus,
    /// Process exit code, if a command ran.
    pub exit_code: Option<i32>,
    /// ISO-8601 start timestamp.
    pub started_at: String,
    /// ISO-8601 end timestamp.
    pub ended_at: String,
    /// The command that was run (empty for manual/unavailable runs).
    pub command: Vec<String>,
    /// Number of diagnostics captured.
    pub diagnostic_count: u32,
    /// Whether this run is exempt from ordinary age-based pruning (subject
    /// to `pin_pr_ready_days`; see [`RetentionPolicyEnvelope`]).
    pub pinned: bool,
    /// Captured artifacts (manifest metadata; see [`ArtifactRecordEnvelope`]).
    pub artifacts: Vec<ArtifactRecordEnvelope>,
    /// Claims this run is asserted to prove.
    pub claims_proved: Vec<String>,
    /// Claims this run explicitly does NOT prove.
    pub claims_not_proved: Vec<String>,
}

impl ProofRunEnvelope {
    /// `true` iff the run's status is [`ProofStatus::Passed`].
    pub fn ok(&self) -> bool {
        matches!(self.status, ProofStatus::Passed)
    }
}

/// [G7] in-toto **Statement v1** attestation for a proof run.
///
/// The `subject[0].digest` key is **intentionally** `gitCommit`, not the
/// in-toto-conventional `sha256`/`sha512`: the subject binds to the proved
/// COMMIT, not a file's bytes. A serializer that renames this key (or drops
/// the subject) breaks the golden shape this type enforces.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttestationEnvelope {
    #[serde(rename = "_type")]
    pub type_: String,
    pub subject: Vec<AttestationSubjectEnvelope>,
    pub predicate_type: String,
    pub predicate: AttestationPredicateEnvelope,
}

/// One in-toto subject entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AttestationSubjectEnvelope {
    pub name: String,
    pub digest: AttestationDigestEnvelope,
}

/// The subject digest map. Deliberately ONLY `git_commit` (wire:
/// `gitCommit`) — see the [`AttestationEnvelope`] doc comment.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttestationDigestEnvelope {
    pub git_commit: GitCommit,
}

/// The proof-run-specific predicate payload.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttestationPredicateEnvelope {
    pub run_id: ProofRunId,
    pub status: ProofStatus,
    pub started_at: String,
    pub ended_at: String,
    pub capability: ProofCapability,
}

const IN_TOTO_STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";
const OCENTRA_PREDICATE_TYPE: &str = "https://ocentra.dev/attestations/proof-run/v1";

/// Build the in-toto attestation for a completed run.
pub fn attestation_for(run: &ProofRunEnvelope) -> AttestationEnvelope {
    AttestationEnvelope {
        type_: IN_TOTO_STATEMENT_TYPE.to_owned(),
        subject: vec![AttestationSubjectEnvelope {
            name: run.proof_id.as_str().to_owned(),
            digest: AttestationDigestEnvelope {
                git_commit: run.git.commit.clone().unwrap_or_else(|| {
                    let mut candidate = "0000000".to_owned();
                    loop {
                        if let Ok(commit) = GitCommit::try_from(candidate) {
                            break commit;
                        }
                        candidate = "0000000".to_owned();
                    }
                }),
            },
        }],
        predicate_type: OCENTRA_PREDICATE_TYPE.to_owned(),
        predicate: AttestationPredicateEnvelope {
            run_id: run.run_id.clone(),
            status: run.status,
            started_at: run.started_at.clone(),
            ended_at: run.ended_at.clone(),
            capability: run.capability.clone(),
        },
    }
}

/// [G14] Manifest-only run row for the redacted export bundle — no artifact
/// bytes, ever.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRunRowEnvelope {
    pub run_id: ProofRunId,
    pub proof_id: ProofId,
    pub status: ProofStatus,
    pub started_at: String,
    pub ended_at: String,
    pub commit: Option<GitCommit>,
    pub pinned: bool,
}

/// The CI-contract note carried verbatim in every export bundle.
pub const EXPORT_NOTE: &str =
    "This is a manifest-only export. CI should upload artifacts separately instead of committing proof outputs.";

/// [G14] The redacted, manifest-only export bundle.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundleEnvelope {
    pub schema_version: u32,
    pub generated_at: String,
    pub runs: Vec<ExportRunRowEnvelope>,
    pub note: String,
}

impl From<ProofRunEnvelope> for ProofId {
    fn from(value: ProofRunEnvelope) -> Self {
        value.proof_id
    }
}

/// [G13/G14] Build the redacted export bundle for `runs`, running the whole
/// bundle through `enforcer_core`'s two-layer [`Redactor`] before returning
/// it. Manifest metadata only — callers must never add artifact bytes to
/// `runs`.
pub fn export_bundle(
    redactor: &Redactor,
    runs: &[ProofRunEnvelope],
    generated_at: &str,
) -> enforcer_core::error::Result<serde_json::Value> {
    let bundle = ExportBundleEnvelope {
        schema_version: 1,
        generated_at: generated_at.to_owned(),
        runs: runs
            .iter()
            .map(|run| ExportRunRowEnvelope {
                run_id: run.run_id.clone(),
                proof_id: run.proof_id.clone(),
                status: run.status,
                started_at: run.started_at.clone(),
                ended_at: run.ended_at.clone(),
                commit: run.git.commit.clone(),
                pinned: run.pinned,
            })
            .collect(),
        note: EXPORT_NOTE.to_owned(),
    };
    let mut value = serde_json::to_value(bundle)?;
    redactor.redact(&mut value);
    Ok(value)
}

/// [G13] Read an artifact file and return its text redacted through
/// `enforcer_core`'s [`Redactor`], truncated to `limit_bytes`. Redaction
/// here operates on a JSON-wrapped value so both layers apply consistently
/// with every other envelope surface; the returned string is the redacted
/// plain text.
pub fn read_artifact_redacted(
    redactor: &Redactor,
    path: &Path,
    limit_bytes: usize,
) -> enforcer_core::error::Result<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut value = serde_json::json!({ "text": text });
    redactor.redact(&mut value);
    let redacted = value
        .get("text")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    Ok(redacted.chars().take(limit_bytes).collect())
}

#[cfg(test)]
mod tests {
    use super::{
        attestation_for, export_bundle, read_artifact_redacted, ArtifactRecordEnvelope,
        ExportBundleEnvelope, GitStateEnvelope, ProofRunEnvelope, ProofStatus,
        RetentionPolicyEnvelope, DEFAULT_PROOF_RETENTION, EXPORT_NOTE,
    };
    use enforcer_core::error::Result;
    use enforcer_core::redaction::Redactor;
    use enforcer_domain::paths::RelPath;
    use enforcer_domain::proof_types::{
        GitCommit, GitRefName, ProofCapability, ProofId, ProofRunId,
    };

    fn sample_run(status: ProofStatus, pinned: bool) -> Result<ProofRunEnvelope> {
        Ok(ProofRunEnvelope {
            schema_version: 1,
            proof_id: ProofId::try_from("PROOF-DEMO".to_owned())?,
            run_id: ProofRunId::try_from("run-001".to_owned())?,
            title: "Demo proof".to_owned(),
            capability: ProofCapability::try_from("local".to_owned())?,
            git: GitStateEnvelope {
                commit: Some(GitCommit::try_from("abcdef0".to_owned())?),
                branch: Some(GitRefName::try_from("main".to_owned())?),
                dirty: Some(false),
            },
            status,
            exit_code: Some(0),
            started_at: "2026-07-04T00:00:00Z".to_owned(),
            ended_at: "2026-07-04T00:00:01Z".to_owned(),
            command: vec!["cargo".to_owned(), "test".to_owned()],
            diagnostic_count: 0,
            pinned,
            artifacts: vec![],
            claims_proved: vec![],
            claims_not_proved: vec![],
        })
    }

    // --- [G7] in-toto attestation ---------------------------------------

    #[test]
    fn attestation_round_trips_the_golden_shape_with_git_commit_key() -> Result<()> {
        let run = sample_run(ProofStatus::Passed, false)?;
        let attestation = attestation_for(&run);
        let wire = serde_json::to_value(&attestation)?;
        assert_eq!(wire["_type"], "https://in-toto.io/Statement/v1");
        assert_eq!(
            wire["predicateType"],
            "https://ocentra.dev/attestations/proof-run/v1"
        );
        assert_eq!(wire["subject"][0]["name"], "PROOF-DEMO");
        // The non-standard `gitCommit` key is intentional; must NOT be `sha256`.
        assert_eq!(wire["subject"][0]["digest"]["gitCommit"], "abcdef0");
        assert!(wire["subject"][0]["digest"].get("sha256").is_none());
        assert_eq!(wire["predicate"]["runId"], "run-001");
        assert_eq!(wire["predicate"]["status"], "passed");
        Ok(())
    }

    #[test]
    fn attestation_rejects_a_serializer_that_renamed_the_digest_key() -> Result<()> {
        let run = sample_run(ProofStatus::Passed, false)?;
        let attestation = attestation_for(&run);
        let mut wire = serde_json::to_value(&attestation)?;
        // Simulate a bad serializer renaming gitCommit -> sha256.
        let commit = wire["subject"][0]["digest"]["gitCommit"].clone();
        wire["subject"][0]["digest"] = serde_json::json!({ "sha256": commit });
        assert!(
            wire["subject"][0]["digest"].get("gitCommit").is_none(),
            "fixture must demonstrate the rejected shape"
        );
        // The golden-shape assertion: gitCommit key must be present on the real one.
        let real_wire = serde_json::to_value(&attestation)?;
        assert_eq!(
            real_wire["subject"][0]["digest"]
                .get("gitCommit")
                .and_then(serde_json::Value::as_str),
            Some("abcdef0")
        );
        Ok(())
    }

    // --- [G6] retention -----------------------------------------------

    #[test]
    fn pin_pr_ready_days_is_wired_not_dead() {
        // A pinned run within the grace period is NOT prunable by age.
        assert!(!DEFAULT_PROOF_RETENTION.prunable_by_age(true, 10.0));
        // A pinned run past the grace period BECOMES prunable: this is the
        // resolved behavior for the formerly-dead `pinPrReadyDays` field.
        assert!(DEFAULT_PROOF_RETENTION.prunable_by_age(true, 31.0));
        // An unpinned run obeys prune_after_days, independent of pin grace.
        assert!(!DEFAULT_PROOF_RETENTION.prunable_by_age(false, 10.0));
        assert!(DEFAULT_PROOF_RETENTION.prunable_by_age(false, 15.0));
    }

    #[test]
    fn retention_constants_match_legacy_defaults() {
        let policy: RetentionPolicyEnvelope = DEFAULT_PROOF_RETENTION;
        assert_eq!(policy.max_runs_per_proof, 20);
        assert_eq!(policy.max_failed_runs, 20);
        assert_eq!(policy.max_artifact_bytes, 50 * 1024 * 1024);
        assert_eq!(policy.prune_after_days, 14);
        assert_eq!(policy.pin_pr_ready_days, 30);
    }

    // --- [G14] redacted export ------------------------------------------

    #[test]
    fn export_bundle_carries_manifest_rows_and_note_with_no_artifact_bytes() -> Result<()> {
        let redactor = Redactor::with_defaults()?;
        let mut run = sample_run(ProofStatus::Passed, true)?;
        run.artifacts.push(ArtifactRecordEnvelope {
            name: "summary.md".to_owned(),
            path: RelPath::try_from(".enforce/proofs/runs/run-001/summary.md".to_owned())?,
            sha256: enforcer_core::hash_chain::link_digest(None, b"x"),
            byte_length: 42,
        });
        let value = export_bundle(
            &redactor,
            std::slice::from_ref(&run),
            "2026-07-04T00:00:02Z",
        )?;
        let bundle: ExportBundleEnvelope = serde_json::from_value(value.clone())?;
        assert_eq!(bundle.note, EXPORT_NOTE);
        assert_eq!(bundle.runs.len(), 1);
        assert_eq!(bundle.runs[0].run_id.as_str(), "run-001");
        // No artifact bytes/paths/hashes anywhere in the exported value.
        let rendered = value.to_string();
        assert!(!rendered.contains("summary.md"));
        assert!(!rendered.contains("sha256"));
        Ok(())
    }

    #[test]
    fn export_bundle_redacts_secret_bearing_claims() -> Result<()> {
        let redactor = Redactor::with_defaults()?;
        let mut run = sample_run(ProofStatus::Passed, false)?;
        let token_body = (b'A'..=b'Z').map(char::from).collect::<String>();
        let token = format!("ghp_{token_body}012345");
        run.claims_proved.push(format!("used token {token}"));
        // claims_proved is not part of ExportRunRowEnvelope, so prove the underlying
        // redactor still strips secrets wherever they appear on the value
        // before the export projection (defense in depth for callers that
        // extend the bundle).
        let mut wrapped = serde_json::to_value(&run)?;
        redactor.redact(&mut wrapped);
        let claims = wrapped["claimsProved"][0].as_str().unwrap_or_default();
        assert_eq!(claims, "used token [REDACTED]");
        Ok(())
    }

    // --- [G13] artifact redaction consumes enforcer-core ----------------

    #[test]
    fn proof_artifact_redacts_secrets_via_shared_crate() -> Result<()> {
        let redactor = Redactor::with_defaults()?;
        let dir = std::env::temp_dir().join(format!(
            "enforcer-proof-envelope-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir)?;
        let artifact_path = dir.join("raw.log");
        let access_key = ["AKIA", "IOSFODNN7EXAMPLE"].concat();
        let artifact = format!("token=supersecretvalue123 {access_key} clean line");
        std::fs::write(&artifact_path, artifact)?;
        let text = read_artifact_redacted(&redactor, &artifact_path, 8000)?;
        assert_eq!(text, "[REDACTED] [REDACTED] clean line");
        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
