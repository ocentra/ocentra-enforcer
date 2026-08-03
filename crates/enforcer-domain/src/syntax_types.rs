//! Serialization-free values for normalized syntax facts.
//!
//! This module is deliberately smaller than the parser registry.  It owns the
//! closed values that can cross from a syntax provider into graph and rule
//! consumers; wire decoding and parser selection remain boundary concerns.

use crate::boundary::decode_error::DecodeError;
use crate::paths::RelPath;
use std::num::NonZeroUsize;
use std::ops::{Range, RangeInclusive};

/// A validated language identity as reported by the syntax boundary.
/// BRAND-INVARIANT: the inner label is non-empty and is created only by the
/// fallible constructor at the syntax boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageIdentity(String);

impl LanguageIdentity {
    /// Construct a language identity from a non-empty provider label.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        if value.trim().is_empty() {
            return Err(DecodeError::new(
                "language",
                "language identity must not be empty",
            ));
        }
        Ok(Self(value))
    }

    /// Return the stable provider label.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The bounded provider set exercised by UL04.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderIdentity {
    /// Rust grammar provider.
    TreeSitterRust,
    /// Python grammar provider.
    TreeSitterPython,
    /// TypeScript grammar provider.
    TreeSitterTypeScript,
    /// Go grammar provider.
    TreeSitterGo,
    /// A route has no selected provider for this language yet.
    Unavailable,
    /// The route is not a structural language route.
    Unsupported,
}

/// Locked dependency version for one selected syntax provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderVersion {
    /// tree-sitter ABI version used by the syntax crate.
    TreeSitter025,
    /// Rust grammar binding version.
    Rust023,
    /// Python grammar binding version.
    Python023,
    /// TypeScript grammar binding version.
    TypeScript023,
    /// Go grammar binding version.
    Go023,
}

impl ProviderVersion {
    /// Return the locked version label used in proof artifacts.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TreeSitter025 => "0.25",
            Self::Rust023 => "0.23",
            Self::Python023 => "0.23",
            Self::TypeScript023 => "0.23",
            Self::Go023 => "0.23",
        }
    }
}

impl ProviderIdentity {
    /// Return the stable provider label used in proof artifacts.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TreeSitterRust => "tree-sitter-rust",
            Self::TreeSitterPython => "tree-sitter-python",
            Self::TreeSitterTypeScript => "tree-sitter-typescript",
            Self::TreeSitterGo => "tree-sitter-go",
            Self::Unavailable => "unavailable",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Parse-quality outcome.  No outcome implies semantic-rule coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseOutcome {
    /// The selected provider returned a tree without error or missing nodes.
    ParsedClean,
    /// The selected provider returned a recovered tree with syntax defects.
    ParsedWithErrors,
    /// Input was refused before crossing the native parser ABI.
    UnsafeInputRefused,
    /// A structural route exists, but this bounded slice has no provider.
    ProviderUnavailable,
    /// The input is not a structural language route.
    Unsupported,
}

/// Capabilities carried by an analysis result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactCapability {
    /// Function declaration facts with checked source spans.
    FunctionFacts,
}

/// A closed capability set for one analysis result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilitySet {
    /// BRAND-INVARIANT: this flag is set only by closed capability factories.
    function_facts: bool,
}

impl CapabilitySet {
    /// An empty capability set for unsupported, unavailable, or unsafe input.
    pub const fn empty() -> Self {
        Self {
            function_facts: false,
        }
    }

    /// The selected function-fact capability.
    pub const fn function_facts() -> Self {
        Self {
            function_facts: true,
        }
    }

    /// Test whether a capability is present.
    pub const fn contains(self, capability: FactCapability) -> bool {
        match capability {
            FactCapability::FunctionFacts => self.function_facts,
        }
    }
}

/// Checked byte interval for a normalized fact.
/// BRAND-INVARIANT: the interval is ordered by the fallible constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteRange {
    start: usize,
    end: usize,
}

impl ByteRange {
    /// Construct an ordered byte interval.
    pub fn try_from_range(range: Range<usize>) -> Result<Self, DecodeError> {
        if range.start > range.end {
            return Err(DecodeError::new("span.bytes", "start must not exceed end"));
        }
        Ok(Self {
            start: range.start,
            end: range.end,
        })
    }

    /// Return the interval start.
    pub const fn start(self) -> usize {
        self.start
    }

    /// Return the interval end.
    pub const fn end(self) -> usize {
        self.end
    }
}

