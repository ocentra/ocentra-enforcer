//! Native process execution for the frozen MCP `ocentra_enforcer_run` tool.

use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc::{self, Receiver, RecvTimeoutError},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use command_group::{CommandGroup, GroupChild};
use enforcer_core::error::{Error, Result};
use enforcer_domain::config_types::{CrateName, HarnessConfig};
use enforcer_domain::harness_types::{
    HarnessBoundedExecution, HarnessCapturedOutput, HarnessCommandArgument, HarnessDomainName,
    HarnessExecutionTermination, HarnessLanguage, HarnessPackageName, HarnessPinned, HarnessRunId,
    HarnessTag, HarnessTimestamp, HarnessToolName, HarnessToolSpec,
};
use enforcer_domain::paths::RepoRoot;
use enforcer_domain::telemetry_types::ProcessExitCode;

use crate::storage::{record_run, RunInput, RunOutcome};

const READER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const ARBITRARY_MAX_WALL_TIME: Duration = Duration::from_secs(30 * 60);
const ARBITRARY_MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

/// Fully decoded, boundary-owned request for one recorded native command.
#[derive(Debug, Clone)]
pub struct ExecuteRequest {
    pub repo_root: RepoRoot,
    pub cwd: Option<String>,
    pub run_id: HarnessRunId,
    pub tool: HarnessToolName,
    pub language: Option<HarnessLanguage>,
    pub command: Vec<HarnessCommandArgument>,
    pub crate_name: Option<CrateName>,
    pub package_name: Option<HarnessPackageName>,
    pub domain: Option<HarnessDomainName>,
    pub tags: Vec<HarnessTag>,
}

/// Validate that a request matches one reviewed allowlisted tool spec.
///
/// This is deliberately separate from [`execute`]: the existing arbitrary
/// user-invoked runner remains available, while policy callers must opt into
/// this exact command and repository-relative working-directory seam before a
/// later bounded adapter executes the process.
pub fn validate_allowlisted_request(
    request: &ExecuteRequest,
    spec: &HarnessToolSpec,
) -> Result<()> {
    if request.tool != *spec.tool() {
        return Err(enforcer_core::error::Error::InvalidConfig(
            "allowlisted tool identity does not match the request".to_owned(),
        ));
    }
    if request.command.as_slice() != spec.command() {
        return Err(enforcer_core::error::Error::InvalidConfig(
            "allowlisted command does not match the reviewed template".to_owned(),
        ));
    }
    if let Some(cwd) = request.cwd.as_deref() {
        let path = Path::new(cwd);
        if path.components().any(|component| {
            matches!(
                component,
                std::path::Component::Prefix(_)
                    | std::path::Component::RootDir
                    | std::path::Component::ParentDir
            )
        }) {
            return Err(enforcer_core::error::Error::InvalidConfig(
                "allowlisted working directory must stay within the repository root".to_owned(),
            ));
        }
    }
    Ok(())
}

/// Execute one reviewed command with bounded output and wall time.
///
/// The existing [`execute`] function remains the arbitrary user-invoked runner;
/// this seam is the only path that combines the reviewed command and cwd policy
/// with child-process limits. A timeout or output overflow always kills, waits
/// for, and drains the child before returning a typed result.
pub fn execute_allowlisted_bounded(
    request: &ExecuteRequest,
    spec: &HarnessToolSpec,
) -> Result<HarnessBoundedExecution> {
    validate_allowlisted_request(request, spec)?;
    execute_bounded_process(
        request,
        Duration::from_millis(spec.limits().max_wall_time_ms()),
        usize::try_from(spec.limits().max_output_bytes()).unwrap_or(usize::MAX),
    )
}

/// Execute an already-decoded command without recording a harness run, while
/// retaining the same process-tree, wall-time, and output bounds as the public
/// arbitrary harness runner. Proof lifecycle uses this seam because it owns a
/// separate durable run envelope and must not create a second harness record.
pub fn execute_unrecorded_bounded(request: &ExecuteRequest) -> Result<HarnessBoundedExecution> {
    execute_bounded_process(request, ARBITRARY_MAX_WALL_TIME, ARBITRARY_MAX_OUTPUT_BYTES)
}

