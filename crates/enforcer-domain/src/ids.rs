//! Branded identifier newtypes. Each validates on construction and has no
//! public raw-string constructor; parse at the boundary, use the brand
//! everywhere after.

use crate::boundary::decode_error::DecodeError;

/// Declare a branded string newtype with validation and serde boundary wiring.
macro_rules! branded_string {
    ($(#[$doc:meta])* $name:ident, $field_path:literal, $validate:path) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
            serde::Serialize, serde::Deserialize, ts_rs::TS,
        )]
        #[serde(try_from = "String", into = "String")]
        #[ts(type = "string")]
        pub struct $name(String);

        impl $name {
            /// View the validated inner value.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = DecodeError;

            fn try_from(raw: String) -> Result<Self, DecodeError> {
                $validate(&raw)?;
                Ok(Self(raw))
            }
        }

        impl std::str::FromStr for $name {
            type Err = DecodeError;

            fn from_str(raw: &str) -> Result<Self, DecodeError> {
                // ALLOC-JUSTIFICATION: each brand owns the validated value so it
                // remains valid across event and async transport boundaries.
                Self::try_from(raw.to_owned())
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

/// Closed catalogue of harness identities compiled into the installer.
///
/// Unlike a user-supplied harness selector, these variants cannot contain an
/// invalid identifier. The process boundary still validates dynamic selector
/// text with [`HarnessId::try_from`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for BuiltInHarness."]
pub enum BuiltInHarness {
    Aider,
    Antigravity,
    Claude,
    Codex,
    Cursor,
    Gemini,
    KiloCode,
    Kiro,
    OpenCode,
    Windsurf,
    Zed,
}

impl BuiltInHarness {
    /// Produce the canonical branded identifier for a compile-time adapter.
    #[must_use]
    #[doc = "The id operation for this canonical domain value."]
    pub fn id(self) -> HarnessId {
        let value = match self {
            Self::Aider => "aider",
            Self::Antigravity => "antigravity",
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::Gemini => "gemini",
            Self::KiloCode => "kilocode",
            Self::Kiro => "kiro",
            Self::OpenCode => "opencode",
            Self::Windsurf => "windsurf",
            Self::Zed => "zed",
        };
        // ALLOC-JUSTIFICATION: HarnessId owns its canonical value across the
        // adapter registry and cannot borrow this compile-time literal.
        HarnessId(value.to_owned())
    }
}

/// Closed catalogue of Rust source validator rule identities.
///
/// These values are compiled into the validator set and cannot contain an
/// invalid runtime identifier. Dynamic rule IDs still validate through
/// [`RuleId::try_from`] at their input boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for BuiltInRustRule."]
pub enum BuiltInRustRule {
    AllowReason,
    ArchMainThin,
    BorrowParam,
    CastLossy,
    DocPublicItem,
    ErrContext,
    ErrMainExitcode,
    ErrMsgStyle,
    ErrNonExhaustive,
    ErrSentinel,
    ErrorHandling,
    FmtCapturedIdent,
    FnComplexity,
    FnMaxParams,
    LayerDomain,
    MatchNoWildcard,
    McpStdout,
    NoReexports,
    NoUtilsModule,
    SafetyComment,
}

impl BuiltInRustRule {
    /// Produce the canonical branded identifier for a compiled Rust validator.
    #[must_use]
    #[doc = "The id operation for this canonical domain value."]
    pub fn id(self) -> RuleId {
        let value = match self {
            Self::AllowReason => "RUST-ALLOW-1.1",
            Self::ArchMainThin => "RUST-ARCH-1.1",
            Self::BorrowParam => "RUST-BORROW-1.1",
            Self::CastLossy => "RUST-CAST-NO-AS-LOSSY",
            Self::DocPublicItem => "RUST-DOC-PUBLIC-ITEM",
            Self::ErrContext => "RUST-ERR-CONTEXT",
            Self::ErrMainExitcode => "RUST-ERR-MAIN-EXITCODE",
            Self::ErrMsgStyle => "RUST-ERR-MSG-STYLE",
            Self::ErrNonExhaustive => "RUST-ERR-NONEXHAUSTIVE",
            Self::ErrSentinel => "RUST-ERR-SENTINEL",
            Self::ErrorHandling => "T1-RUSTERR.1",
            Self::FmtCapturedIdent => "RUST-FMT-CAPTURED-IDENT",
            Self::FnComplexity => "RUST-FN-COMPLEXITY",
            Self::FnMaxParams => "RUST-FN-MAX-PARAMS",
            Self::LayerDomain => "RUST-LAYER-1.1",
            Self::MatchNoWildcard => "RUST-MATCH-NO-WILDCARD",
            Self::McpStdout => "RUST-MCP-1.1",
            Self::NoReexports => "T1-NOREEXPORT.1",
            Self::NoUtilsModule => "RUST-NO-UTILS-MODULE",
            Self::SafetyComment => "RUST-SAFETY-COMMENT",
        };
        // ALLOC-JUSTIFICATION: RuleId owns the canonical value across validator
        // construction and finding production.
        RuleId(value.to_owned())
    }
}

/// Closed catalogue of infrastructure-as-code validator rule identities.
///
/// These variants are the canonical built-in identity set shared by the IaC
/// registry, validators, and completeness proofs. Dynamic catalog text still
/// validates through [`RuleId::try_from`] at its JSON boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for BuiltInIacRule."]
pub enum BuiltInIacRule {
    TerraformS3Encryption,
    TerraformOpenIngress,
    TerraformHardcodedSecrets,
    CloudFormationPublicAccess,
    CloudFormationWildcardIam,
    TerraformProviderVersion,
    TerraformRemoteStateEncryption,
    KubernetesPrivilegedContainer,
}

impl BuiltInIacRule {
    /// Every built-in IaC rule in canonical catalog order.
    pub const ALL: [Self; 8] = [
        Self::TerraformS3Encryption,
        Self::TerraformOpenIngress,
        Self::TerraformHardcodedSecrets,
        Self::CloudFormationPublicAccess,
        Self::CloudFormationWildcardIam,
        Self::TerraformProviderVersion,
        Self::TerraformRemoteStateEncryption,
        Self::KubernetesPrivilegedContainer,
    ];

    /// Produce the canonical branded identifier for a compiled IaC validator.
    #[must_use]
    #[doc = "The id operation for this canonical domain value."]
    pub fn id(self) -> RuleId {
        let value = match self {
            Self::TerraformS3Encryption => "IAC-1.1",
            Self::TerraformOpenIngress => "IAC-1.2",
            Self::TerraformHardcodedSecrets => "IAC-1.3",
            Self::CloudFormationPublicAccess => "IAC-1.4",
            Self::CloudFormationWildcardIam => "IAC-1.5",
            Self::TerraformProviderVersion => "IAC-1.6",
            Self::TerraformRemoteStateEncryption => "IAC-1.7",
            Self::KubernetesPrivilegedContainer => "IAC-1.8",
        };
        // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
        RuleId(value.to_owned())
    }

    /// Produce the canonical finding title for this built-in rule.
    pub fn finding_title(self) -> Result<crate::findings::FindingTitle, DecodeError> {
        let value = match self {
            Self::TerraformS3Encryption => {
                "Terraform S3 buckets must enable server-side encryption"
            }
            Self::TerraformOpenIngress => {
                "Terraform security groups must not allow unrestricted ingress"
            }
            Self::TerraformHardcodedSecrets => {
                "Terraform resources must not hardcode secrets or credentials"
            }
            Self::CloudFormationPublicAccess => {
                "CloudFormation S3 buckets must block public access"
            }
            Self::CloudFormationWildcardIam => {
                "CloudFormation IAM policies must not grant wildcard action+resource"
            }
            Self::TerraformProviderVersion => "Terraform provider blocks must pin an exact version",
            Self::TerraformRemoteStateEncryption => {
                "Terraform remote state backends must enable encryption"
            }
            Self::KubernetesPrivilegedContainer => "Kubernetes containers must not run privileged",
        };
        value.parse()
    }
}

/// Closed catalogue of Kubernetes manifest validator rule identities.
///
/// These variants are the canonical built-in identity set shared by the
/// Kubernetes registry, validators, fixture proofs, and completeness checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for BuiltInK8sRule."]
pub enum BuiltInK8sRule {
    PrivilegedContainer,
    RunAsRoot,
    PrivilegeEscalation,
    WritableRootFilesystem,
    WildcardRbacVerbs,
    WildcardRbacResources,
    MissingResourceLimits,
    MissingMemoryRequests,
    HostNetwork,
    HostProcessNamespace,
}

impl BuiltInK8sRule {
    /// Every built-in Kubernetes rule in canonical family order.
    pub const ALL: [Self; 10] = [
        Self::PrivilegedContainer,
        Self::RunAsRoot,
        Self::PrivilegeEscalation,
        Self::WritableRootFilesystem,
        Self::WildcardRbacVerbs,
        Self::WildcardRbacResources,
        Self::MissingResourceLimits,
        Self::MissingMemoryRequests,
        Self::HostNetwork,
        Self::HostProcessNamespace,
    ];

    /// Produce the canonical branded identifier for a compiled validator.
    #[must_use]
    #[doc = "The id operation for this canonical domain value."]
    pub fn id(self) -> RuleId {
        let value = match self {
            Self::PrivilegedContainer => "K8S-1.1",
            Self::RunAsRoot => "K8S-1.2",
            Self::PrivilegeEscalation => "K8S-1.3",
            Self::WritableRootFilesystem => "K8S-1.4",
            Self::WildcardRbacVerbs => "K8S-2.1",
            Self::WildcardRbacResources => "K8S-2.2",
            Self::MissingResourceLimits => "K8S-3.1",
            Self::MissingMemoryRequests => "K8S-3.2",
            Self::HostNetwork => "K8S-4.1",
            Self::HostProcessNamespace => "K8S-4.2",
        };
        // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
        RuleId(value.to_owned())
    }

    /// Produce the canonical finding title for this built-in rule.
    pub fn finding_title(self) -> Result<crate::findings::FindingTitle, DecodeError> {
        let value = match self {
            Self::PrivilegedContainer => "Privileged containers are forbidden",
            Self::RunAsRoot => "Containers must not run as root",
            Self::PrivilegeEscalation => "Privilege escalation must be disabled",
            Self::WritableRootFilesystem => "Root filesystem must be read-only",
            Self::WildcardRbacVerbs => "Wildcard RBAC verbs are forbidden",
            Self::WildcardRbacResources => "Wildcard RBAC resources are forbidden",
            Self::MissingResourceLimits => "Containers must declare resource limits",
            Self::MissingMemoryRequests => "Containers must declare memory requests",
            Self::HostNetwork => "Host network access is forbidden",
            Self::HostProcessNamespace => "Host PID/IPC namespace access is forbidden",
        };
        value.parse()
    }
}

/// Closed catalogue of built-in CFML validator rule identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for BuiltInCfmlRule."]
pub enum BuiltInCfmlRule {
    LayeredArchitecture,
    WireBoxDi,
    ServiceScopeRead,
    ApplicationScopeLookup,
    CflintAdvisory,
    TypedThrow,
    EmptyCatch,
    SqlInjection,
    XssOutput,
    HardcodedSecret,
    InformationDisclosure,
    MissingVarScope,
    ArgumentsScope,
    TypedSignature,
    BannedDynamicEval,
    LogboxDiagnostics,
    ScriptFirstComponent,
    FilenameConvention,
    PrivateByDefault,
    UnusedLocal,
    CflintrcHardGate,
    PinnedDependency,
    TestboxBaseSpec,
    CfformatCiStep,
    CoverageFloor,
}

impl BuiltInCfmlRule {
    /// Every fixture-provable built-in CFML rule in registry order.
    pub const ALL: [Self; 25] = [
        Self::LayeredArchitecture,
        Self::WireBoxDi,
        Self::ServiceScopeRead,
        Self::ApplicationScopeLookup,
        Self::CflintAdvisory,
        Self::TypedThrow,
        Self::EmptyCatch,
        Self::SqlInjection,
        Self::XssOutput,
        Self::HardcodedSecret,
        Self::InformationDisclosure,
        Self::MissingVarScope,
        Self::ArgumentsScope,
        Self::TypedSignature,
        Self::BannedDynamicEval,
        Self::LogboxDiagnostics,
        Self::ScriptFirstComponent,
        Self::FilenameConvention,
        Self::PrivateByDefault,
        Self::UnusedLocal,
        Self::CflintrcHardGate,
        Self::PinnedDependency,
        Self::TestboxBaseSpec,
        Self::CfformatCiStep,
        Self::CoverageFloor,
    ];

    /// Produce the canonical branded identifier for a compiled CFML validator.
    #[must_use]
    #[doc = "The id operation for this canonical domain value."]
    pub fn id(self) -> RuleId {
        let value = match self {
            Self::LayeredArchitecture => "CF-ARCH-1.1",
            Self::WireBoxDi => "CF-DI-1.1",
            Self::ServiceScopeRead => "CF-ARCH-3.1",
            Self::ApplicationScopeLookup => "CF-DI-1.2",
            Self::CflintAdvisory => "CFML-CPLX-2.1",
            Self::TypedThrow => "CF-ERR-1.1",
            Self::EmptyCatch => "CF-ERR-2.1",
            Self::SqlInjection => "CF-SEC-1.1",
            Self::XssOutput => "CF-SEC-2.1",
            Self::HardcodedSecret => "CF-SEC-4.1",
            Self::InformationDisclosure => "CF-SEC-3.1",
            Self::MissingVarScope => "CF-STYLE-1.1",
            Self::ArgumentsScope => "CFML-VAR-1.2",
            Self::TypedSignature => "CF-STYLE-2.1",
            Self::BannedDynamicEval => "CF-STYLE-4.1",
            Self::LogboxDiagnostics => "CF-LOG-1.1",
            Self::ScriptFirstComponent => "CF-STYLE-3.1",
            Self::FilenameConvention => "CF-STYLE-5.1",
            Self::PrivateByDefault => "CF-STYLE-2.2",
            Self::UnusedLocal => "CFML-DEAD-1.1",
            Self::CflintrcHardGate => "CF-TOOL-1.1",
            Self::PinnedDependency => "CF-DEP-1.1",
            Self::TestboxBaseSpec => "CF-TEST-1.1",
            Self::CfformatCiStep => "CF-TOOL-2.1",
            Self::CoverageFloor => "CF-CI-2.1",
        };
        // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
        RuleId(value.to_owned())
    }

    /// Produce the canonical finding title for this built-in rule.
    pub fn finding_title(self) -> Result<crate::findings::FindingTitle, DecodeError> {
        let value = match self {
            Self::LayeredArchitecture => "CFML layered architecture boundary violation",
            Self::WireBoxDi => "CFML collaborator bypasses WireBox dependency injection",
            Self::ServiceScopeRead => "CFML service reads a request scope directly",
            Self::ApplicationScopeLookup => "CFML service uses application-scope lookup",
            Self::CflintAdvisory => "CFLint advisory result",
            Self::TypedThrow => "CFML throw lacks a namespaced type",
            Self::EmptyCatch => "CFML catch block swallows an exception",
            Self::SqlInjection => "CFML query contains unparameterized input",
            Self::XssOutput => "CFML output contains an unencoded value",
            Self::HardcodedSecret => "CFML source contains a hardcoded secret",
            Self::InformationDisclosure => "CFML output discloses internal error detail",
            Self::MissingVarScope => "CFML local assignment lacks an explicit scope",
            Self::ArgumentsScope => "CFML argument reference lacks arguments scope",
            Self::TypedSignature => "CFML public signature lacks explicit types",
            Self::BannedDynamicEval => "CFML source uses banned dynamic evaluation",
            Self::LogboxDiagnostics => "CFML diagnostics bypass LogBox",
            Self::ScriptFirstComponent => "CFML component uses tag syntax",
            Self::FilenameConvention => "CFML component filename violates convention",
            Self::PrivateByDefault => "CFML method is public without an API marker",
            Self::UnusedLocal => "CFML local variable is unused",
            Self::CflintrcHardGate => "CFLint hard-gate configuration is invalid",
            Self::PinnedDependency => "CFML dependency is not exactly pinned",
            Self::TestboxBaseSpec => "TestBox specification does not extend BaseSpec",
            Self::CfformatCiStep => "CFML CI lacks a format check",
            Self::CoverageFloor => "TestBox coverage floor is below policy",
        };
        value.parse()
    }
}

/// Closed catalogue of built-in Dart validator rule identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for BuiltInDartRule."]
pub enum BuiltInDartRule {
    LayerBoundary,
    UncheckedBangOrCast,
    FreezedEntity,
    RawExceptionThrow,
    ExceptionRenderedToUser,
    SnakeCaseFilename,
    UngroupedImports,
    StringConcatenation,
    EmbeddedSensitiveLiteral,
    InsecureStorage,
    PlaintextHttp,
    DisabledTls,
    BarePrint,
    UnguardedDebugOutput,
    NoChangeNotifier,
    LegacyStateNotifierProvider,
    RefReadInBuild,
    DetailMutatesListProvider,
    DataFetchInInitState,
    StrictAnalysisOptions,
    CiRunsAnalyze,
    CiRunsFormatCheck,
    UnpinnedDependency,
    HandEditedGeneratedFile,
    TypedDto,
    SilentFallback,
    FormStateMap,
    OnePublicWidgetPerFile,
    SuperKeyFirstParam,
    ListViewBuilderRequired,
    SetStateInBuild,
    HardcodedColor,
    ImperativeNavigation,
    HardcodedUserString,
}

impl BuiltInDartRule {
    /// Every built-in Dart validator rule in registry order.
    pub const ALL: [Self; 34] = [
        Self::LayerBoundary,
        Self::UncheckedBangOrCast,
        Self::FreezedEntity,
        Self::RawExceptionThrow,
        Self::ExceptionRenderedToUser,
        Self::SnakeCaseFilename,
        Self::UngroupedImports,
        Self::StringConcatenation,
        Self::EmbeddedSensitiveLiteral,
        Self::InsecureStorage,
        Self::PlaintextHttp,
        Self::DisabledTls,
        Self::BarePrint,
        Self::UnguardedDebugOutput,
        Self::NoChangeNotifier,
        Self::LegacyStateNotifierProvider,
        Self::RefReadInBuild,
        Self::DetailMutatesListProvider,
        Self::DataFetchInInitState,
        Self::StrictAnalysisOptions,
        Self::CiRunsAnalyze,
        Self::CiRunsFormatCheck,
        Self::UnpinnedDependency,
        Self::HandEditedGeneratedFile,
        Self::TypedDto,
        Self::SilentFallback,
        Self::FormStateMap,
        Self::OnePublicWidgetPerFile,
        Self::SuperKeyFirstParam,
        Self::ListViewBuilderRequired,
        Self::SetStateInBuild,
        Self::HardcodedColor,
        Self::ImperativeNavigation,
        Self::HardcodedUserString,
    ];

    /// Produce the canonical branded identifier for a compiled Dart validator.
    #[must_use]
    #[doc = "The id operation for this canonical domain value."]
    pub fn id(self) -> RuleId {
        let value = match self {
            Self::LayerBoundary => "DART-ARCH-1.1",
            Self::UncheckedBangOrCast => "DART-BANG-1.1",
            Self::FreezedEntity => "DART-FREEZED-1.1",
            Self::RawExceptionThrow => "DART-ERR-1.1",
            Self::ExceptionRenderedToUser => "DART-ERR-2.1",
            Self::SnakeCaseFilename => "DART-NAME-1.1",
            Self::UngroupedImports => "DART-IMP-1.1",
            Self::StringConcatenation => "DART-STYLE-2.1",
            Self::EmbeddedSensitiveLiteral => "DART-SEC-1.1",
            Self::InsecureStorage => "DART-SEC-1.2",
            Self::PlaintextHttp => "DART-SEC-1.3",
            Self::DisabledTls => "DART-SEC-1.4",
            Self::BarePrint => "DART-SEC-1.5",
            Self::UnguardedDebugOutput => "DART-SEC-1.6",
            Self::NoChangeNotifier => "DART-STATE-1.1",
            Self::LegacyStateNotifierProvider => "DART-RIVERPOD-1.1",
            Self::RefReadInBuild => "DART-STATE-1.2",
            Self::DetailMutatesListProvider => "DART-STATE-1.3",
            Self::DataFetchInInitState => "DART-INITSTATE-1.1",
            Self::StrictAnalysisOptions => "DART-TOOL-1.1",
            Self::CiRunsAnalyze => "DART-TOOL-1.2",
            Self::CiRunsFormatCheck => "DART-TOOL-1.3",
            Self::UnpinnedDependency => "DART-DEP-1.1",
            Self::HandEditedGeneratedFile => "DART-GEN-1.1",
            Self::TypedDto => "DART-TYPE-1.1",
            Self::SilentFallback => "DART-FALLBACK-1.1",
            Self::FormStateMap => "DART-FORMMAP-1.1",
            Self::OnePublicWidgetPerFile => "DART-COMP-1.1",
            Self::SuperKeyFirstParam => "DART-COMP-1.2",
            Self::ListViewBuilderRequired => "DART-PERF-1.1",
            Self::SetStateInBuild => "DART-PERF-2.1",
            Self::HardcodedColor => "DART-COLOR-1.1",
            Self::ImperativeNavigation => "DART-NAV-2.1",
            Self::HardcodedUserString => "DART-L10N-2.1",
        };
        // ALLOC-JUSTIFICATION: RuleId owns the canonical Dart rule identity across findings.
        RuleId(value.to_owned())
    }

    /// Produce the canonical finding title for this built-in rule.
    pub fn finding_title(self) -> Result<crate::findings::FindingTitle, DecodeError> {
        let value = match self {
            Self::LayerBoundary => "Dart layer boundary crossed",
            Self::UncheckedBangOrCast => "Unchecked null assertion or unguarded cast",
            Self::FreezedEntity => "Mutable entity should be an immutable Freezed class",
            Self::RawExceptionThrow => "Raw exception thrown instead of a typed failure",
            Self::ExceptionRenderedToUser => "Raw exception rendered to the user",
            Self::SnakeCaseFilename => "Filename does not match its widget in snake case",
            Self::UngroupedImports => "Ungrouped or interleaved import order",
            Self::StringConcatenation => "String concatenation used instead of interpolation",
            Self::EmbeddedSensitiveLiteral => "Dart source contains an embedded sensitive literal",
            Self::InsecureStorage => "Sensitive data written to insecure storage",
            Self::PlaintextHttp => "Dart source contains a plaintext HTTP URI",
            Self::DisabledTls => "TLS certificate verification is disabled",
            Self::BarePrint => "Bare print call used for diagnostics",
            Self::UnguardedDebugOutput => "Debug output is not guarded",
            Self::NoChangeNotifier => "ChangeNotifier used in new code",
            Self::LegacyStateNotifierProvider => "Legacy StateNotifierProvider used",
            Self::RefReadInBuild => "Riverpod ref.read used inside build",
            Self::DetailMutatesListProvider => "Detail widget mutates a list provider directly",
            Self::DataFetchInInitState => "Data fetch kicked off from initState",
            Self::StrictAnalysisOptions => "Dart analysis options are missing or not strict",
            Self::CiRunsAnalyze => "Dart CI does not run analyze with fatal infos",
            Self::CiRunsFormatCheck => "Dart CI does not run the format check",
            Self::UnpinnedDependency => "Dart dependency version is not pinned",
            Self::HandEditedGeneratedFile => "Generated Dart file lacks its generated marker",
            Self::TypedDto => "Dart DTO or signature uses an untyped dynamic value",
            Self::SilentFallback => "Required Dart field has a silent default fallback",
            Self::FormStateMap => "Dart form state is carried as an untyped map",
            Self::OnePublicWidgetPerFile => "Dart file declares more than one public widget",
            Self::SuperKeyFirstParam => "Widget constructor lacks super key as its first parameter",
            Self::ListViewBuilderRequired => {
                "Dynamic collection uses ListView children instead of builder"
            }
            Self::SetStateInBuild => "Dart setState is called inside build",
            Self::HardcodedColor => "Widget contains a hardcoded color literal",
            Self::ImperativeNavigation => "Widget uses imperative navigation",
            Self::HardcodedUserString => "Widget contains a hardcoded user-facing string",
        };
        value.parse()
    }
}

fn validate_rule_id(raw: &str) -> Result<(), DecodeError> {
    // e.g. `RR-6.1`, `DEP-1.1`, `SEC-2.3`: uppercase alnum family prefix,
    // then dash-separated alnum/dot segments.
    let mut parts = raw.split('-');
    let Some(prefix) = parts.next() else {
        return Err(DecodeError::new(
            "ruleId",
            "expected `PREFIX-segment[...]` with uppercase alnum prefix (e.g. `RR-6.1`)",
        ));
    };
    let prefix_ok = !prefix.is_empty()
        && prefix
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
        && prefix
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    let mut rest_count = 0usize;
    let mut rest_ok = true;
    for segment in parts {
        rest_count += 1;
        if segment.is_empty()
            || !segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.')
        {
            rest_ok = false;
        }
    }
    if prefix_ok && rest_count > 0 && rest_ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "ruleId",
            "expected `PREFIX-segment[...]` with uppercase alnum prefix (e.g. `RR-6.1`)",
        ))
    }
}

