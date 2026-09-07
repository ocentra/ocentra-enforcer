//! Prepared analysis and parse-once provider contracts.

use enforcer_domain::boundary::hash::validate;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::paths::RelPath;
use enforcer_domain::syntax_types::{
    CapabilitySet, FactCapability, ParseOutcome, ProviderVersion, SyntaxAnalysisResult,
};

/// Outcome produced once by one provider for one content hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisOutcome {
    /// Existing text-only validators remain behavior-compatible.
    LegacyText,
    /// A normalized syntax result is available to fact-backed consumers.
    FactBacked(SyntaxAnalysisResult),
    /// A structural route exists but no provider is available.
    ProviderUnavailable,
    /// A provider was selected but could not produce a trustworthy result.
    ParserFailure,
}

impl AnalysisOutcome {
    /// Return the closed capability set carried by this outcome.
    pub const fn capabilities(&self) -> CapabilitySet {
        match self {
            Self::FactBacked(result) => result.capabilities(),
            Self::LegacyText | Self::ProviderUnavailable | Self::ParserFailure => {
                CapabilitySet::empty()
            }
        }
    }

    /// Return whether a requested capability is present and parse-trustworthy.
    pub const fn capability_match(&self, capability: FactCapability) -> CapabilityMatch {
        match self {
            Self::FactBacked(result)
                if matches!(
                    result.outcome(),
                    ParseOutcome::ParsedClean | ParseOutcome::ParsedWithErrors
                ) =>
            {
                if result.capabilities().contains(capability) {
                    CapabilityMatch::Satisfied
                } else {
                    CapabilityMatch::NotSatisfied
                }
            }
            Self::LegacyText | Self::ProviderUnavailable | Self::ParserFailure => {
                CapabilityMatch::NotSatisfied
            }
            Self::FactBacked(_) => CapabilityMatch::NotSatisfied,
        }
    }
}

/// Closed result of checking whether one analysis capability is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityMatch {
    /// The provider supplied the requested trustworthy capability.
    Satisfied,
    /// The capability is absent, incomplete, or not trustworthy.
    NotSatisfied,
}

/// A provider result retained by the parse-once cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedAnalysis {
    content_hash: Sha256,
    provider_version: ProviderVersion,
    outcome: AnalysisOutcome,
}

impl PreparedAnalysis {
    /// Create a retained result for one validated source observation.
    pub fn new(
        content_hash: Sha256,
        provider_version: ProviderVersion,
        outcome: AnalysisOutcome,
    ) -> Self {
        Self {
            content_hash,
            provider_version,
            outcome,
        }
    }

    /// Return the source content hash used in the cache key.
    pub fn content_hash(&self) -> &Sha256 {
        &self.content_hash
    }

    /// Return the provider version used in the cache key.
    pub const fn provider_version(&self) -> ProviderVersion {
        self.provider_version
    }

    /// Return the closed outcome.
    pub fn outcome(&self) -> &AnalysisOutcome {
        &self.outcome
    }

    /// Test one requested fact capability without treating fallback as clean.
    pub fn capability_match(&self, capability: FactCapability) -> CapabilityMatch {
        self.outcome.capability_match(capability)
    }
}

/// Provider seam used by scan orchestration.
pub trait AnalysisProvider: Send + Sync {
    /// Return the provider version used for cache identity.
    fn provider_version(&self) -> ProviderVersion;

    /// Analyze one file once; callers own caching and deterministic ordering.
    fn analyze(
        &self,
        file: &RelPath,
        source: ValidationSource<'_>,
        scope: ScanScope,
    ) -> AnalysisOutcome;
}

/// Compatibility provider that preserves the pre-UL05 text-validator behavior.
#[derive(Debug, Default, Clone, Copy)]
pub struct LegacyAnalysisProvider;

impl AnalysisProvider for LegacyAnalysisProvider {
    fn provider_version(&self) -> ProviderVersion {
        ProviderVersion::TreeSitter025
    }

    fn analyze(
        &self,
        _file: &RelPath,
        _source: ValidationSource<'_>,
        _scope: ScanScope,
    ) -> AnalysisOutcome {
        AnalysisOutcome::LegacyText
    }
}

/// Hash one source boundary using the canonical domain hash provider.
pub fn content_hash(source: ValidationSource<'_>) -> Sha256 {
    validate(source.as_str().as_bytes())
}
