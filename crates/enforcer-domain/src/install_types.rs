//! Canonical values exchanged by installation, distribution, and release flows.

use crate::boundary::decode_error::DecodeError;
use crate::ids::{GitHubCheckContext, HarnessId};
use crate::paths::RepoRoot;
use crate::severity::Severity;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Stable, sorted harness identities presented when selection fails.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for KnownHarnesses."]
pub struct KnownHarnesses(Vec<HarnessId>);

impl KnownHarnesses {
    #[must_use]
    #[doc = "The from_sorted operation for this canonical domain value."]
    pub fn from_sorted(values: Vec<HarnessId>) -> Self {
        Self(values)
    }

    #[must_use]
    #[doc = "The as_slice operation for this canonical domain value."]
    pub fn as_slice(&self) -> &[HarnessId] {
        &self.0
    }
}

impl std::fmt::Display for KnownHarnesses {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, harness) in self.0.iter().enumerate() {
            if index > 0 {
                formatter.write_str(", ")?;
            }
            formatter.write_str(harness.as_str())?;
        }
        Ok(())
    }
}

/// Validated absolute path of the installed `enforcer` binary.
///
/// BRAND-INVARIANT: this path is absolute and is therefore safe to register
/// in a user-level harness configuration regardless of the caller's cwd.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for InstallBinaryPath."]
pub struct InstallBinaryPath(PathBuf);

impl InstallBinaryPath {
    /// Borrow the validated absolute filesystem path.
    #[must_use]
    #[doc = "The as_path operation for this canonical domain value."]
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl TryFrom<PathBuf> for InstallBinaryPath {
    type Error = DecodeError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        if value.is_absolute() {
            Ok(Self(value))
        } else {
            Err(DecodeError::new(
                "installBinaryPath",
                "must be an absolute filesystem path",
            ))
        }
    }
}

/// Validated absolute path of a harness configuration file owned by an
/// installer adapter.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for InstallTargetPath."]
pub struct InstallTargetPath(PathBuf);

impl InstallTargetPath {
    #[must_use]
    #[doc = "The as_path operation for this canonical domain value."]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    #[must_use]
    #[doc = "The display operation for this canonical domain value."]
    pub fn display(&self) -> std::path::Display<'_> {
        self.0.display()
    }
}

/// Validated absolute directory used as an installer or emitter root.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for InstallRootPath."]
pub struct InstallRootPath(PathBuf);

impl InstallRootPath {
    #[must_use]
    #[doc = "The as_path operation for this canonical domain value."]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    #[must_use]
    #[doc = "The join_target operation for this canonical domain value."]
    pub fn join_target(&self, relative: impl AsRef<Path>) -> InstallTargetPath {
        InstallTargetPath(self.0.join(relative))
    }
}

impl TryFrom<PathBuf> for InstallRootPath {
    type Error = DecodeError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        if value.is_absolute() {
            Ok(Self(value))
        } else {
            Err(DecodeError::new(
                "installRootPath",
                "must be an absolute filesystem path",
            ))
        }
    }
}

impl TryFrom<PathBuf> for InstallTargetPath {
    type Error = DecodeError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        if value.is_absolute() {
            Ok(Self(value))
        } else {
            Err(DecodeError::new(
                "installTargetPath",
                "must be an absolute filesystem path",
            ))
        }
    }
}

/// A release binary resolved to a validated installation target.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ResolvedBinary."]
pub struct ResolvedBinary {
    pub platform: TargetPlatform,
    pub version: ReleaseVersion,
    pub install_path: InstallBinaryPath,
}

/// A released target platform in the Enforcer distribution matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for TargetPlatform."]
pub enum TargetPlatform {
    WindowsX86_64,
    MacX86_64,
    MacAarch64,
    LinuxX86_64Gnu,
    LinuxX86_64Musl,
    LinuxAarch64Gnu,
}