/// Checked one-based line interval for a normalized fact.
/// BRAND-INVARIANT: both endpoints are non-zero and ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineRange {
    start: NonZeroUsize,
    end: NonZeroUsize,
}

impl LineRange {
    /// Construct an ordered, one-based line interval.
    pub fn try_from_range(range: RangeInclusive<usize>) -> Result<Self, DecodeError> {
        let start = NonZeroUsize::new(*range.start())
            .ok_or_else(|| DecodeError::new("span.lines", "start line must be one-based"))?;
        let end = NonZeroUsize::new(*range.end())
            .ok_or_else(|| DecodeError::new("span.lines", "end line must be one-based"))?;
        if start > end {
            return Err(DecodeError::new("span.lines", "lines must be ordered"));
        }
        Ok(Self { start, end })
    }

    /// Return the one-based interval start.
    pub const fn start(self) -> usize {
        self.start.get()
    }

    /// Return the one-based interval end.
    pub const fn end(self) -> usize {
        self.end.get()
    }
}

/// Checked source span for a normalized fact.
/// BRAND-INVARIANT: byte and line ranges have both passed their closed
/// constructors before they can cross the syntax/domain boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxSpan {
    byte_range: ByteRange,
    line_range: LineRange,
}

impl SyntaxSpan {
    /// Construct a span from already typed, checked intervals.
    pub const fn from_ranges(byte_range: ByteRange, line_range: LineRange) -> Self {
        Self {
            byte_range,
            line_range,
        }
    }

    /// Return the checked byte interval.
    pub const fn byte_range(self) -> ByteRange {
        self.byte_range
    }

    /// Return the checked one-based line interval.
    pub const fn line_range(self) -> LineRange {
        self.line_range
    }
}

/// One normalized function declaration fact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionFact {
    /// BRAND-INVARIANT: the name is non-empty and created only by `try_new`.
    name: String,
    /// BRAND-INVARIANT: the span has passed both checked interval factories.
    span: SyntaxSpan,
}

impl FunctionFact {
    /// Construct a function fact with a non-empty name and checked span.
    pub fn try_new(name: String, span: SyntaxSpan) -> Result<Self, DecodeError> {
        if name.trim().is_empty() {
            return Err(DecodeError::new(
                "function.name",
                "function name must not be empty",
            ));
        }
        Ok(Self { name, span })
    }

    /// Return the source-level function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the checked declaration span.
    pub const fn span(&self) -> SyntaxSpan {
        self.span
    }
}

/// Provider provenance for one analysis result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProviderProvenance {
    /// BRAND-INVARIANT: provider is a closed enum and version is a locked enum.
    provider: ProviderIdentity,
    version: ProviderVersion,
}

impl ProviderProvenance {
    /// Create provenance from a closed provider identity and locked version.
    pub const fn new(provider: ProviderIdentity, version: ProviderVersion) -> Self {
        Self { provider, version }
    }

    /// Return the provider identity.
    pub const fn provider(self) -> ProviderIdentity {
        self.provider
    }

    /// Return the dependency version recorded by the syntax boundary.
    pub const fn version(self) -> ProviderVersion {
        self.version
    }
}

/// Parse quality recorded independently from semantic-rule coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseQuality {
    /// No native parser was invoked.
    NotParsed,
    /// The native provider returned a tree with no error or missing nodes.
    Clean,
    /// The native provider returned a recovered tree.
    Recovered {
        /// Number of error nodes, if any.
        errors: Option<NonZeroUsize>,
        /// Number of missing nodes, if any.
        missing: Option<NonZeroUsize>,
    },
}

impl ParseQuality {
    /// Build quality from checked provider counts.
    pub const fn recovered(errors: Option<NonZeroUsize>, missing: Option<NonZeroUsize>) -> Self {
        Self::Recovered { errors, missing }
    }

    /// Return the error-node count.
    pub const fn error_count(self) -> usize {
        match self {
            Self::Recovered {
                errors: Some(errors),
                ..
            } => errors.get(),
            Self::NotParsed | Self::Clean | Self::Recovered { errors: None, .. } => 0,
        }
    }

    /// Return the missing-node count.
    pub const fn missing_count(self) -> usize {
        match self {
            Self::Recovered {
                missing: Some(missing),
                ..
            } => missing.get(),
            Self::NotParsed | Self::Clean | Self::Recovered { missing: None, .. } => 0,
        }
    }
}