fn execute_bounded_process(
    request: &ExecuteRequest,
    max_wall_time: Duration,
    output_limit: usize,
) -> Result<HarnessBoundedExecution> {
    let executable = request.command.first().ok_or_else(|| {
        Error::InvalidConfig("allowlisted tool command must not be empty".to_owned())
    })?;
    let cwd = request
        .cwd
        .as_deref()
        .map(|relative| Path::new(request.repo_root.as_str()).join(relative))
        .unwrap_or_else(|| Path::new(request.repo_root.as_str()).to_path_buf());

    let mut command = Command::new(executable.as_str());
    command
        .args(
            request
                .command
                .iter()
                .skip(1)
                .map(HarnessCommandArgument::as_str),
        )
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match command.group_spawn() {
        Ok(child) => child,
        Err(error) => {
            let termination = if error.kind() == io::ErrorKind::NotFound {
                HarnessExecutionTermination::MissingExecutable
            } else {
                HarnessExecutionTermination::SpawnFailed
            };
            return Ok(HarnessBoundedExecution::from_parts(
                termination,
                HarnessCapturedOutput::from_owned(String::new()),
                HarnessCapturedOutput::from_owned(String::new()),
                None,
                false,
            ));
        }
    };

    let stdout = match child.inner().stdout.take() {
        Some(stdout) => stdout,
        None => return Err(reap_after_pipe_failure(&mut child, "stdout")),
    };
    let stderr = match child.inner().stderr.take() {
        Some(stderr) => stderr,
        None => return Err(reap_after_pipe_failure(&mut child, "stderr")),
    };
    let output_bytes = Arc::new(AtomicUsize::new(0));
    let output_overflow = Arc::new(AtomicBool::new(false));
    let stdout_reader = spawn_bounded_reader(
        stdout,
        output_limit,
        Arc::clone(&output_bytes),
        Arc::clone(&output_overflow),
    );
    let stderr_reader = spawn_bounded_reader(
        stderr,
        output_limit,
        output_bytes,
        Arc::clone(&output_overflow),
    );

    let deadline = Instant::now() + max_wall_time;
    let mut cleanup_error = None;
    let (status, forced_termination) = loop {
        if output_overflow.load(Ordering::Acquire) {
            match terminate_and_reap(&mut child) {
                Ok(status) => {
                    break (
                        Some(status),
                        Some(HarnessExecutionTermination::OutputLimitExceeded),
                    );
                }
                Err(error) => {
                    cleanup_error = Some(error);
                    break (None, Some(HarnessExecutionTermination::OutputLimitExceeded));
                }
            }
        }
        match child.try_wait() {
            Ok(Some(exit_status)) => {
                // The process-group leader may exit while descendants retain
                // inherited output pipes. Terminate the residual group before
                // joining readers so a successful leader cannot leak children
                // or wedge this request indefinitely.
                if let Err(error) = child.kill() {
                    if !residual_group_is_already_gone(&error) {
                        record_cleanup_error(
                            &mut cleanup_error,
                            invalid_process_error("terminate residual process group", &error),
                        );
                    }
                }
                break (Some(exit_status), None);
            }
            Ok(None) => {}
            Err(error) => {
                record_cleanup_error(
                    &mut cleanup_error,
                    invalid_process_error("poll child", &error),
                );
                match terminate_and_reap(&mut child) {
                    Ok(status) => break (Some(status), None),
                    Err(error) => {
                        record_cleanup_error(&mut cleanup_error, error);
                        break (None, None);
                    }
                }
            }
        }
        if Instant::now() >= deadline {
            match terminate_and_reap(&mut child) {
                Ok(status) => break (Some(status), Some(HarnessExecutionTermination::TimedOut)),
                Err(error) => {
                    record_cleanup_error(&mut cleanup_error, error);
                    break (None, Some(HarnessExecutionTermination::TimedOut));
                }
            }
        }
        thread::sleep(Duration::from_millis(1));
    };

    let stdout = match join_bounded_reader(stdout_reader) {
        Ok(stdout) => stdout,
        Err(error) => {
            record_cleanup_error(&mut cleanup_error, error);
            Vec::new()
        }
    };
    let stderr = match join_bounded_reader(stderr_reader) {
        Ok(stderr) => stderr,
        Err(error) => {
            record_cleanup_error(&mut cleanup_error, error);
            Vec::new()
        }
    };
    if let Some(error) = cleanup_error {
        return Err(error);
    }
    let mut converted_budget = output_limit;
    let (stdout, stdout_truncated) = bounded_lossy_output(&stdout, &mut converted_budget);
    let (stderr, stderr_truncated) = bounded_lossy_output(&stderr, &mut converted_budget);
    let termination = forced_termination.unwrap_or_else(|| {
        if output_overflow.load(Ordering::Acquire) || stdout_truncated || stderr_truncated {
            HarnessExecutionTermination::OutputLimitExceeded
        } else if status.as_ref().is_some_and(ExitStatus::success) {
            HarnessExecutionTermination::Completed
        } else {
            HarnessExecutionTermination::NonZeroExit
        }
    });
    let exit_code = status.map(|exit_status| {
        enforcer_domain::telemetry_types::ProcessExitCode::new(exit_status.code().unwrap_or(1))
    });
    Ok(HarnessBoundedExecution::from_parts(
        termination,
        HarnessCapturedOutput::from_owned(stdout),
        HarnessCapturedOutput::from_owned(stderr),
        exit_code,
        true,
    ))
}

