//! Canonical scan request value types.

use std::path::PathBuf;

use crate::boundary::decode_error::DecodeError;
use crate::boundary::validation::ValidationSource;
use crate::findings::ScanScope;
use crate::paths::{RelPath, RepoRoot};
use crate::severity::Tier;

macro_rules! literal_path_slice_target {
    () => {
        [PathBuf]
    };
}

macro_rules! literal_static_str_slice_target {
    () => {
        [&'static str]
    };
}

macro_rules! literal_text_target {
    () => {
        str
    };
}

/// Filesystem root selected for a literal-risk scan.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for LiteralScanRoot."]
#[doc = "BRAND-INVARIANT: the owned path is decoded at the CLI or API boundary before scanning."]
pub struct LiteralScanRoot(PathBuf);

impl LiteralScanRoot {
    /// Borrow the selected filesystem root.
    #[must_use]
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Default for LiteralScanRoot {
    fn default() -> Self {
        Self(PathBuf::from("."))
    }
}

impl From<PathBuf> for LiteralScanRoot {
    fn from(value: PathBuf) -> Self {
        Self(value)
    }
}

impl AsRef<std::path::Path> for LiteralScanRoot {
    fn as_ref(&self) -> &std::path::Path {
        self.as_path()
    }
}

impl std::ops::Deref for LiteralScanRoot {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

/// Explicit filesystem targets selected for a literal-risk scan.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for LiteralScanPaths."]
#[doc = "BRAND-INVARIANT: raw path arguments enter only through a scan boundary."]
pub struct LiteralScanPaths(Vec<PathBuf>);

impl LiteralScanPaths {
    /// Borrow the selected target paths.
    #[must_use]
    pub fn as_slice(&self) -> &[PathBuf] {
        &self.0
    }

    /// Add one path decoded by a scan request boundary.
    pub fn push(&mut self, value: PathBuf) {
        self.0.push(value);
    }
}

impl From<Vec<PathBuf>> for LiteralScanPaths {
    fn from(value: Vec<PathBuf>) -> Self {
        Self(value)
    }
}

impl AsRef<[PathBuf]> for LiteralScanPaths {
    fn as_ref(&self) -> &[PathBuf] {
        self.as_slice()
    }
}

impl std::ops::Deref for LiteralScanPaths {
    type Target = literal_path_slice_target!();

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

/// Explicit enabled/disabled state for literal-scan options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for LiteralScanToggle."]
pub enum LiteralScanToggle {
    Disabled,
    Enabled,
}

impl LiteralScanToggle {
    /// Whether this option is enabled.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

impl From<bool> for LiteralScanToggle {
    fn from(value: bool) -> Self {
        if value {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

/// Curated static language name stored by the literal lexer registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for LiteralLanguageName."]
#[doc = "BRAND-INVARIANT: values originate from the curated static registry and use canonical lowercase names."]
pub struct LiteralLanguageName(&'static str);

impl LiteralLanguageName {
    /// Construct a curated static language name.
    #[must_use]
    pub const fn from_static(value: &'static str) -> Self {
        Self(value)
    }

    /// Borrow the canonical language name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for LiteralLanguageName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl From<LiteralLanguageName> for LiteralLanguageId {
    fn from(value: LiteralLanguageName) -> Self {
        // ALLOC-JUSTIFICATION: the canonical language ID owns the registry spelling beyond this conversion.
        Self(value.0.to_owned())
    }
}

/// Curated file-extension aliases for one literal lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for LiteralExtensionSet."]
#[doc = "BRAND-INVARIANT: entries are static registry suffixes without filesystem ownership."]
pub struct LiteralExtensionSet(&'static [&'static str]);

impl LiteralExtensionSet {
    /// Construct a static extension set.
    #[must_use]
    pub const fn from_static(value: &'static [&'static str]) -> Self {
        Self(value)
    }

    /// Borrow the extension entries.
    #[must_use]
    pub const fn as_slice(self) -> &'static [&'static str] {
        self.0
    }
}

impl AsRef<[&'static str]> for LiteralExtensionSet {
    fn as_ref(&self) -> &[&'static str] {
        self.0
    }
}

impl std::ops::Deref for LiteralExtensionSet {
    type Target = literal_static_str_slice_target!();

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// Curated basename aliases for one literal lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for LiteralBasenameSet."]
#[doc = "BRAND-INVARIANT: entries are static registry basenames without path separators."]
pub struct LiteralBasenameSet(&'static [&'static str]);

impl LiteralBasenameSet {
    /// Construct a static basename set.
    #[must_use]
    pub const fn from_static(value: &'static [&'static str]) -> Self {
        Self(value)
    }

    /// Borrow the basename entries.
    #[must_use]
    pub const fn as_slice(self) -> &'static [&'static str] {
        self.0
    }
}

impl AsRef<[&'static str]> for LiteralBasenameSet {
    fn as_ref(&self) -> &[&'static str] {
        self.0
    }
}

impl std::ops::Deref for LiteralBasenameSet {
    type Target = literal_static_str_slice_target!();

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// Supported non-default string syntaxes for one literal lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for LiteralStringSyntaxProfile."]
#[doc = "BRAND-INVARIANT: the bit set contains only the three declared syntax capabilities."]
pub struct LiteralStringSyntaxProfile(u8);

impl LiteralStringSyntaxProfile {
    pub const NONE: Self = Self(0);
    pub const SINGLE_QUOTE: u8 = 1;
    pub const BACKTICK: u8 = 2;
    pub const TRIPLE_DOUBLE: u8 = 4;

    /// Construct a registry profile from its curated capability bits.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & 0b111)
    }

    #[must_use]
    pub const fn supports_single_quote(self) -> bool {
        self.0 & Self::SINGLE_QUOTE != 0
    }

    #[must_use]
    pub const fn supports_backtick(self) -> bool {
        self.0 & Self::BACKTICK != 0
    }

    #[must_use]
    pub const fn supports_triple_double(self) -> bool {
        self.0 & Self::TRIPLE_DOUBLE != 0
    }
}

macro_rules! literal_owned_text {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        #[doc = "BRAND-INVARIANT: scanner-owned text is created at lexer, classifier, or rendering boundaries."]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn from_owned(value: String) -> Self {
                Self(value)
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::from_owned(value)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl std::ops::Deref for $name {
            type Target = literal_text_target!();

            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

literal_owned_text!(
    /// Extracted literal text retained for classification.
    LiteralSourceText
);
literal_owned_text!(
    /// Source-line context retained for a literal candidate.
    LiteralSourceContext
);
literal_owned_text!(
    /// Redacted or truncated literal preview emitted in a finding.
    LiteralPreview
);
literal_owned_text!(
    /// Stable finding hash rendered for a literal occurrence.
    LiteralFindingHash
);
literal_owned_text!(
    /// Human-readable classifier reason emitted in a finding.
    LiteralFindingReason
);
literal_owned_text!(
    /// Human-readable remediation emitted in a finding.
    LiteralFindingSuggestion
);

/// One normalized scanner-relative finding path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for LiteralFindingPath."]
#[doc = "BRAND-INVARIANT: the path is normalized by scanner discovery before construction."]
pub struct LiteralFindingPath(String);

impl LiteralFindingPath {
    /// Construct a normalized scanner finding path.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        if value.trim().is_empty() || value.contains('\\') {
            return Err(DecodeError::new(
                "literalFindingPath",
                "must be a non-empty normalized path",
            ));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for LiteralFindingPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for LiteralFindingPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One-based source line retained by literal scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for LiteralSourceLine."]
pub enum LiteralSourceLine {
    /// A boundary could not provide a source line.
    Unknown,
    /// A validated one-based source line.
    OneBased(std::num::NonZeroUsize),
}

impl LiteralSourceLine {
    #[must_use]
    pub const fn from_one_based(value: usize) -> Self {
        match std::num::NonZeroUsize::new(value) {
            Some(value) => Self::OneBased(value),
            None => Self::Unknown,
        }
    }

    #[must_use]
    pub const fn get(self) -> usize {
        match self {
            Self::Unknown => 0,
            Self::OneBased(value) => value.get(),
        }
    }
}

impl std::fmt::Display for LiteralSourceLine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(formatter)
    }
}

/// One-based source column retained by literal scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for LiteralSourceColumn."]
pub enum LiteralSourceColumn {
    /// A boundary could not provide a source column.
    Unknown,
    /// A validated one-based source column.
    OneBased(std::num::NonZeroUsize),
}

impl LiteralSourceColumn {
    #[must_use]
    pub const fn from_one_based(value: usize) -> Self {
        match std::num::NonZeroUsize::new(value) {
            Some(value) => Self::OneBased(value),
            None => Self::Unknown,
        }
    }

    #[must_use]
    pub const fn get(self) -> usize {
        match self {
            Self::Unknown => 0,
            Self::OneBased(value) => value.get(),
        }
    }
}

impl std::fmt::Display for LiteralSourceColumn {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(formatter)
    }
}

/// Count produced by literal discovery, classification, or reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[doc = "Canonical domain representation for LiteralScanCount."]
#[doc = "BRAND-INVARIANT: count values are produced by scanner collection lengths and increments."]
pub struct LiteralScanCount(usize);

impl LiteralScanCount {
    #[must_use]
    pub const fn from_count(value: usize) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl PartialEq<usize> for LiteralScanCount {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<usize> for LiteralScanCount {
    fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl From<usize> for LiteralScanCount {
    fn from(value: usize) -> Self {
        Self::from_count(value)
    }
}

impl std::ops::AddAssign<usize> for LiteralScanCount {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl std::ops::Add for LiteralScanCount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::iter::Sum for LiteralScanCount {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(Self::get).sum())
    }
}

impl std::fmt::Display for LiteralScanCount {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Elapsed literal-scan duration in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[doc = "Canonical domain representation for LiteralScanDurationMillis."]
#[doc = "BRAND-INVARIANT: value is derived from a monotonic elapsed duration."]
pub struct LiteralScanDurationMillis(u128);

impl LiteralScanDurationMillis {
    #[must_use]
    pub const fn from_millis(value: u128) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u128 {
        self.0
    }
}

impl std::fmt::Display for LiteralScanDurationMillis {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Whether a literal-risk finding blocks the scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for LiteralFindingDisposition."]
pub enum LiteralFindingDisposition {
    Advisory,
    Blocking,
}

impl LiteralFindingDisposition {
    #[must_use]
    pub const fn is_blocking(self) -> bool {
        matches!(self, Self::Blocking)
    }
}

impl From<bool> for LiteralFindingDisposition {
    fn from(value: bool) -> Self {
        if value {
            Self::Blocking
        } else {
            Self::Advisory
        }
    }
}

/// Output encoding selected for a literal-risk scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for LiteralScanOutputFormat."]
pub enum LiteralScanOutputFormat {
    Json,
    JsonLines,
    Human,
}

/// Lexer family selected for a registered literal-scan language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for LiteralLanguageFamily."]
pub enum LiteralLanguageFamily {
    Rust,
    TypeScript,
    Python,
    CLike,
    HashComment,
    Shell,
    Lisp,
    Markup,
    CommonText,
    Sql,
    Fallback,
}

/// Command selected at the literal-scanner CLI boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for LiteralScanCommand."]
pub enum LiteralScanCommand {
    Scan,
    Languages,
    Explain,
}

/// Validated language identifier accepted by literal-scan filtering and reports.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for LiteralLanguageId."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct LiteralLanguageId(String);

impl LiteralLanguageId {
    /// Construct a language id, rejecting invalid casing and punctuation.
    pub fn try_new(value: &str) -> Result<Self, DecodeError> {
        let value = value.trim();
        if value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(DecodeError::new(
                "literalLanguageId",
                "must contain lowercase ASCII letters, digits, or hyphens",
            ));
        }
        Ok(Self(String::from(value)))
    }

    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for LiteralLanguageId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for LiteralLanguageId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for LiteralLanguageId {
    type Err = DecodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_new(value)
    }
}

/// Fully rendered literal-scan report text at a presentation boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for LiteralRenderedReport."]
#[doc = "BRAND-INVARIANT: empty text is valid report output; raw storage remains presentation-owned and private."]
pub struct LiteralRenderedReport(String);

impl LiteralRenderedReport {
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for LiteralRenderedReport {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for LiteralRenderedReport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One rendered JSON-lines record emitted by literal-scan.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for LiteralRenderedLine."]
#[doc = "BRAND-INVARIANT: empty text is valid rendered-line output; raw storage remains presentation-owned and private."]
pub struct LiteralRenderedLine(String);

impl LiteralRenderedLine {
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for LiteralRenderedLine {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for LiteralRenderedLine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Stable non-cryptographic identity used to group literal text and file locations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for LiteralStableHash."]
pub struct LiteralStableHash {
    #[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
    first: u64,
    #[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
    second: u64,
}

impl LiteralStableHash {
    /// Compute the stable identity for validated source text.
    #[must_use]
    pub fn of_source(source: ValidationSource<'_>) -> Self {
        let mut first = 0xcbf29ce484222325_u64;
        let mut second = 0x84222325cbf29ce4_u64;
        for byte in source.as_str().as_bytes() {
            first ^= u64::from(*byte);
            first = first.wrapping_mul(0x100000001b3);
            second ^= u64::from(*byte).rotate_left(1);
            second = second.wrapping_mul(0x100000001b3);
        }
        Self { first, second }
    }
}

impl std::fmt::Display for LiteralStableHash {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:016x}{:016x}", self.first, self.second)
    }
}

/// A validated literal-risk score or score threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for LiteralRiskScore."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct LiteralRiskScore(u8);

impl LiteralRiskScore {
    pub const ZERO: Self = Self(0);
    pub const HIGH_RISK_THRESHOLD: Self = Self(60);

    /// Construct a validated score decoded by a scanner boundary.
    pub fn try_new(value: u8) -> Result<Self, DecodeError> {
        if value <= 100 {
            Ok(Self(value))
        } else {
            Err(DecodeError::new(
                "literalRiskScore",
                "must be between 0 and 100",
            ))
        }
    }

    /// Return the validated numeric score.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl std::fmt::Display for LiteralRiskScore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl PartialEq<u8> for LiteralRiskScore {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u8> for LiteralRiskScore {
    fn partial_cmp(&self, other: &u8) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl TryFrom<std::num::NonZeroU8> for LiteralRiskScore {
    type Error = DecodeError;

    fn try_from(value: std::num::NonZeroU8) -> Result<Self, Self::Error> {
        Self::try_new(value.get())
    }
}

impl From<LiteralRiskScore> for u8 {
    fn from(value: LiteralRiskScore) -> Self {
        value.0
    }
}

impl Default for LiteralRiskScore {
    fn default() -> Self {
        Self(40)
    }
}

/// A non-zero upper bound for bytes read from one literal-scan target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for LiteralFileByteLimit."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct LiteralFileByteLimit(u64);

impl LiteralFileByteLimit {
    #[doc = "Construct a positive byte limit decoded at a boundary."]
    pub const fn try_from_nonzero(value: std::num::NonZeroU64) -> Self {
        Self(value.get())
    }
}

impl From<LiteralFileByteLimit> for u64 {
    fn from(value: LiteralFileByteLimit) -> Self {
        value.0
    }
}

impl Default for LiteralFileByteLimit {
    fn default() -> Self {
        Self(2 * 1024 * 1024)
    }
}

/// Closed confidence band derived from a literal-risk score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for LiteralConfidence."]
pub enum LiteralConfidence {
    Low,
    Medium,
    High,
}

impl LiteralConfidence {
    /// Stable wire spelling used by reports and transport adapters.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

/// Semantic role of a file evaluated by literal-risk classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for LiteralFileRole."]
pub enum LiteralFileRole {
    Domain,
    Boundary,
    Config,
    Test,
    Generated,
    Tooling,
    Script,
    Docs,
    CommonText,
    Unknown,
}

impl LiteralFileRole {
    /// Stable wire spelling used by reports and transport adapters.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Domain => "domain",
            Self::Boundary => "boundary",
            Self::Config => "config",
            Self::Test => "test",
            Self::Generated => "generated",
            Self::Tooling => "tooling",
            Self::Script => "script",
            Self::Docs => "docs",
            Self::CommonText => "common-text",
            Self::Unknown => "unknown",
        }
    }
}

/// Syntax family for one extracted literal candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for LiteralSyntaxKind."]
pub enum LiteralSyntaxKind {
    Normal,
    Raw,
    Byte,
    Template,
    InterpolatedTemplate,
    Triple,
    FString,
    ImportSpecifier,
    DocString,
    Attribute,
}

impl LiteralSyntaxKind {
    /// Stable wire spelling used by reports and transport adapters.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Raw => "raw",
            Self::Byte => "byte",
            Self::Template => "template",
            Self::InterpolatedTemplate => "interpolated-template",
            Self::Triple => "triple",
            Self::FString => "f-string",
            Self::ImportSpecifier => "import-specifier",
            Self::DocString => "doc-string",
            Self::Attribute => "attribute",
        }
    }
}

/// Canonical semantic category assigned to a literal-risk finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for LiteralRiskCategory."]
pub enum LiteralRiskCategory {
    SecretLike,
    EventOrCommandName,
    RouteOrUrl,
    ProtocolHeaderOrMedia,
    IdOrKeyName,
    StateOrStatus,
    RawJsonBlob,
    SqlFragment,
    ShellFragment,
    MagicStringComparison,
    RepeatedLiteral,
    HumanMessage,
    TestFixture,
    ImportSpecifier,
    SchemaOwnerLiteral,
    UnknownLiteral,
}

impl LiteralRiskCategory {
    /// Stable wire spelling used by reports, filters, and transport adapters.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::SecretLike => "secret-like",
            Self::EventOrCommandName => "event-or-command-name",
            Self::RouteOrUrl => "route-or-url",
            Self::ProtocolHeaderOrMedia => "protocol-header-or-media",
            Self::IdOrKeyName => "id-or-key-name",
            Self::StateOrStatus => "state-or-status",
            Self::RawJsonBlob => "raw-json-blob",
            Self::SqlFragment => "sql-fragment",
            Self::ShellFragment => "shell-fragment",
            Self::MagicStringComparison => "magic-string-comparison",
            Self::RepeatedLiteral => "repeated-literal",
            Self::HumanMessage => "human-message",
            Self::TestFixture => "test-fixture",
            Self::ImportSpecifier => "import-specifier",
            Self::SchemaOwnerLiteral => "schema-owner-literal",
            Self::UnknownLiteral => "unknown-literal",
        }
    }
}

impl std::str::FromStr for LiteralRiskCategory {
    type Err = DecodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "secret-like" => Ok(Self::SecretLike),
            "event-or-command-name" => Ok(Self::EventOrCommandName),
            "route-or-url" => Ok(Self::RouteOrUrl),
            "protocol-header-or-media" => Ok(Self::ProtocolHeaderOrMedia),
            "id-or-key-name" => Ok(Self::IdOrKeyName),
            "state-or-status" => Ok(Self::StateOrStatus),
            "raw-json-blob" => Ok(Self::RawJsonBlob),
            "sql-fragment" => Ok(Self::SqlFragment),
            "shell-fragment" => Ok(Self::ShellFragment),
            "magic-string-comparison" => Ok(Self::MagicStringComparison),
            "repeated-literal" => Ok(Self::RepeatedLiteral),
            "human-message" => Ok(Self::HumanMessage),
            "test-fixture" => Ok(Self::TestFixture),
            "import-specifier" => Ok(Self::ImportSpecifier),
            "schema-owner-literal" => Ok(Self::SchemaOwnerLiteral),
            "unknown-literal" => Ok(Self::UnknownLiteral),
            _ => Err(DecodeError::new(
                "literalRiskCategory",
                "must be a known literal-risk category",
            )),
        }
    }
}

/// A validated git revision expression used as one endpoint of a scan diff.
// SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
#[doc = "Canonical domain representation for CommitRef."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct CommitRef(String);

impl CommitRef {
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for CommitRef {
    type Err = DecodeError;
    fn from_str(raw: &str) -> Result<Self, DecodeError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DecodeError::new("scope.commitRef", "must not be empty"));
        }
        // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
        Ok(Self(trimmed.to_owned()))
    }
}

impl TryFrom<String> for CommitRef {
    type Error = DecodeError;
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        raw.parse()
    }
}