fn validate_hub_name(raw: &str) -> Result<(), DecodeError> {
    let ok = !raw.is_empty()
        && raw.len() <= 128
        && raw
            .chars()
            // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !raw.starts_with('-')
        && !raw.ends_with('-');
    if ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "hubName",
            "expected lowercase kebab-case (e.g. `enforcer-rust-build`)",
        ))
    }
}

fn validate_lane_id(raw: &str) -> Result<(), DecodeError> {
    let ok = !raw.is_empty()
        && raw.len() <= 64
        && raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        && !raw.starts_with('-')
        && !raw.ends_with('-');
    if ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "laneId",
            "expected lowercase alnum/dash/underscore (e.g. `arc-02`)",
        ))
    }
}

fn validate_harness_id(raw: &str) -> Result<(), DecodeError> {
    let ok = !raw.is_empty()
        && raw.len() <= 64
        && raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "harnessId",
            "expected lowercase kebab-case (e.g. `claude`, `codex`, `kilocode`)",
        ))
    }
}

fn validate_correlation_like(raw: &str) -> Result<(), DecodeError> {
    let ok = !raw.is_empty()
        && raw.len() <= 128
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "correlationId",
            "expected 1..=128 chars of alnum/dash/underscore/dot",
        ))
    }
}

fn validate_threat_id(raw: &str) -> Result<(), DecodeError> {
    // MITRE ATT&CK technique (`T1059` / `T1059.001`), CWE (`CWE-79`), or
    // OWASP Top-10 slot (`A03:2021`).
    let mitre = raw.strip_prefix('T').is_some_and(|rest| {
        let mut halves = rest.splitn(2, '.');
        let Some(base) = halves.next() else {
            return false;
        };
        let sub = halves.next();
        base.len() == 4
            && base.chars().all(|c| c.is_ascii_digit())
            && sub.is_none_or(|s| s.len() == 3 && s.chars().all(|c| c.is_ascii_digit()))
    });
    let cwe = raw
        .strip_prefix("CWE-")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()));
    let owasp = raw.strip_prefix('A').is_some_and(|rest| {
        let mut halves = rest.splitn(2, ':');
        let Some(slot) = halves.next() else {
            return false;
        };
        let year = halves.next();
        slot.len() == 2
            && slot.chars().all(|c| c.is_ascii_digit())
            && year.is_some_and(|y| y.len() == 4 && y.chars().all(|c| c.is_ascii_digit()))
    });
    if mitre || cwe || owasp {
        Ok(())
    } else {
        Err(DecodeError::new(
            "threatId",
            "expected MITRE `T####[.###]`, `CWE-#`, or OWASP `A##:####`",
        ))
    }
}