impl TargetPlatform {
    #[must_use]
    #[doc = "The target_triple operation for this canonical domain value."]
    pub fn target_triple(self) -> &'static str {
        match self {
            Self::WindowsX86_64 => "x86_64-pc-windows-msvc",
            Self::MacX86_64 => "x86_64-apple-darwin",
            Self::MacAarch64 => "aarch64-apple-darwin",
            Self::LinuxX86_64Gnu => "x86_64-unknown-linux-gnu",
            Self::LinuxX86_64Musl => "x86_64-unknown-linux-musl",
            Self::LinuxAarch64Gnu => "aarch64-unknown-linux-gnu",
        }
    }

    #[must_use]
    #[doc = "The all operation for this canonical domain value."]
    pub fn all() -> &'static [Self] {
        &[
            Self::WindowsX86_64,
            Self::MacX86_64,
            Self::MacAarch64,
            Self::LinuxX86_64Gnu,
            Self::LinuxX86_64Musl,
            Self::LinuxAarch64Gnu,
        ]
    }

    #[must_use]
    #[doc = "The asset_name operation for this canonical domain value."]
    pub fn asset_name(self, version: &ReleaseVersion) -> String {
        let extension = if matches!(self, Self::WindowsX86_64) {
            "zip"
        } else {
            "tar.gz"
        };
        format!(
            "enforcer-v{}-{}.{extension}",
            version.as_str(),
            self.target_triple()
        )
    }
}

/// Install registry scope. This is deliberately distinct from scan and route scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for InstallScope."]
pub enum InstallScope {
    #[default]
    User,
    Project,
}

/// Rendering mode of an install command result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for InstallOutputMode."]
pub enum InstallOutputMode {
    #[default]
    Human,
    Json,
}

/// Whether an installation command may mutate the filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for DryRun."]
pub enum DryRun {
    #[default]
    Disabled,
    Enabled,
}

/// Canonical execution context for every installation command.
///
/// This is deliberately serde-free: process/CLI wire input is decoded by the
/// installer boundary before any adapter receives it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for InstallRequestContext."]
pub struct InstallRequestContext {
    pub scope: InstallScope,
    pub dry_run: DryRun,
    pub output: InstallOutputMode,
    pub binary_path: InstallBinaryPath,
}

impl InstallRequestContext {
    /// Decode the executable path supplied by a process boundary and apply
    /// the command defaults.
    pub fn try_with_defaults(binary_path: PathBuf) -> Result<Self, DecodeError> {
        Ok(Self {
            scope: InstallScope::default(),
            dry_run: DryRun::Disabled,
            output: InstallOutputMode::default(),
            binary_path: InstallBinaryPath::try_from(binary_path)?,
        })
    }
}

/// Canonical install command consumed by the installer core.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for InstallCommand."]
pub struct InstallCommand {
    pub context: InstallRequestContext,
    pub only_harnesses: Vec<HarnessId>,
}

/// Canonical uninstall command consumed by the installer core.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for UninstallCommand."]
pub struct UninstallCommand {
    pub context: InstallRequestContext,
    pub only_harnesses: Vec<HarnessId>,
}

/// Canonical update command consumed by the installer core.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for UpdateCommand."]
pub struct UpdateCommand {
    pub dry_run: DryRun,
    pub output: InstallOutputMode,
}

/// Canonical doctor command consumed by the installer core.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for DoctorCommand."]
pub struct DoctorCommand {
    pub output: InstallOutputMode,
}

/// Human-readable text emitted by the installer domain.
///
/// BRAND-INVARIANT: report text is valid Unicode and contains no NUL byte,
/// so it can cross a process/JSON boundary without truncation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for InstallReportText."]
pub struct InstallReportText(String);

impl InstallReportText {
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for InstallReportText {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.contains('\0') {
            Err(DecodeError::new(
                "installReportText",
                "must not contain NUL",
            ))
        } else {
            Ok(Self(value))
        }
    }
}

/// Whether a plan will create or update an existing artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ChangeDisposition."]
pub enum ChangeDisposition {
    Create,
    Update,
}

/// Whether an apply or verification operation succeeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for CheckStatus."]
pub enum CheckStatus {
    Passed,
    Failed,
}

/// Whether an emitter may replace an existing file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for OverwriteMode."]
pub enum OverwriteMode {
    PreserveExisting,
    Force,
}

/// Outcome of applying one planned consumer-repository file write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for FileWriteOutcome."]
pub enum FileWriteOutcome {
    Written,
    PreservedExisting,
}