impl From<CommitRef> for String {
    fn from(value: CommitRef) -> Self {
        value.0
    }
}

impl<'de> serde::Deserialize<'de> for CommitRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::try_from(raw).map_err(serde::de::Error::custom)
    }
}

/// One mutually exclusive input scope for a scan.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ScopeRequest."]
pub enum ScopeRequest {
    Paths(Vec<PathBuf>),
    Diff { base: CommitRef, head: CommitRef },
    All,
}

/// A scope normalized against one repository root.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ResolvedScope."]
pub struct ResolvedScope {
    pub kind: ScanScope,
    pub repo_root: RepoRoot,
    pub explicit_paths: Vec<RelPath>,
    pub diff_range: Option<(CommitRef, CommitRef)>,
}

/// A non-empty reason explaining why a scan target was not run.
// SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, ts_rs::TS)]
#[serde(try_from = "String", into = "String")]
#[doc = "Canonical domain representation for SkipReason."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct SkipReason(String);

impl SkipReason {
    #[doc = "The try_new operation for this canonical domain value."]
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        if value.trim().is_empty() {
            return Err(DecodeError::new(
                "skip_reason",
                "a skip reason must not be empty - a skip with no reason is a silent skip",
            ));
        }
        Ok(Self(value))
    }
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for SkipReason {
    type Error = DecodeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}