fn validate_github_check_context(raw: &str) -> Result<(), DecodeError> {
    let valid = !raw.is_empty()
        && raw.len() <= 512
        && raw
            .chars()
            .all(|character| !character.is_control() && character != '\n' && character != '\r');
    if valid {
        Ok(())
    } else {
        Err(DecodeError::new(
            "githubCheckContext",
            "expected 1..=512 printable characters without line breaks",
        ))
    }
}

fn validate_github_branch_name(raw: &str) -> Result<(), DecodeError> {
    let valid = !raw.is_empty()
        && raw.len() <= 255
        && !raw.starts_with('-')
        && !raw.ends_with('/')
        && !raw.contains("..")
        && raw.chars().all(|character| {
            !character.is_control()
                && !character.is_whitespace()
                && character != '~'
                && character != '^'
                && character != ':'
                && character != '?'
                && character != '*'
                && character != '['
                && character != '\\'
        });
    if valid {
        Ok(())
    } else {
        Err(DecodeError::new(
            "githubBranchName",
            "expected a non-empty GitHub branch name without control, whitespace, or Git ref special characters",
        ))
    }
}

branded_string!(
    /// Branded rule identifier (e.g. `RR-6.1`, `DEP-1.1`).
    RuleId,
    "ruleId",
    validate_rule_id
);

