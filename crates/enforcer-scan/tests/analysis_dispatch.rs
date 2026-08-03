use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_domain::syntax_types::ProviderVersion;
use enforcer_scan::analysis_cache::AnalysisCache;
use enforcer_validator::analysis::{AnalysisOutcome, AnalysisProvider};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct CountingProvider {
    calls: Arc<AtomicUsize>,
}

impl AnalysisProvider for CountingProvider {
    fn provider_version(&self) -> ProviderVersion {
        ProviderVersion::TreeSitter025
    }

    fn analyze(
        &self,
        _file: &RelPath,
        _source: ValidationSource<'_>,
        _scope: ScanScope,
    ) -> AnalysisOutcome {
        self.calls.fetch_add(1, Ordering::SeqCst);
        AnalysisOutcome::LegacyText
    }
}

#[test]
fn cache_calls_provider_once_for_same_file_content_and_version() -> Result<(), DecodeError> {
    let path = RelPath::try_new("src/cache.rs")?;
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = CountingProvider {
        calls: Arc::clone(&calls),
    };
    let mut cache = AnalysisCache::default();
    let first = cache
        .prepare(
            &path,
            ValidationSource::from_text("fn cached() {}"),
            ScanScope::Files,
            &provider,
        )
        .clone();
    let second = cache
        .prepare(
            &path,
            ValidationSource::from_text("fn cached() {}"),
            ScanScope::Files,
            &provider,
        )
        .clone();

    assert_eq!(first.outcome(), second.outcome());
    assert_eq!(cache.provider_calls(), 1);
    assert_eq!(cache.len(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    Ok(())
}

#[test]
fn changed_content_gets_a_distinct_analysis_entry() -> Result<(), DecodeError> {
    let path = RelPath::try_new("src/cache.rs")?;
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = CountingProvider {
        calls: Arc::clone(&calls),
    };
    let mut cache = AnalysisCache::default();
    let _first = cache.prepare(
        &path,
        ValidationSource::from_text("fn first() {}"),
        ScanScope::Files,
        &provider,
    );
    let _second = cache.prepare(
        &path,
        ValidationSource::from_text("fn second() {}"),
        ScanScope::Files,
        &provider,
    );

    assert_eq!(cache.provider_calls(), 2);
    assert_eq!(cache.len(), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    Ok(())
}