impl From<SkipReason> for String {
    fn from(value: SkipReason) -> Self {
        value.0
    }
}
impl std::str::FromStr for SkipReason {
    type Err = DecodeError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
        Self::try_from(value.to_owned())
    }
}
impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for SkipReason {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::try_from(raw).map_err(serde::de::Error::custom)
    }
}

/// Positive number of validators that executed for one scan target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ts_rs::TS)]
#[doc = "BRAND-INVARIANT: a ran outcome always records at least one validator."]
pub struct ScanValidatorCount(std::num::NonZeroUsize);

impl ScanValidatorCount {
    pub const fn try_new(value: std::num::NonZeroUsize) -> Self {
        Self(value)
    }
}

impl serde::Serialize for ScanValidatorCount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // CAST-JUSTIFICATION: usize widens losslessly to the serde u64 wire count on supported targets.
        serializer.serialize_u64(self.0.get() as u64)
    }
}

impl<'de> serde::Deserialize<'de> for ScanValidatorCount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = <usize as serde::Deserialize>::deserialize(deserializer)?;
        std::num::NonZeroUsize::new(raw)
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom("validator count must be positive"))
    }
}

/// Number of scan targets represented by one coverage measurement.
///
/// Zero is valid: an empty selection and a fully skipped scan are both
/// meaningful states that the anti-silent-skip gate must distinguish.
/// ZERO-VALID: zero explicitly represents a scan selection with no targets.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    ts_rs::TS,
)]
#[serde(transparent)]
#[ts(type = "number")]
#[doc = "Canonical domain representation for ScanTargetCount."]
#[doc = "BRAND-INVARIANT: scan coverage counts cannot be confused with unrelated usize values."]
pub struct ScanTargetCount(usize);