branded_string!(
    /// Branded coordination hub name (e.g. `enforcer-rust-build`).
    HubName,
    "hubName",
    validate_hub_name
);

branded_string!(
    /// Branded coordination lane id (e.g. `arc-02`).
    LaneId,
    "laneId",
    validate_lane_id
);

branded_string!(
    /// Branded agent-harness identifier (e.g. `claude`, `codex`, `kilocode`).
    HarnessId,
    "harnessId",
    validate_harness_id
);

branded_string!(
    /// Branded correlation id stitching one logical flow across crates.
    CorrelationId,
    "correlationId",
    validate_correlation_like
);

branded_string!(
    /// Branded causation id linking an event to the event that caused it.
    CausationId,
    "causationId",
    validate_correlation_like
);

branded_string!(
    /// Branded threat identifier: MITRE ATT&CK, CWE, or OWASP Top-10.
    ThreatId,
    "threatId",
    validate_threat_id
);

branded_string!(
    /// Branded GitHub status-check context, validated before protection policy comparison.
    GitHubCheckContext,
    "githubCheckContext",
    validate_github_check_context
);

branded_string!(
    /// Branded GitHub branch name used by branch-protection policy and reports.
    GitHubBranchName,
    "githubBranchName",
    validate_github_branch_name
);

/// Closed catalogue of Python and FastAPI validator rule identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Closed catalogue of Python and FastAPI validator rule identities."]
pub enum BuiltInPythonRule {
    Py1Rule1,
    Py1Rule2,
    Py1Rule3,
    Py2Rule1,
    Py3Rule1,
    Py3Rule2,
    Py4Rule1,
    Py4Rule2,
    Py4Rule3,
    Py4Rule4,
    Py4Rule5,
    Py4Rule6,
    Py4Rule7,
    Py4Rule8,
    Py4Rule9,
    Py4Rule10,
    Py4Rule11,
    Py4Rule12,
    Py4Rule13,
    Py4Rule14,
    Py4Rule15,
    Py4Rule16,
    Py4Rule17,
    Py4Rule18,
    Py4Rule19,
    Py4Rule20,
    Py4Rule21,
    Py4Rule22,
    Py4Rule23,
    Py4Rule24,
    Py4Rule25,
    Py4Rule26,
    Py4Rule27,
    Py4Rule28,
    Py4Rule29,
    Py4Rule30,
    Py4Rule31,
    Py4Rule32,
    Py4Rule33,
    Py4Rule34,
    Py4Rule35,
    Py5Rule1,
    Py5Rule2,
    Py5Rule3,
    Py5Rule4,
    Py5Rule5,
    Py5Rule6,
    Py5Rule7,
    Py5Rule8,
    Py5Rule9,
    Py5Rule10,
    Py6Rule1,
    Py6Rule2,
    Py6Rule3,
    Py6Rule4,
    Py6Rule5,
    Py6Rule6,
    Py6Rule7,
    Py6Rule8,
    Py6Rule9,
    Py6Rule10,
    Pyfa1Rule1,
    Pyfa2Rule1,
    Pyfa3Rule1,
    Pyfa4Rule1,
    Pyfa5Rule1,
    Pyfa6Rule1,
    Pyfa7Rule1,
    #[doc = "Canonical public API owned by this domain module."]
    Pyfa8Rule1,
    Pyfa9Rule1,
    Pyfa10Rule1,
    Pyfa11Rule1,
    Pyfa12Rule1,
    Pyfa12Rule2,
    Pyfa12Rule3,
}