/// Identity of the subject a verification check describes.
///
/// A harness registration and a repository skill asset are different domain
/// subjects; collapsing them into one raw `harness: String` loses that fact.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for CheckSubject."]
pub enum CheckSubject {
    Harness(HarnessId),
    SkillAsset(InstallReportText),
}

/// Canonical planned filesystem change produced by a harness adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for PlannedInstallChange."]
pub struct PlannedInstallChange {
    pub harness: HarnessId,
    pub kind: ArtifactKind,
    pub path: RepoRoot,
    pub description: InstallReportText,
    pub disposition: ChangeDisposition,
}

/// Canonical installation plan consumed by apply orchestration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[doc = "Canonical domain representation for InstallReport."]
pub struct InstallReport {
    pub planned_changes: Vec<PlannedInstallChange>,
    pub warnings: Vec<InstallReportText>,
}

/// Result of applying one planned installer change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for AppliedInstallChange."]
pub struct AppliedInstallChange {
    pub change: PlannedInstallChange,
    pub status: CheckStatus,
    pub backup_path: Option<RepoRoot>,
}

/// Canonical result returned from an adapter apply operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[doc = "Canonical domain representation for ApplyResult."]
pub struct ApplyResult {
    pub applied: Vec<AppliedInstallChange>,
}

/// Canonical filesystem write planned by a consumer-repository emitter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for PlannedFileWrite."]
pub struct PlannedFileWrite {
    pub path: InstallTargetPath,
    pub content: InstallReportText,
}

/// Canonical result of applying one consumer-repository file write.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for AppliedFileWrite."]
pub struct AppliedFileWrite {
    pub planned: PlannedFileWrite,
    pub outcome: FileWriteOutcome,
}

/// Canonical post-install health check.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for InstallVerifyCheck."]
pub struct InstallVerifyCheck {
    pub subject: CheckSubject,
    pub name: InstallReportText,
    pub status: CheckStatus,
    pub detail: InstallReportText,
}

/// Canonical verification result consumed by doctor and CLI orchestration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[doc = "Canonical domain representation for InstallVerifyReport."]
pub struct InstallVerifyReport {
    pub checks: Vec<InstallVerifyCheck>,
}

/// Branded path used by an adapter-provided skill contract.
///
/// BRAND-INVARIANT: this path is a caller-declared asset location, kept as a
/// path value rather than raw text until the filesystem boundary joins it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for SkillAssetPath."]
pub struct SkillAssetPath(PathBuf);

impl SkillAssetPath {
    #[must_use]
    #[doc = "The as_path operation for this canonical domain value."]
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl From<PathBuf> for SkillAssetPath {
    fn from(value: PathBuf) -> Self {
        Self(value)
    }
}

/// One skill asset that must exist for an installation to be healthy.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for SkillAsset."]
pub struct SkillAsset {
    pub name: InstallReportText,
    pub path: SkillAssetPath,
}

/// Plugin manifest field/value contract supplied by an adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for PluginPublishContract."]
pub struct PluginPublishContract {
    pub manifest_path: SkillAssetPath,
    pub field: InstallReportText,
    pub expected_value: InstallReportText,
}

/// Canonical skill asset contract used by the installer core.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[doc = "Canonical domain representation for SkillAssetManifest."]
pub struct SkillAssetManifest {
    pub assets: Vec<SkillAsset>,
    pub plugin_contracts: Vec<PluginPublishContract>,
}

/// Stable install command discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for CommandName."]
pub enum CommandName {
    Install,
    Uninstall,
    Update,
    Doctor,
}

/// Release binary feature set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for BinaryVariant."]
pub enum BinaryVariant {
    Full,
    Lite,
}

impl BinaryVariant {
    #[must_use]
    pub const fn ci_default() -> Self {
        Self::Lite
    }
    #[must_use]
    #[doc = "The all operation for this canonical domain value."]
    pub fn all() -> &'static [Self] {
        &[Self::Full, Self::Lite]
    }
}

/// Result of an individual release smoke run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for SmokeOutcome."]
pub enum SmokeOutcome {
    Passed,
    Failed,
}