impl ScanTargetCount {
    /// Brand a zero-or-greater target count.
    #[must_use]
    pub const fn from_count(value: usize) -> Self {
        Self(value)
    }

    /// Read the target count at a presentation or collection boundary.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }

    /// Record one additional target in this coverage category.
    pub fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }

    /// Combine two disjoint target categories.
    #[must_use]
    pub const fn plus(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Whether this coverage category contains no targets.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl From<usize> for ScanTargetCount {
    fn from(value: usize) -> Self {
        Self::from_count(value)
    }
}

/// Number of persisted violation occurrences in a scan baseline.
/// ZERO-VALID: zero explicitly represents an empty persisted baseline.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for BaselineEntryCount."]
#[doc = "BRAND-INVARIANT: baseline occurrence counts cannot be confused with unrelated usize values."]
pub struct BaselineEntryCount(usize);

impl BaselineEntryCount {
    /// Brand a zero-or-greater baseline occurrence count.
    #[must_use]
    pub const fn from_count(value: usize) -> Self {
        Self(value)
    }

    /// Read the count at a presentation or collection boundary.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl From<usize> for BaselineEntryCount {
    fn from(value: usize) -> Self {
        Self::from_count(value)
    }
}

/// Canonical registration of one onboarded scan repository.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ProjectRegistration."]
pub struct ProjectRegistration {
    pub version: crate::telemetry_types::RecordSchemaVersion,
    pub project_id: crate::hashes::Sha256,
    pub repo_root: RepoRoot,
}

/// The explicit result for each scan candidate.
// SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[doc = "Canonical domain representation for Outcome."]
pub enum Outcome {
    Ran { validator_count: ScanValidatorCount },
    Skipped { reason: SkipReason },
}

impl Outcome {
    #[doc = "The ran operation for this canonical domain value."]
    pub const fn ran(validator_count: ScanValidatorCount) -> Self {
        Self::Ran { validator_count }
    }
    #[doc = "The skipped operation for this canonical domain value."]
    pub const fn skipped(reason: SkipReason) -> Self {
        Self::Skipped { reason }
    }
}

/// Named caller intent for a scan run.
// SERIALIZATION-DOC: this stable wire representation is consumed by durable adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[doc = "Canonical domain representation for ScanMode."]
pub enum ScanMode {
    Quick,
    Full,
    Repo,
    Workspace,
    Scoped,
    Diff,
    PlanScan,
}

impl<'de> serde::Deserialize<'de> for Outcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::boundary::scan::deserialize_outcome(deserializer)
    }
}

