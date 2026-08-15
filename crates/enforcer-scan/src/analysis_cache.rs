//! Deterministic parse-once analysis cache for scan dispatch.

use std::collections::hash_map::{Entry, HashMap};

use enforcer_domain::hashes::Sha256;
use enforcer_domain::paths::RelPath;
use enforcer_domain::syntax_types::ProviderVersion;
use enforcer_validator::analysis::{content_hash, AnalysisProvider, PreparedAnalysis};

/// Cache identity: file, source content, and provider version.
#[derive(Clone, PartialEq, Eq, Hash)]
struct AnalysisCacheKey {
    file: RelPath,
    content_hash: Sha256,
    provider_version: ProviderVersion,
}

impl std::fmt::Debug for AnalysisCacheKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AnalysisCacheKey")
            .field("file", &self.file)
            .field("content_hash", &"[REDACTED]")
            .field("provider_version", &self.provider_version)
            .finish()
    }
}

impl AnalysisCacheKey {
    /// Build a stable key from the validated analysis inputs.
    fn from_parts(
        file: &RelPath,
        source: enforcer_domain::boundary::validation::ValidationSource<'_>,
        provider_version: ProviderVersion,
    ) -> Self {
        Self {
            file: file.clone(),
            content_hash: content_hash(source),
            provider_version,
        }
    }
}

/// Per-scan cache that guarantees one provider call per exact cache key.
#[derive(Default)]
pub struct AnalysisCache {
    entries: HashMap<AnalysisCacheKey, PreparedAnalysis>,
    provider_calls: usize,
}

impl AnalysisCache {
    /// Prepare one file or return the retained result for the same key.
    pub fn prepare<'a>(
        &'a mut self,
        file: &RelPath,
        source: enforcer_domain::boundary::validation::ValidationSource<'_>,
        scope: enforcer_domain::findings::ScanScope,
        provider: &dyn AnalysisProvider,
    ) -> &'a PreparedAnalysis {
        let provider_version = provider.provider_version();
        let key = AnalysisCacheKey::from_parts(file, source, provider_version);
        match self.entries.entry(key) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                self.provider_calls = self.provider_calls.saturating_add(1);
                let outcome = provider.analyze(file, source, scope);
                entry.insert(PreparedAnalysis::new(
                    content_hash(source),
                    provider_version,
                    outcome,
                ))
            }
        }
    }

    /// Return provider invocation count for instrumentation proofs.
    pub const fn provider_calls(&self) -> usize {
        self.provider_calls
    }

    /// Return the number of retained analysis entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Test whether no analysis entries were retained.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