impl BuiltInPythonRule {
    /// Produce the canonical branded identifier for a compiled Python validator.
    #[must_use]
    #[doc = "Return the canonical branded Python rule identifier."]
    pub fn id(self) -> RuleId {
        let value = match self {
            Self::Py1Rule1 => "PY-1.1",
            Self::Py1Rule2 => "PY-1.2",
            Self::Py1Rule3 => "PY-1.3",
            Self::Py2Rule1 => "PY-2.1",
            Self::Py3Rule1 => "PY-3.1",
            Self::Py3Rule2 => "PY-3.2",
            Self::Py4Rule1 => "PY-4.1",
            Self::Py4Rule2 => "PY-4.2",
            Self::Py4Rule3 => "PY-4.3",
            Self::Py4Rule4 => "PY-4.4",
            Self::Py4Rule5 => "PY-4.5",
            Self::Py4Rule6 => "PY-4.6",
            Self::Py4Rule7 => "PY-4.7",
            Self::Py4Rule8 => "PY-4.8",
            Self::Py4Rule9 => "PY-4.9",
            Self::Py4Rule10 => "PY-4.10",
            Self::Py4Rule11 => "PY-4.11",
            Self::Py4Rule12 => "PY-4.12",
            Self::Py4Rule13 => "PY-4.13",
            Self::Py4Rule14 => "PY-4.14",
            Self::Py4Rule15 => "PY-4.15",
            Self::Py4Rule16 => "PY-4.16",
            Self::Py4Rule17 => "PY-4.17",
            Self::Py4Rule18 => "PY-4.18",
            Self::Py4Rule19 => "PY-4.19",
            Self::Py4Rule20 => "PY-4.20",
            Self::Py4Rule21 => "PY-4.21",
            Self::Py4Rule22 => "PY-4.22",
            Self::Py4Rule23 => "PY-4.23",
            Self::Py4Rule24 => "PY-4.24",
            Self::Py4Rule25 => "PY-4.25",
            Self::Py4Rule26 => "PY-4.26",
            Self::Py4Rule27 => "PY-4.27",
            Self::Py4Rule28 => "PY-4.28",
            Self::Py4Rule29 => "PY-4.29",
            Self::Py4Rule30 => "PY-4.30",
            Self::Py4Rule31 => "PY-4.31",
            Self::Py4Rule32 => "PY-4.32",
            Self::Py4Rule33 => "PY-4.33",
            Self::Py4Rule34 => "PY-4.34",
            Self::Py4Rule35 => "PY-4.35",
            Self::Py5Rule1 => "PY-5.1",
            Self::Py5Rule2 => "PY-5.2",
            Self::Py5Rule3 => "PY-5.3",
            Self::Py5Rule4 => "PY-5.4",
            Self::Py5Rule5 => "PY-5.5",
            Self::Py5Rule6 => "PY-5.6",
            Self::Py5Rule7 => "PY-5.7",
            Self::Py5Rule8 => "PY-5.8",
            Self::Py5Rule9 => "PY-5.9",
            Self::Py5Rule10 => "PY-5.10",
            Self::Py6Rule1 => "PY-6.1",
            Self::Py6Rule2 => "PY-6.2",
            Self::Py6Rule3 => "PY-6.3",
            Self::Py6Rule4 => "PY-6.4",
            Self::Py6Rule5 => "PY-6.5",
            Self::Py6Rule6 => "PY-6.6",
            Self::Py6Rule7 => "PY-6.7",
            Self::Py6Rule8 => "PY-6.8",
            Self::Py6Rule9 => "PY-6.9",
            Self::Py6Rule10 => "PY-6.10",
            Self::Pyfa1Rule1 => "PYFA-1.1",
            Self::Pyfa2Rule1 => "PYFA-2.1",
            Self::Pyfa3Rule1 => "PYFA-3.1",
            Self::Pyfa4Rule1 => "PYFA-4.1",
            Self::Pyfa5Rule1 => "PYFA-5.1",
            Self::Pyfa6Rule1 => "PYFA-6.1",
            Self::Pyfa7Rule1 => "PYFA-7.1",
            Self::Pyfa8Rule1 => "PYFA-8.1",
            Self::Pyfa9Rule1 => "PYFA-9.1",
            Self::Pyfa10Rule1 => "PYFA-10.1",
            Self::Pyfa11Rule1 => "PYFA-11.1",
            Self::Pyfa12Rule1 => "PYFA-12.1",
            Self::Pyfa12Rule2 => "PYFA-12.2",
            Self::Pyfa12Rule3 => "PYFA-12.3",
        };
        // ALLOC-JUSTIFICATION: RuleId owns the canonical value across validator
        // construction and finding production.
        RuleId(
            // ALLOC-JUSTIFICATION: RuleId owns the selected built-in catalogue identifier.
            value.to_owned(),
        )
    }
}