/// Typed input collected before constructing one analysis result.
#[derive(Debug, Default)]
pub struct SyntaxAnalysisInput {
    language: Option<LanguageIdentity>,
    file: Option<RelPath>,
    provenance: Option<ProviderProvenance>,
    outcome: Option<ParseOutcome>,
    quality: Option<ParseQuality>,
    capabilities: Option<CapabilitySet>,
    function_facts: Option<Vec<FunctionFact>>,
}

impl SyntaxAnalysisInput {
    /// Start an empty typed observation.
    pub const fn empty() -> Self {
        Self {
            language: None,
            file: None,
            provenance: None,
            outcome: None,
            quality: None,
            capabilities: None,
            function_facts: None,
        }
    }

    /// Set the language identity.
    pub fn with_language(mut self, value: LanguageIdentity) -> Self {
        self.language = Some(value);
        self
    }

    /// Set the validated file path.
    pub fn with_file(mut self, value: RelPath) -> Self {
        self.file = Some(value);
        self
    }

    /// Set provider provenance.
    pub fn with_provenance(mut self, value: ProviderProvenance) -> Self {
        self.provenance = Some(value);
        self
    }

    /// Set the parse-quality outcome.
    pub fn with_outcome(mut self, value: ParseOutcome) -> Self {
        self.outcome = Some(value);
        self
    }

    /// Set provider quality counts.
    pub fn with_quality(mut self, value: ParseQuality) -> Self {
        self.quality = Some(value);
        self
    }

    /// Set the closed capability set.
    pub fn with_capabilities(mut self, value: CapabilitySet) -> Self {
        self.capabilities = Some(value);
        self
    }

    /// Set the normalized function facts.
    pub fn with_function_facts(mut self, value: Vec<FunctionFact>) -> Self {
        self.function_facts = Some(value);
        self
    }
}

/// One complete, serialization-free syntax analysis result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxAnalysisResult {
    /// BRAND-INVARIANT: all fields are private and constructed by `try_new`.
    language: LanguageIdentity,
    file: RelPath,
    provenance: ProviderProvenance,
    outcome: ParseOutcome,
    quality: ParseQuality,
    capabilities: CapabilitySet,
    function_facts: Vec<FunctionFact>,
}

impl SyntaxAnalysisResult {
    /// Build an analysis result from provider observations.
    pub fn try_new(input: SyntaxAnalysisInput) -> Result<Self, DecodeError> {
        let language = input
            .language
            .ok_or_else(|| DecodeError::new("language", "analysis language is required"))?;
        let file = input
            .file
            .ok_or_else(|| DecodeError::new("file", "analysis file is required"))?;
        let provenance = input
            .provenance
            .ok_or_else(|| DecodeError::new("provenance", "analysis provenance is required"))?;
        let outcome = input
            .outcome
            .ok_or_else(|| DecodeError::new("outcome", "analysis outcome is required"))?;
        let quality = input
            .quality
            .ok_or_else(|| DecodeError::new("quality", "analysis quality is required"))?;
        let capabilities = input.capabilities.ok_or_else(|| {
            DecodeError::new("capabilities", "analysis capabilities are required")
        })?;
        let function_facts = input.function_facts.ok_or_else(|| {
            DecodeError::new("functionFacts", "analysis function facts are required")
        })?;
        if matches!(
            outcome,
            ParseOutcome::ParsedClean | ParseOutcome::ParsedWithErrors
        ) && matches!(quality, ParseQuality::NotParsed)
        {
            return Err(DecodeError::new(
                "quality",
                "parsed outcomes require provider quality",
            ));
        }
        Ok(Self {
            language,
            file,
            provenance,
            outcome,
            quality,
            capabilities,
            function_facts,
        })
    }

    /// Return the language identity.
    pub fn language(&self) -> &LanguageIdentity {
        &self.language
    }

    /// Return the validated repository-relative file path.
    pub fn file(&self) -> &RelPath {
        &self.file
    }

    /// Return provider provenance.
    pub const fn provenance(&self) -> ProviderProvenance {
        self.provenance
    }

    /// Return the parse-quality outcome.
    pub const fn outcome(&self) -> ParseOutcome {
        self.outcome
    }

    /// Return the number of error nodes observed by the provider.
    pub const fn error_count(&self) -> usize {
        self.quality.error_count()
    }

    /// Return the number of missing nodes observed by the provider.
    pub const fn missing_count(&self) -> usize {
        self.quality.missing_count()
    }

    /// Return the capability set.
    pub const fn capabilities(&self) -> CapabilitySet {
        self.capabilities
    }

    /// Return normalized function facts without exposing parser nodes.
    pub fn function_facts(&self) -> &[FunctionFact] {
        &self.function_facts
    }
}