impl<'de> serde::Deserialize<'de> for ScanMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::boundary::scan::deserialize_scan_mode(deserializer)
    }
}

impl std::fmt::Display for ScanMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Quick => "quick",
            Self::Full => "full",
            Self::Repo => "repo",
            Self::Workspace => "workspace",
            Self::Scoped => "scoped",
            Self::Diff => "diff",
            Self::PlanScan => "plan-scan",
        })
    }
}

/// Boundary failure while decoding or resolving a scan mode.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ScanModeError."]
pub enum ScanModeError {
    #[error("unknown scan mode `{raw}`; expected quick, full, repo, workspace, scoped, diff, or plan-scan")]
    UnknownMode { raw: String },
    #[error("scan mode `diff` requires both a base and head commit")]
    DiffRangeMissing,
    #[error("diff range supplied for non-diff scan mode `{mode}`")]
    DiffRangeUnexpected { mode: ScanMode },
    #[error("scan mode `plan-scan` requires a plan scope")]
    PlanScanScopeMissing,
    #[error("scan request scope failed to resolve")]
    Scope(#[from] DecodeError),
}

impl std::str::FromStr for ScanMode {
    type Err = ScanModeError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "quick" => Ok(Self::Quick),
            "full" => Ok(Self::Full),
            "repo" => Ok(Self::Repo),
            "workspace" => Ok(Self::Workspace),
            "scoped" => Ok(Self::Scoped),
            "diff" => Ok(Self::Diff),
            "plan-scan" => Ok(Self::PlanScan),
            other => Err(ScanModeError::UnknownMode {
                // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
                raw: other.to_owned(),
            }),
        }
    }
}