/// A typed asset that must pass a smoke run before publication.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ReleaseAsset."]
pub struct ReleaseAsset {
    pub platform: TargetPlatform,
    pub variant: BinaryVariant,
}

/// The typed result of one release asset's smoke run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for SmokeResult."]
pub struct SmokeResult {
    pub asset: ReleaseAsset,
    pub outcome: SmokeOutcome,
}

/// Whether release publication may proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for PublicationStatus."]
pub enum PublicationStatus {
    Approved,
    Blocked,
}

/// The pre-publish decision and any assets that require repair.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ReleaseGateVerdict."]
pub enum ReleaseGateVerdict {
    Publish,
    Blocked { failing: Vec<ReleaseAsset> },
}

impl ReleaseGateVerdict {
    #[must_use]
    #[doc = "The from_failing operation for this canonical domain value."]
    pub fn from_failing(failing: Vec<ReleaseAsset>) -> Self {
        if failing.is_empty() {
            Self::Publish
        } else {
            Self::Blocked { failing }
        }
    }

    #[must_use]
    #[doc = "The publication_status operation for this canonical domain value."]
    pub fn publication_status(&self) -> PublicationStatus {
        match self {
            Self::Publish => PublicationStatus::Approved,
            Self::Blocked { .. } => PublicationStatus::Blocked,
        }
    }
}

/// Linux C library selected by a release target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for Libc."]
pub enum Libc {
    Gnu,
    Musl,
}

/// Artifact category emitted by an installer adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ArtifactKind."]
pub enum ArtifactKind {
    McpRegistration,
    CargoAlias,
    PrecommitHook,
    DoctrineReference,
    HarnessSpecific,
}

/// Validated release tag used by installation and release planning.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ReleaseVersion."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct ReleaseVersion(String);

/// Rejection marker for an empty CI release tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for EmptyReleaseVersion."]
pub struct EmptyReleaseVersion;