/// Closed catalogue of literal-risk validator identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Closed catalogue of literal-risk validator identities."]
pub enum BuiltInLiteralRule {
    Lit1Rule1,
    Lit1Rule2,
    Lit1Rule3,
    Lit1Rule4,
    Lit1Rule5,
    Lit1Rule6,
    Lit1Rule7,
    Lit1Rule8,
    Lit1Rule9,
    Lit2Rule1,
}

impl BuiltInLiteralRule {
    /// Produce the canonical branded identifier for a compiled literal validator.
    #[must_use]
    #[doc = "Return the canonical branded literal-risk rule identifier."]
    pub fn id(self) -> RuleId {
        let value = match self {
            Self::Lit1Rule1 => "LIT-1.1",
            Self::Lit1Rule2 => "LIT-1.2",
            Self::Lit1Rule3 => "LIT-1.3",
            Self::Lit1Rule4 => "LIT-1.4",
            Self::Lit1Rule5 => "LIT-1.5",
            Self::Lit1Rule6 => "LIT-1.6",
            Self::Lit1Rule7 => "LIT-1.7",
            Self::Lit1Rule8 => "LIT-1.8",
            Self::Lit1Rule9 => "LIT-1.9",
            Self::Lit2Rule1 => "LIT-2.1",
        };
        RuleId(
            // ALLOC-JUSTIFICATION: RuleId owns the selected built-in catalogue identifier.
            value.to_owned(),
        )
    }
}