/// Rule-tier subset selected by a scan mode.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for TierFilter."]
pub enum TierFilter {
    All,
    Only(Vec<Tier>),
}

/// Result of evaluating whether one tier is admitted by a scan filter.
///
/// This closed value keeps scan-mode APIs from returning an unbranded boolean
/// whose meaning can be confused with unrelated predicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for TierFilterDecision."]
pub enum TierFilterDecision {
    Allowed,
    Rejected,
}

impl TierFilterDecision {
    #[must_use]
    pub const fn is_allowed(self) -> bool {
        matches!(self, Self::Allowed)
    }
}

/// A scan request resolved into its execution inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ResolvedScanPlan."]
pub struct ResolvedScanPlan {
    pub mode: ScanMode,
    pub scope_request: ScopeRequest,
    pub tier_filter: TierFilter,
}

/// One validated directory-segment name excluded from repository walks.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for IgnoreDirectorySegment."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct IgnoreDirectorySegment(String);

impl IgnoreDirectorySegment {
    #[doc = "The new operation for this canonical domain value."]
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains('\\') {
            return Err(DecodeError::new(
                "ignoreDirectorySegment",
                "must be one non-empty directory segment",
            ));
        }
        if trimmed.len() == value.len() {
            Ok(Self(value))
        } else {
            // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
            Ok(Self(trimmed.to_owned()))
        }
    }

    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Coarse implementation-language family used by validator dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for LanguageFamily."]
