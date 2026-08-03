//! BOUNDARY-INVARIANT: raw MJS proof JSON is validated here before typed
//! mutation-risk evidence crosses into CLI/MCP adapters.
//! Read-only compatibility boundary for the frozen MJS mutation-risk proof.
//!
//! The native scanner deliberately does not know about proof storage. This
//! module owns the small MJS-compatible decoder and returns a detailed typed
//! validation result to the CLI/MCP adapters, which map it to the scanner's
//! minimal accepted/not-accepted state.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde_json::Value;

pub const MUTATION_RISK_PROOF_ID: &str = "PROOF-MUTATION-RISK-CI";
pub const MUTATION_RISK_MANIFEST: &str = ".enforce/proofs/db/proof-manifest.json";
pub const MUTATION_RISK_RUNS_DIRECTORY: &str = ".enforce/proofs/runs";
pub const MUTATION_RISK_CANONICAL_COMMAND: &str = "node scripts/ci-local.mjs";

/// The complete result of validating one project's MJS-compatible mutation
/// risk proof inventory. Rejection is deliberately represented as data so
/// callers cannot accidentally turn malformed proof state into an internal
/// error or a waiver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationRiskProofValidation {
    Accepted {
        run_id: String,
        commit: String,
        command: [String; 2],
    },
    Rejected {
        reason: MutationRiskProofRejection,
    },
}

impl MutationRiskProofValidation {
    #[must_use]
    pub const fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted { .. })
    }

    #[must_use]
    pub const fn rejection(&self) -> Option<&MutationRiskProofRejection> {
        match self {
            Self::Accepted { .. } => None,
            Self::Rejected { reason } => Some(reason),
        }
    }
}

/// Fail-closed reasons from the MJS-compatible proof boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationRiskProofRejection {
    ManifestMissing(String),
    ManifestIo(String),
    ManifestMalformed(String),
    ManifestSchema,
    ManifestRuns,
    RunIdMalformed(String),
    DuplicateRunId(String),
    RunMissing(String),
    RunIo(String),
    RunMalformed(String),
    RunIdMismatch(String),
    ProofIdMismatch(String),
    StatusNotPassed(String),
    CommitMissing,
    CommitMalformed(String),
    GitRefUnresolved(String),
    StaleCommit { expected: String, observed: String },
    CommandMalformed(String),
    CommandNotCanonical(String),
    NoAcceptedRun,
}

impl std::fmt::Display for MutationRiskProofRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ManifestMissing(path) => {
                write!(formatter, "mutation-risk proof manifest missing: {path}")
            }
            Self::ManifestIo(reason) => write!(
                formatter,
                "mutation-risk proof manifest read failed: {reason}"
            ),
            Self::ManifestMalformed(reason) => write!(
                formatter,
                "mutation-risk proof manifest malformed: {reason}"
            ),
            Self::ManifestSchema => {
                formatter.write_str("mutation-risk proof manifest schemaVersion must be 1")
            }
            Self::ManifestRuns => {
                formatter.write_str("mutation-risk proof manifest runs must be an array")
            }
            Self::RunIdMalformed(run_id) => {
                write!(formatter, "mutation-risk proof run id is unsafe: {run_id}")
            }
            Self::DuplicateRunId(run_id) => write!(
                formatter,
                "mutation-risk proof run id is duplicated: {run_id}"
            ),
            Self::RunMissing(run_id) => {
                write!(formatter, "mutation-risk proof run file missing: {run_id}")
            }
            Self::RunIo(reason) => {
                write!(formatter, "mutation-risk proof run read failed: {reason}")
            }
            Self::RunMalformed(reason) => {
                write!(formatter, "mutation-risk proof run malformed: {reason}")
            }
            Self::RunIdMismatch(run_id) => {
                write!(formatter, "mutation-risk proof run id mismatch: {run_id}")
            }
            Self::ProofIdMismatch(proof_id) => {
                write!(formatter, "mutation-risk proof id mismatch: {proof_id}")
            }
            Self::StatusNotPassed(status) => write!(
                formatter,
                "mutation-risk proof status is not passed: {status}"
            ),
            Self::CommitMissing => formatter.write_str("mutation-risk proof git.commit is missing"),
            Self::CommitMalformed(commit) => write!(
                formatter,
                "mutation-risk proof commit is not a full SHA: {commit}"
            ),
            Self::GitRefUnresolved(reason) => write!(
                formatter,
                "mutation-risk proof target ref could not resolve: {reason}"
            ),
            Self::StaleCommit { expected, observed } => write!(
                formatter,
                "mutation-risk proof commit is stale: expected {expected}, observed {observed}"
            ),
            Self::CommandMalformed(reason) => {
                write!(formatter, "mutation-risk proof command malformed: {reason}")
            }
            Self::CommandNotCanonical(command) => write!(
                formatter,
                "mutation-risk proof command is not canonical: {command}"
            ),
            Self::NoAcceptedRun => formatter
                .write_str("no passed current-commit canonical mutation-risk proof was found"),
        }
    }
}