fn residual_group_is_already_gone(error: &io::Error) -> bool {
    if matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::InvalidInput
    ) {
        return true;
    }
    // command-group signals the Unix process group after its leader exits.
    // A group with no remaining descendants returns ESRCH (3), which means
    // the desired cleanup state has already been reached.
    #[cfg(unix)]
    if error.raw_os_error() == Some(3) {
        return true;
    }
    false
}

fn termination_failure_after_reap_is_benign(error: &io::Error) -> bool {
    if residual_group_is_already_gone(error) {
        return true;
    }
    // On macOS, command-group can race a short-lived process-group leader:
    // killpg reports EPERM after the leader has exited, while wait still
    // successfully reaps that owned child. Reader shutdown remains bounded,
    // so a descendant retaining either output pipe still fails closed.
    #[cfg(target_os = "macos")]
    if error.raw_os_error() == Some(1) {
        return true;
    }
    false
}

struct BoundedReader {
    receiver: Receiver<io::Result<Vec<u8>>>,
    handle: JoinHandle<()>,
}

fn spawn_bounded_reader<R>(
    mut reader: R,
    output_limit: usize,
    output_bytes: Arc<AtomicUsize>,
    output_overflow: Arc<AtomicBool>,
) -> BoundedReader
where
    R: Read + Send + 'static,
{
    let (sender, receiver) = mpsc::sync_channel(1);
    let handle = thread::spawn(move || {
        let result = (|| {
            let mut captured = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let read = reader.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                let previous = output_bytes.fetch_add(read, Ordering::AcqRel);
                if previous >= output_limit {
                    output_overflow.store(true, Ordering::Release);
                    break;
                }
                let allowed = output_limit.saturating_sub(previous);
                if read > allowed {
                    captured.extend(buffer.iter().take(allowed));
                    output_overflow.store(true, Ordering::Release);
                    break;
                }
                captured.extend(buffer.iter().take(read));
            }
            Ok(captured)
        })();
        let _ignored = sender.send(result);
    });
    BoundedReader { receiver, handle }
}

fn bounded_lossy_output(bytes: &[u8], remaining: &mut usize) -> (String, bool) {
    let lossy = String::from_utf8_lossy(bytes);
    let mut output = String::new();
    let mut truncated = false;
    for character in lossy.chars() {
        let width = character.len_utf8();
        if width > *remaining {
            truncated = true;
            break;
        }
        output.push(character);
        *remaining -= width;
    }
    if output.len() < lossy.len() {
        truncated = true;
    }
    (output, truncated)
}

fn join_bounded_reader(reader: BoundedReader) -> Result<Vec<u8>> {
    let output = match reader.receiver.recv_timeout(READER_SHUTDOWN_TIMEOUT) {
        Ok(output) => output,
        Err(RecvTimeoutError::Timeout) => {
            return Err(Error::InvalidConfig(
                "bounded output reader did not close after process-tree termination".to_owned(),
            ));
        }
        Err(RecvTimeoutError::Disconnected) => {
            return Err(Error::InvalidConfig(
                "bounded output reader disconnected".to_owned(),
            ));
        }
    };
    reader
        .handle
        .join()
        .map_err(|_panic| Error::InvalidConfig("bounded output reader panicked".to_owned()))?;
    output.map_err(|error| invalid_process_error("read child output", &error))
}