/// Closed catalogue of built-in security and cybersecurity validator identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Closed catalogue of built-in security and cybersecurity validator identities."]
pub enum BuiltInSecurityRule {
    Sec1Rule1,
    Sec1Rule2,
    Sec2Rule1,
    Sec2Rule2,
    Sec2Rule3,
    Sec2Rule4,
    Sec2Rule5,
    Sec2Rule6,
    Sec2Rule7,
    Sec2Rule8,
    Sec2Rule9,
    Sec2Rule10,
    Sec2Rule11,
    Sec2Rule12,
    Sec2Rule13,
    Sec2Rule14,
    Sec2Rule15,
    Sec2Rule16,
    Sec2Rule17,
    Sec2Rule18,
    Sec2Rule19,
    Sec2Rule20,
    CyberAuthJwt,
    CyberAws,
    CyberAzureBlobPublic,
    CyberAzureHttps,
    CyberAzureTls12,
    CyberCmdInject,
    CyberCookieSecure,
    CyberCors,
    CyberDependencyConfusion,
    CyberDeserialize,
    CyberDocker,
    #[doc = "Canonical public API owned by this domain module."]
    CyberDockerDaemon,
    CyberGcp,
    CyberGithubActions,
    CyberHeadersCsp,
    CyberHeadersHsts,
    CyberIacIamWildcard,
    CyberIacS3Sse,
    CyberIacSgSsh,
    CyberK8sPod,
    CyberK8sRbac,
    CyberMassAssign,
    CyberFilelessMalware,
    CyberFilelessTelemetry,
    CyberFilelessReport,
    CyberMcpPoison,
    CyberNetTls,
    CyberNosqlInject,
    CyberOauth,
    CyberPathTraversal,
    #[doc = "Canonical public API owned by this domain module."]
    CyberPrototypePollution,
    CyberProviderSecret,
    CyberSqliSource,
    CyberSsrf,
    CyberSsti,
    CyberTlsVerify,
    CyberTypeJuggle,
    CyberWafSqli,
    CyberWeakCrypto,
    CyberWebsocket,
}