impl ReleaseVersion {
    /// Validates the release tag and rejects invalid blank input.
    pub fn try_new(value: String) -> Result<Self, EmptyReleaseVersion> {
        if value.trim().is_empty() {
            Err(EmptyReleaseVersion)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ReleaseVersion {
    type Error = EmptyReleaseVersion;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for HookFlavor."]
pub enum HookFlavor {
    PlainGitHook,
    Husky,
    Lefthook,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for HookEvent."]
pub enum HookEvent {
    SessionStart,
}

/// Claude hook matcher text. An empty matcher deliberately means every session-start source.
///
/// BRAND-INVARIANT: matcher text is preserved exactly; the empty value has the
/// explicit Claude semantics of matching every session-start source.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for SessionStartHookMatcher."]
pub struct SessionStartHookMatcher(String);

impl SessionStartHookMatcher {
    #[doc = "Construct a matcher while preserving Claude's meaningful empty value."]
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        Ok(Self(value))
    }

    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Non-empty command executed by a session-start hook.
///
/// BRAND-INVARIANT: command text is trimmed only for validation and retained
/// verbatim so the registered hook command remains byte-identical.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for SessionStartHookCommand."]
pub struct SessionStartHookCommand(String);

impl SessionStartHookCommand {
    #[doc = "Construct a non-empty hook command."]
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        if value.trim().is_empty() {
            Err(DecodeError::new("command", "must not be empty"))
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SessionStartHookCommand {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

/// Exact context text injected by a session-start hook.
///
/// BRAND-INVARIANT: reminder text must contain non-whitespace content and is
/// then preserved verbatim for deterministic hook rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for SessionStartHookReminderBody."]
pub struct SessionStartHookReminderBody(String);

impl SessionStartHookReminderBody {
    #[doc = "Construct non-empty reminder text."]
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        if value.trim().is_empty() {
            Err(DecodeError::new("reminderBody", "must not be empty"))
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SessionStartHookReminderBody {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for a configured session-start hook."]
pub struct SessionStartHookConfig {
    pub event: HookEvent,
    pub matcher: SessionStartHookMatcher,
    pub command: SessionStartHookCommand,
    pub reminder_body: SessionStartHookReminderBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for HookDecision."]
pub enum HookDecision {
    Allow,
    AllowWithWarning { reason: InstallReportText },
    Deny { reason: InstallReportText },
}
/// Exit state captured from a hook subprocess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for HookExitStatus."]
pub enum HookExitStatus {
    Success,
    Failure(std::num::NonZeroI32),
    Unavailable,
}

/// Captured process outcome consumed by the hook decision classifier.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for HookCheckOutcome."]
pub struct HookCheckOutcome {
    pub exit_status: HookExitStatus,
    pub stdout: InstallReportText,
}

/// Positive hook execution timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: hook timeouts are positive millisecond durations."]
pub struct HookTimeout(std::time::Duration);

impl HookTimeout {
    pub const fn try_from_millis(value: std::num::NonZeroU64) -> Self {
        Self(std::time::Duration::from_millis(value.get()))
    }
}

/// Canonical configuration rendered into a Claude PreToolUse hook entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for PreToolUseHookConfig."]
pub struct PreToolUseHookConfig {
    pub matcher: InstallReportText,
    pub command: InstallBinaryPath,
    pub args: Vec<InstallReportText>,
    pub timeout: HookTimeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ConfigFormat."]
pub enum ConfigFormat {
    JsonMcpServers,
    TomlMcpServers,
    ManagedText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for FindingKind."]
pub enum FindingKind {
    LegacyServerRegistration,
    ConflictingServerRegistration,
    LegacyToolNameLiteral,
    LegacySkillDirPresent,
}

/// Canonical legacy-install residue discovered before transport encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for MigrationFinding."]
pub struct MigrationFinding {
    pub harness: Option<HarnessId>,
    pub path: InstallTargetPath,
    pub kind: FindingKind,
    pub detail: InstallReportText,
}

/// Canonical record of one configuration file rewritten by migration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for RewrittenFile."]
pub struct RewrittenFile {
    pub path: InstallTargetPath,
    pub backup_path: InstallTargetPath,
}

/// Canonical in-process migration outcome before JSON transport encoding.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[doc = "Canonical domain representation for MigrationOutcome."]
pub struct MigrationOutcome {
    pub findings: Vec<MigrationFinding>,
    pub rewritten: Vec<RewrittenFile>,
    pub retired_skill_dir: Option<InstallTargetPath>,
    pub notice: Option<InstallReportText>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for Cap."]
pub enum Cap {
    Bounded(u32),
    Unbounded,
    #[default]
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for Support."]
pub enum Support {
    Yes,
    No,
    #[default]
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for PullRequestRequirement."]
pub enum PullRequestRequirement {
    Required,
    NotRequired,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for UpToDateRequirement."]
pub enum UpToDateRequirement {
    Required,
    NotRequired,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for BypassAllowance."]
pub enum BypassAllowance {
    Denied,
    Allowed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for RequiredChecksHealth."]
pub enum RequiredChecksHealth {
    Passing,
    RedOrPending,
}

/// Required branch-protection gates observed at the GitHub boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for BranchProtectionRequirements."]
pub struct BranchProtectionRequirements {
    up_to_date: UpToDateRequirement,
    pull_requests: PullRequestRequirement,
    required_checks: RequiredChecksHealth,
}

impl BranchProtectionRequirements {
    #[must_use]
    pub const fn new(
        up_to_date: UpToDateRequirement,
        pull_requests: PullRequestRequirement,
        required_checks: RequiredChecksHealth,
    ) -> Self {
        Self {
            up_to_date,
            pull_requests,
            required_checks,
        }
    }
}

/// Branch operations that may bypass protected-branch policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for BranchProtectionBypassPolicy."]
pub struct BranchProtectionBypassPolicy {
    administrator: BypassAllowance,
    force_push: BypassAllowance,
    deletion: BypassAllowance,
}

impl BranchProtectionBypassPolicy {
    #[must_use]
    pub const fn new(
        administrator: BypassAllowance,
        force_push: BypassAllowance,
        deletion: BypassAllowance,
    ) -> Self {
        Self {
            administrator,
            force_push,
            deletion,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ContextRequirement."]
pub enum ContextRequirement {
    Present,
    Missing,
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for RefusalReason."]
pub enum RefusalReason {
    NoRequiredChecks,
    MissingRequiredContext,
    AdministratorBypassAllowed,
    ForcePushAllowed,
    DeletionAllowed,
    UpToDateNotRequired,
    PullRequestNotRequired,
    RequiredChecksNotPassing,
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for Verification."]
pub enum Verification {
    Attested,
    Refused(Vec<RefusalReason>),
}

/// Typed observation produced from a GitHub branch-protection response.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ObservedBranchProtection."]
pub struct ObservedBranchProtection {
    required_contexts: BTreeSet<GitHubCheckContext>,
    up_to_date: UpToDateRequirement,
    pull_requests: PullRequestRequirement,
    administrator_bypass: BypassAllowance,
    force_push: BypassAllowance,
    deletion: BypassAllowance,
    required_checks: RequiredChecksHealth,
}

/// Normalized branch-protection policy required for the protected branch.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for DesiredProtection."]
pub struct DesiredProtection {
    required_contexts: BTreeSet<GitHubCheckContext>,
}

impl DesiredProtection {
    #[must_use]
    #[doc = "The baseline operation for this canonical domain value."]
    pub fn baseline(required_contexts: BTreeSet<GitHubCheckContext>) -> Self {
        Self { required_contexts }
    }

    #[must_use]
    #[doc = "The required_contexts operation for this canonical domain value."]
    pub fn required_contexts(&self) -> &BTreeSet<GitHubCheckContext> {
        &self.required_contexts
    }
}

impl ObservedBranchProtection {
    #[must_use]
    #[doc = "The new operation for this canonical domain value."]
    pub fn new(
        required_contexts: BTreeSet<GitHubCheckContext>,
        requirements: BranchProtectionRequirements,
        bypass_policy: BranchProtectionBypassPolicy,
    ) -> Self {
        Self {
            required_contexts,
            up_to_date: requirements.up_to_date,
            pull_requests: requirements.pull_requests,
            administrator_bypass: bypass_policy.administrator,
            force_push: bypass_policy.force_push,
            deletion: bypass_policy.deletion,
            required_checks: requirements.required_checks,
        }
    }

    #[must_use]
    #[doc = "The context_requirement operation for this canonical domain value."]
    pub fn context_requirement(&self, context: &GitHubCheckContext) -> ContextRequirement {
        if self.required_contexts.contains(context) {
            ContextRequirement::Present
        } else {
            ContextRequirement::Missing
        }
    }

    #[must_use]
    #[doc = "The up_to_date operation for this canonical domain value."]
    pub fn up_to_date(&self) -> UpToDateRequirement {
        self.up_to_date
    }
    #[must_use]
    #[doc = "The pull_requests operation for this canonical domain value."]
    pub fn pull_requests(&self) -> PullRequestRequirement {
        self.pull_requests
    }
    #[must_use]
    #[doc = "The administrator_bypass operation for this canonical domain value."]
    pub fn administrator_bypass(&self) -> BypassAllowance {
        self.administrator_bypass
    }
    #[must_use]
    #[doc = "The force_push operation for this canonical domain value."]
    pub fn force_push(&self) -> BypassAllowance {
        self.force_push
    }
    #[must_use]
    #[doc = "The deletion operation for this canonical domain value."]
    pub fn deletion(&self) -> BypassAllowance {
        self.deletion
    }
    #[must_use]
    #[doc = "The required_checks operation for this canonical domain value."]
    pub fn required_checks(&self) -> RequiredChecksHealth {
        self.required_checks
    }
}

/// One aggregated doctor check with fail-closed severity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for DoctorCheck."]
pub struct DoctorCheck {
    pub check: InstallVerifyCheck,
    pub severity: Severity,
}

impl DoctorCheck {
    #[must_use]
    #[doc = "The from_verify_check operation for this canonical domain value."]
    pub fn from_verify_check(check: InstallVerifyCheck) -> Self {
        let severity = match check.status {
            CheckStatus::Passed => Severity::Info,
            CheckStatus::Failed => Severity::Error,
        };
        Self { check, severity }
    }
}

/// Full aggregated doctor report across every adapter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[doc = "Canonical domain representation for DoctorReport."]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
}