fn terminate_and_reap(child: &mut GroupChild) -> Result<ExitStatus> {
    let kill_error = child.kill().err();
    let wait_result = child.wait();
    match (kill_error, wait_result) {
        (None, Ok(status)) => Ok(status),
        (Some(error), Ok(status)) if termination_failure_after_reap_is_benign(&error) => Ok(status),
        (Some(kill_error), Ok(_)) => Err(Error::InvalidConfig(format!(
            "terminate child failed: {kill_error}; child was reaped"
        ))),
        (None, Err(wait_error)) => Err(invalid_process_error("reap child", &wait_error)),
        (Some(kill_error), Err(wait_error)) => Err(Error::InvalidConfig(format!(
            "terminate child failed: {kill_error}; reap child failed: {wait_error}"
        ))),
    }
}

fn reap_after_pipe_failure(child: &mut GroupChild, stream: &str) -> Error {
    let kill_error = child.kill().err();
    let wait_error = child.wait().err();
    let cleanup = match (kill_error, wait_error) {
        (None, None) => "child was terminated and reaped".to_owned(),
        (Some(kill_error), None) if kill_error.kind() == io::ErrorKind::NotFound => {
            "child was already stopped and reaped".to_owned()
        }
        (Some(kill_error), None) => {
            format!("terminate child failed: {kill_error}; child was reaped")
        }
        (None, Some(wait_error)) => format!("reap child failed: {wait_error}"),
        (Some(kill_error), Some(wait_error)) => {
            format!("terminate child failed: {kill_error}; reap child failed: {wait_error}")
        }
    };
    Error::InvalidConfig(format!(
        "bounded child did not expose {stream} pipe; {cleanup}"
    ))
}

fn record_cleanup_error(slot: &mut Option<Error>, error: Error) {
    match slot.take() {
        Some(previous) => {
            *slot = Some(Error::InvalidConfig(format!("{previous}; {error}")));
        }
        None => *slot = Some(error),
    }
}

fn invalid_process_error(operation: &str, error: &io::Error) -> Error {
    Error::InvalidConfig(format!("{operation} failed: {error}"))
}

/// Execute without a shell, capture both streams, then persist the real outcome.
pub fn execute(request: &ExecuteRequest, config: &HarnessConfig) -> Result<RunOutcome> {
    let started_at = timestamp_now()?;
    let output =
        execute_bounded_process(request, ARBITRARY_MAX_WALL_TIME, ARBITRARY_MAX_OUTPUT_BYTES)?;
    let stdout = output.stdout().clone();
    let stderr = if output.stderr().as_str().is_empty()
        && output.termination() != HarnessExecutionTermination::Completed
    {
        HarnessCapturedOutput::from_owned(format!(
            "Harness child process terminated: {}",
            output.termination().as_str()
        ))
    } else {
        output.stderr().clone()
    };
    let exit_code = output.exit_code().map_or(1, ProcessExitCode::get);
    let ended_at = timestamp_now()?;
    record_run(
        &RunInput {
            repo_root: &request.repo_root,
            run_id: request.run_id.clone(),
            tool: request.tool.clone(),
            language: request.language,
            command: request.command.clone(),
            stdout,
            stderr,
            exit_code: ProcessExitCode::new(exit_code),
            crate_name: request.crate_name.clone(),
            package_name: request
                .package_name
                .as_ref()
                .map(|value| CrateName::try_new(value.as_str().to_owned()))
                .transpose()
                .map_err(|error| enforcer_core::error::Error::InvalidConfig(error.to_string()))?,
            domain: request.domain.clone(),
            tags: request.tags.clone(),
            pinned: HarnessPinned::Unpinned,
            started_at,
            ended_at,
        },
        config,
    )
}

fn timestamp_now() -> Result<HarnessTimestamp> {
    let millis = enforcer_core::platform::epoch_millis()?;
    HarnessTimestamp::try_new(enforcer_core::platform::iso8601_utc(millis)).map_err(Into::into)
}