/// Validate the MJS proof inventory for `head` (or `HEAD` when omitted).
///
/// This function only reads proof files and invokes Git for exact commit
/// resolution. It never creates, updates, or claims proof state.
#[must_use]
pub fn validate(root: &Path, head: Option<&str>) -> MutationRiskProofValidation {
    let expected_commit = match resolve_commit(root, head) {
        Ok(commit) => commit,
        Err(reason) => {
            return MutationRiskProofValidation::Rejected { reason };
        }
    };
    let manifest_path = root.join(MUTATION_RISK_MANIFEST);
    let manifest = match read_json(&manifest_path) {
        Ok(value) => value,
        Err(ReadJsonError::Missing) => {
            return rejected(MutationRiskProofRejection::ManifestMissing(
                manifest_path.display().to_string(),
            ));
        }
        Err(ReadJsonError::Io(reason)) => {
            return rejected(MutationRiskProofRejection::ManifestIo(reason));
        }
        Err(ReadJsonError::Malformed(reason)) => {
            return rejected(MutationRiskProofRejection::ManifestMalformed(reason));
        }
    };
    let Some(manifest) = manifest.as_object() else {
        return rejected(MutationRiskProofRejection::ManifestMalformed(
            "root must be an object".to_owned(),
        ));
    };
    if manifest.get("schemaVersion").and_then(Value::as_u64) != Some(1) {
        return rejected(MutationRiskProofRejection::ManifestSchema);
    }
    let Some(runs) = manifest.get("runs").and_then(Value::as_array) else {
        return rejected(MutationRiskProofRejection::ManifestRuns);
    };
    if runs.is_empty() {
        return rejected(MutationRiskProofRejection::NoAcceptedRun);
    }

    // Validate the complete manifest structure before evaluating any run.
    // An accepted early run must never hide a later duplicate or malformed
    // entry.
    let mut seen = HashSet::with_capacity(runs.len());
    let mut run_ids = Vec::with_capacity(runs.len());
    for entry in runs {
        let Some(entry) = entry.as_object() else {
            return rejected(MutationRiskProofRejection::ManifestMalformed(
                "runs entries must be objects".to_owned(),
            ));
        };
        let Some(run_id) = entry.get("runId").and_then(Value::as_str) else {
            return rejected(MutationRiskProofRejection::ManifestMalformed(
                "runs entries require string runId".to_owned(),
            ));
        };
        if !safe_run_id(run_id) {
            return rejected(MutationRiskProofRejection::RunIdMalformed(
                run_id.to_owned(),
            ));
        }
        if !seen.insert(run_id.to_owned()) {
            return rejected(MutationRiskProofRejection::DuplicateRunId(
                run_id.to_owned(),
            ));
        }
        run_ids.push(run_id.to_owned());
    }

    let mut evaluations = Vec::with_capacity(run_ids.len());
    for run_id in run_ids {
        let run_path = root
            .join(MUTATION_RISK_RUNS_DIRECTORY)
            .join(&run_id)
            .join("proof-run.json");
        let run = match read_json(&run_path) {
            Ok(value) => value,
            Err(ReadJsonError::Missing) => {
                return rejected(MutationRiskProofRejection::RunMissing(run_id));
            }
            Err(ReadJsonError::Io(reason)) => {
                return rejected(MutationRiskProofRejection::RunIo(reason));
            }
            Err(ReadJsonError::Malformed(reason)) => {
                return rejected(MutationRiskProofRejection::RunMalformed(reason));
            }
        };
        let evaluation = validate_run(&run, &run_id, &expected_commit, root);
        if let Err(reason) = &evaluation {
            if reason.is_structural() {
                return rejected(reason.clone());
            }
        }
        evaluations.push((run_id, evaluation));
    }

    let mut last_rejection = None;
    for (run_id, evaluation) in evaluations {
        match evaluation {
            Ok((commit, command)) => {
                return MutationRiskProofValidation::Accepted {
                    run_id,
                    commit,
                    command,
                };
            }
            Err(reason) => last_rejection = Some(reason),
        }
    }

    rejected(last_rejection.unwrap_or(MutationRiskProofRejection::NoAcceptedRun))
}

fn rejected(reason: MutationRiskProofRejection) -> MutationRiskProofValidation {
    MutationRiskProofValidation::Rejected { reason }
}

impl MutationRiskProofRejection {
    fn is_structural(&self) -> bool {
        matches!(
            self,
            Self::RunMalformed(_)
                | Self::RunIdMismatch(_)
                | Self::CommitMissing
                | Self::CommitMalformed(_)
                | Self::CommandMalformed(_)
        )
    }
}