impl BuiltInSecurityRule {
    /// Produce the canonical branded identifier for a compiled security validator.
    #[must_use]
    #[doc = "Return the canonical branded security rule identifier."]
    pub fn id(self) -> RuleId {
        let value = match self {
            Self::Sec1Rule1 => "SEC-1.1",
            Self::Sec1Rule2 => "SEC-1.2",
            Self::Sec2Rule1 => "SEC-2.1",
            Self::Sec2Rule2 => "SEC-2.2",
            Self::Sec2Rule3 => "SEC-2.3",
            Self::Sec2Rule4 => "SEC-2.4",
            Self::Sec2Rule5 => "SEC-2.5",
            Self::Sec2Rule6 => "SEC-2.6",
            Self::Sec2Rule7 => "SEC-2.7",
            Self::Sec2Rule8 => "SEC-2.8",
            Self::Sec2Rule9 => "SEC-2.9",
            Self::Sec2Rule10 => "SEC-2.10",
            Self::Sec2Rule11 => "SEC-2.11",
            Self::Sec2Rule12 => "SEC-2.12",
            Self::Sec2Rule13 => "SEC-2.13",
            Self::Sec2Rule14 => "SEC-2.14",
            Self::Sec2Rule15 => "SEC-2.15",
            Self::Sec2Rule16 => "SEC-2.16",
            Self::Sec2Rule17 => "SEC-2.17",
            Self::Sec2Rule18 => "SEC-2.18",
            Self::Sec2Rule19 => "SEC-2.19",
            Self::Sec2Rule20 => "SEC-2.20",
            Self::CyberAuthJwt => "CYBER-AUTH-JWT.1",
            Self::CyberAws => "CYBER-AWS.1",
            Self::CyberAzureBlobPublic => "CYBER-AZURE-BLOB-PUBLIC.1",
            Self::CyberAzureHttps => "CYBER-AZURE-HTTPS.1",
            Self::CyberAzureTls12 => "CYBER-AZURE-TLS12.1",
            Self::CyberCmdInject => "CYBER-CMD-INJECT.1",
            Self::CyberCookieSecure => "CYBER-COOKIE-SECURE.1",
            Self::CyberCors => "CYBER-CORS.1",
            Self::CyberDependencyConfusion => "CYBER-DEPCONFUSION.1",
            Self::CyberDeserialize => "CYBER-DESERIALIZE.1",
            Self::CyberDocker => "CYBER-DOCKER.1",
            Self::CyberDockerDaemon => "CYBER-DOCKER-DAEMON.1",
            Self::CyberGcp => "CYBER-GCP.1",
            Self::CyberGithubActions => "CYBER-GHA.1",
            Self::CyberHeadersCsp => "CYBER-HEADERS-CSP.1",
            Self::CyberHeadersHsts => "CYBER-HEADERS-HSTS.1",
            Self::CyberIacIamWildcard => "CYBER-IAC-IAM-WILDCARD.1",
            Self::CyberIacS3Sse => "CYBER-IAC-S3-SSE.1",
            Self::CyberIacSgSsh => "CYBER-IAC-SG-SSH.1",
            Self::CyberK8sPod => "CYBER-K8S-POD.1",
            Self::CyberK8sRbac => "CYBER-K8S-RBAC.1",
            Self::CyberMassAssign => "CYBER-MASS-ASSIGN.1",
            Self::CyberFilelessMalware => "CYBER-FILELESS-MALWARE.1",
            Self::CyberFilelessTelemetry => "CYBER-FILELESS-TELEMETRY.1",
            Self::CyberFilelessReport => "CYBER-FILELESS-REPORT.1",
            Self::CyberMcpPoison => "CYBER-MCP-POISON.1",
            Self::CyberNetTls => "CYBER-TLS.1",
            Self::CyberNosqlInject => "CYBER-NOSQL-INJECT.1",
            Self::CyberOauth => "CYBER-OAUTH.1",
            Self::CyberPathTraversal => "CYBER-PATH-TRAVERSAL.1",
            Self::CyberPrototypePollution => "CYBER-PROTO-POLLUTION.1",
            Self::CyberProviderSecret => "CYBER-SECRET.1",
            Self::CyberSqliSource => "CYBER-SQLI-SOURCE.1",
            Self::CyberSsrf => "CYBER-SSRF.1",
            Self::CyberSsti => "CYBER-SSTI.1",
            Self::CyberTlsVerify => "CYBER-TLS-VERIFY.1",
            Self::CyberTypeJuggle => "CYBER-TYPE-JUGGLE.1",
            Self::CyberWafSqli => "CYBER-WAF-SQLI.1",
            Self::CyberWeakCrypto => "CYBER-WEAK-CRYPTO.1",
            Self::CyberWebsocket => "CYBER-WEBSOCKET.1",
        };
        // ALLOC-JUSTIFICATION: RuleId owns the canonical value across validator
        // construction and finding production.
        RuleId(value.to_owned())
    }
}