pub enum LanguageFamily {
    Rust,
    TypeScript,
    Python,
    Terraform,
    YamlOrConfig,
    Unknown,
}

/// Rich language identity used by route-plan construction.
// SERDE-TAG-JUSTIFICATION: the established external wire form uses closed variant names.
// SERDE-TAG-JUSTIFICATION: the established external wire form uses closed variant names.
// SERDE-TAG-JUSTIFICATION: the established external wire form uses closed variant names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for DetectedLanguage."]
pub enum DetectedLanguage {
    Rust,
    TypeScript,
    Python,
    Dart,
    Go,
    Cfml,
    Other,
}

/// Canonical rule-pack selection key.
// SERDE-TAG-JUSTIFICATION: the established external wire form uses closed variant names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for RulePack."]
pub enum RulePack {
    Rust,
    TypeScript,
    Python,
    Security,
    LiteralScanFloor,
    SecurityAudit,
}

/// Semantic scope used to narrow route planning.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for RouteScope."]
pub enum RouteScope {
    #[default]
    Repo,
    Workspace,
    Crate(RelPath),
    Package(RelPath),
    Folder(RelPath),
    Domain(RelPath),
    Diff,
}

impl RouteScope {
    #[doc = "The root operation for this canonical domain value."]
    pub fn root(&self) -> Option<&RelPath> {
        match self {
            Self::Repo | Self::Workspace | Self::Diff => None,
            Self::Crate(root) | Self::Package(root) | Self::Folder(root) | Self::Domain(root) => {
                Some(root)
            }
        }
    }
}

macro_rules! serde_camel_case_unit_enum {
    ($name:ty, {$($variant:path => $wire:literal),+ $(,)?}) => {
        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(match self { $($variant => $wire),+ })
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let wire = <String as serde::Deserialize>::deserialize(deserializer)?;
                match wire.as_str() {
                    $($wire => Ok($variant)),+,
                    _ => Err(serde::de::Error::unknown_variant(&wire, &[$($wire),+])),
                }
            }
        }
    };
}

serde_camel_case_unit_enum!(DetectedLanguage, {
    DetectedLanguage::Rust => "rust",
    DetectedLanguage::TypeScript => "typeScript",
    DetectedLanguage::Python => "python",
    DetectedLanguage::Dart => "dart",
    DetectedLanguage::Go => "go",
    DetectedLanguage::Cfml => "cfml",
    DetectedLanguage::Other => "other",
});
serde_camel_case_unit_enum!(RulePack, {
    RulePack::Rust => "rust",
    RulePack::TypeScript => "typeScript",
    RulePack::Python => "python",
    RulePack::Security => "security",
    RulePack::LiteralScanFloor => "literalScanFloor",
    RulePack::SecurityAudit => "securityAudit",
});