fn validate_run(
    run: &Value,
    manifest_run_id: &str,
    expected_commit: &str,
    root: &Path,
) -> Result<(String, [String; 2]), MutationRiskProofRejection> {
    let Some(run) = run.as_object() else {
        return Err(MutationRiskProofRejection::RunMalformed(
            "root must be an object".to_owned(),
        ));
    };
    if let Some(run_id) = run.get("runId") {
        let Some(run_id) = run_id.as_str() else {
            return Err(MutationRiskProofRejection::RunMalformed(
                "runId must be a string when present".to_owned(),
            ));
        };
        if run_id != manifest_run_id {
            return Err(MutationRiskProofRejection::RunIdMismatch(run_id.to_owned()));
        }
    }
    let proof_id = run.get("proofId").and_then(Value::as_str).ok_or_else(|| {
        MutationRiskProofRejection::RunMalformed("proofId is required".to_owned())
    })?;
    if proof_id != MUTATION_RISK_PROOF_ID {
        return Err(MutationRiskProofRejection::ProofIdMismatch(
            proof_id.to_owned(),
        ));
    }
    let status = run
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| MutationRiskProofRejection::RunMalformed("status is required".to_owned()))?;
    if status != "passed" {
        return Err(MutationRiskProofRejection::StatusNotPassed(
            status.to_owned(),
        ));
    }
    let commit = run
        .get("git")
        .and_then(Value::as_object)
        .and_then(|git| git.get("commit"))
        .and_then(Value::as_str)
        .ok_or(MutationRiskProofRejection::CommitMissing)?;
    if !full_sha(commit) {
        return Err(MutationRiskProofRejection::CommitMalformed(
            commit.to_owned(),
        ));
    }
    if commit != expected_commit {
        return Err(MutationRiskProofRejection::StaleCommit {
            expected: expected_commit.to_owned(),
            observed: commit.to_owned(),
        });
    }
    let command = parse_command(run.get("command"), root)?;
    Ok((commit.to_owned(), command))
}

fn parse_command(
    value: Option<&Value>,
    root: &Path,
) -> Result<[String; 2], MutationRiskProofRejection> {
    let Some(command) = value.and_then(Value::as_array) else {
        return Err(MutationRiskProofRejection::CommandMalformed(
            "command must be an argv array".to_owned(),
        ));
    };
    if command.len() != 2 {
        return Err(MutationRiskProofRejection::CommandMalformed(
            "command must contain exactly two argv entries".to_owned(),
        ));
    }
    let Some(executable) = command.first().and_then(Value::as_str) else {
        return Err(MutationRiskProofRejection::CommandMalformed(
            "argv[0] must be a string".to_owned(),
        ));
    };
    let Some(script) = command.get(1).and_then(Value::as_str) else {
        return Err(MutationRiskProofRejection::CommandMalformed(
            "argv[1] must be a string".to_owned(),
        ));
    };
    let executable_name = executable.rsplit(['/', '\\']).next().unwrap_or(executable);
    if !executable_name.eq_ignore_ascii_case("node")
        && !executable_name.eq_ignore_ascii_case("node.exe")
    {
        return Err(MutationRiskProofRejection::CommandNotCanonical(format!(
            "{executable} {script}"
        )));
    }
    let expected_script = lexical_normalize(&root.join("scripts").join("ci-local.mjs"));
    let observed_script = lexical_normalize(&resolve_command_path(root, script));
    if observed_script != expected_script {
        return Err(MutationRiskProofRejection::CommandNotCanonical(format!(
            "{executable} {script}"
        )));
    }
    Ok([executable.to_owned(), script.to_owned()])
}

fn resolve_command_path(root: &Path, raw: &str) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() {
        path.to_owned()
    } else {
        root.join(path)
    }
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(std::path::MAIN_SEPARATOR.to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

fn safe_run_id(run_id: &str) -> bool {
    !run_id.is_empty()
        && run_id != "."
        && run_id != ".."
        && !run_id.contains('/')
        && !run_id.contains('\\')
        && !Path::new(run_id).is_absolute()
        && Path::new(run_id)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn full_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn resolve_commit(root: &Path, head: Option<&str>) -> Result<String, MutationRiskProofRejection> {
    let reference = head.unwrap_or("HEAD");
    let expression = format!("{reference}^{{commit}}");
    let output = Command::new("git")
        .args(["rev-parse", "--verify", expression.as_str()])
        .current_dir(root)
        .output()
        .map_err(|error| MutationRiskProofRejection::GitRefUnresolved(error.to_string()))?;
    if !output.status.success() {
        return Err(MutationRiskProofRejection::GitRefUnresolved(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    let commit = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !full_sha(&commit) {
        return Err(MutationRiskProofRejection::GitRefUnresolved(commit));
    }
    Ok(commit)
}

enum ReadJsonError {
    Missing,
    Io(String),
    Malformed(String),
}

fn read_json(path: &Path) -> Result<Value, ReadJsonError> {
    let bytes = std::fs::read(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ReadJsonError::Missing
        } else {
            ReadJsonError::Io(error.to_string())
        }
    })?;
    serde_json::from_slice(&bytes).map_err(|error| ReadJsonError::Malformed(error.to_string()))
}
