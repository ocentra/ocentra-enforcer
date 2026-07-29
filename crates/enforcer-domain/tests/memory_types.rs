use enforcer_domain::memory_types::{
    ArchitectureCohesion, ArchitectureHotspotLimit, ArchitectureMaxIterations, Aspect,
    CodeSearchMode, ComplexityLanguage, ComplexitySignal, EdgeProvenance, EmbeddingGeneration,
    EmbeddingGenerationId, EmbeddingVector, EntryPointKind, Format, GraphArtifactPresence,
    GraphEventKind, GraphQueryResultRow, GraphQueryVariable, GraphSearchMode,
    GraphSymbolKindSnapshot, LanguageTag, LayerCategory, Level, LlamaCppBackendHint,
    LlamaCppLifecycleAction, LlamaCppLifecycleState, LlamaCppProbeKind, LocalRuntimeKind,
    MemoryAnalysisNodeId, MemoryLogSchemaVersion, MemoryProjectStoreRoot, MemoryProofPrefixStatus,
    MemoryStorePath, MemoryWatchFileCount, ModelRuntimeObservationKind, NodeLabel,
    OrtWorkerLifecycleState, OrtWorkerTask, ProceduralOutcome, ProviderKind, ReceiverHint,
    RecurrenceNegativeKind, ResourceClass, RiskLabel, RouteConfidence, RuntimeActivityState,
    RuntimeOwnershipMode, Seq, SkipPhase, SnippetAbsolutePath, SnippetIncludeNeighbors,
    SnippetMatchMethod, SnippetSourceBytes, StreamingCachePathSegment, TaskOutcome,
    VectorSearchLimit, VectorStaleReason,
};
use proptest::prelude::any;
use proptest::{prop_assert, prop_assert_eq, proptest};

fn assert_decode_rejected<T>(
    result: Result<T, enforcer_domain::boundary::decode_error::DecodeError>,
    path: &str,
) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
    match result {
        Err(error) => {
            assert_eq!(error.path, path);
            assert_ne!(error.reason, "");
            Ok(())
        }
        Ok(_) => Err(enforcer_domain::boundary::decode_error::DecodeError::new(
            path,
            "invalid input unexpectedly accepted",
        )),
    }
}

#[test]
fn memory_sequence_is_monotonic_and_preserves_its_wire_value() {
    let first = Seq::GENESIS.next();
    assert_eq!(u64::from(first), 1);
    assert_eq!(u64::from(first.next()), 2);
    assert_eq!(first.to_string(), "1");
}

#[test]
fn memory_log_schema_version_accepts_only_supported_wire_values(
) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
    assert_eq!(
        MemoryLogSchemaVersion::try_new(1),
        Ok(MemoryLogSchemaVersion::INITIAL)
    );
    assert_decode_rejected(MemoryLogSchemaVersion::try_new(0), "memoryLogSchemaVersion")?;
    assert_decode_rejected(MemoryLogSchemaVersion::try_new(2), "memoryLogSchemaVersion")?;
    assert_eq!(
        serde_json::from_str::<MemoryLogSchemaVersion>("0").map_err(|error| error.classify()),
        Err(serde_json::error::Category::Data)
    );
    assert_eq!(
        serde_json::from_str::<MemoryLogSchemaVersion>("2").map_err(|error| error.classify()),
        Err(serde_json::error::Category::Data)
    );
    Ok(())
}

#[test]
fn streaming_cache_path_segment_accepts_only_safe_nonempty_components(
) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
    let segment = StreamingCachePathSegment::try_new("artifact-v1.2_3".to_owned())?;
    assert_eq!(segment.as_str(), "artifact-v1.2_3");
    assert_decode_rejected(
        StreamingCachePathSegment::try_new(String::new()),
        "streamingCachePathSegment",
    )?;
    assert_decode_rejected(
        StreamingCachePathSegment::try_new(".".to_owned()),
        "streamingCachePathSegment",
    )?;
    assert_decode_rejected(
        StreamingCachePathSegment::try_new("..".to_owned()),
        "streamingCachePathSegment",
    )?;
    assert_decode_rejected(
        StreamingCachePathSegment::try_new("../escape".to_owned()),
        "streamingCachePathSegment",
    )?;
    Ok(())
}

#[test]
fn canonical_memory_wire_values_roundtrip_through_explicit_boundaries(
) -> Result<(), serde_json::Error> {
    let snippet_path = SnippetAbsolutePath::from(std::path::PathBuf::from("snippet.rs"));
    assert_eq!(serde_json::to_string(&snippet_path)?, "\"snippet.rs\"");
    assert_eq!(
        serde_json::from_str::<SnippetAbsolutePath>("\"snippet.rs\"")?,
        snippet_path
    );

    let store_root = MemoryProjectStoreRoot::from(std::path::PathBuf::from("memory-store"));
    assert_eq!(
        serde_json::from_str::<MemoryProjectStoreRoot>(&serde_json::to_string(&store_root)?)?,
        store_root
    );

    let embedding = EmbeddingVector::from(vec![0.25_f32, -0.5_f32]);
    assert_eq!(
        serde_json::from_str::<EmbeddingVector>(&serde_json::to_string(&embedding)?)?,
        embedding
    );
    let snippet = SnippetSourceBytes::from(vec![0_u8, 1_u8, 255_u8]);
    assert_eq!(
        serde_json::from_str::<SnippetSourceBytes>(&serde_json::to_string(&snippet)?)?,
        snippet
    );

    let store_path = MemoryStorePath::from(std::path::PathBuf::from("memory-store/index.json"));
    assert_eq!(
        serde_json::from_str::<MemoryStorePath>(&serde_json::to_string(&store_path)?)?,
        store_path
    );

    let mut query_row = GraphQueryResultRow::new();
    query_row.insert(
        GraphQueryVariable::from("document"),
        MemoryAnalysisNodeId::from("node-1"),
    );
    assert_eq!(
        serde_json::from_str::<GraphQueryResultRow>(&serde_json::to_string(&query_row)?)?,
        query_row
    );

    assert_eq!(
        serde_json::to_string(&SnippetMatchMethod::Suffix)?,
        "\"suffix\""
    );
    assert_eq!(
        serde_json::from_str::<ComplexitySignal>("\"Present\"")?,
        ComplexitySignal::Present
    );
    assert_eq!(serde_json::from_str::<RouteConfidence>("2.0")?.get(), 1.0);
    assert_eq!(
        f64::from(serde_json::from_str::<ArchitectureCohesion>("2.0")?),
        1.0
    );
    Ok(())
}

#[test]
fn zero_inclusive_counts_and_explicit_boolean_policies_preserve_domain_meaning() {
    assert_eq!(ArchitectureHotspotLimit::from(0).get(), 0);
    assert_eq!(ArchitectureMaxIterations::from(0).get(), 0);
    assert_eq!(MemoryWatchFileCount::from(0).get(), 0);
    assert_eq!(VectorSearchLimit::from(0).get(), 0);
    assert!(!SnippetIncludeNeighbors::from(false).is_included());
    assert!(GraphArtifactPresence::from(true).is_present());
}

proptest! {
    #[test]
    fn route_confidence_normalized_is_bounded_and_idempotent(value in any::<f64>()) {
        let normalized = RouteConfidence::normalized(value);
        prop_assert!((0.0..=1.0).contains(&normalized.get()));
        prop_assert_eq!(RouteConfidence::normalized(normalized.get()), normalized);
    }
}

#[test]
fn runtime_catalog_keeps_provider_ownership_and_worker_semantics_typed() {
    assert_eq!(ProviderKind::Cpu.resource_class(), ResourceClass::Cpu);
    assert_eq!(ProviderKind::Cuda.resource_class(), ResourceClass::Gpu);
    assert_eq!(ProviderKind::Npu.resource_class(), ResourceClass::Npu);
    assert!(RuntimeOwnershipMode::EnforcerSubprocess.is_enforcer_owned());
    assert!(!RuntimeOwnershipMode::ExternalServer.is_enforcer_owned());
    assert!(LocalRuntimeKind::OnnxOrt.is_real_backend());
    assert!(!LocalRuntimeKind::DeterministicFallback.is_real_backend());
    assert_eq!(OrtWorkerTask::Embedding.env_value(), "embedding");
    assert_eq!(
        OrtWorkerLifecycleState::PausedReranking.activity(),
        RuntimeActivityState::Paused
    );
}

#[test]
fn search_modes_are_distinct_and_graph_search_defaults_to_bm25() {
    assert_ne!(CodeSearchMode::Compact, CodeSearchMode::Full);
    assert_eq!(GraphSearchMode::default(), GraphSearchMode::Bm25);
}

#[test]
fn model_observation_kind_uses_the_stable_durable_spelling() -> Result<(), serde_json::Error> {
    let kind = ModelRuntimeObservationKind::SuccessfulLocalLoad;
    assert_eq!(kind.as_str(), "successful-local-load");
    assert_eq!(serde_json::to_string(&kind)?, "\"successful-local-load\"");
    Ok(())
}

#[test]
fn graph_and_embedding_catalog_preserves_wire_and_state_semantics() -> Result<(), serde_json::Error>
{
    assert_eq!(u32::from(EmbeddingGenerationId::RECOVERY), 0);
    assert_eq!(u32::from(EmbeddingGenerationId::INITIAL), 1);
    assert_eq!(EmbeddingGenerationId::INITIAL.to_string(), "1");
    assert_eq!(RiskLabel::Critical.as_str(), "CRITICAL");
    assert_eq!(
        serde_json::to_string(&EdgeProvenance::Runtime)?,
        "\"runtime\""
    );
    assert_eq!(
        serde_json::to_string(&GraphSymbolKindSnapshot::TypeAlias)?,
        "\"TypeAlias\""
    );
    let generation = EmbeddingGeneration::Migrating {
        active: EmbeddingGenerationId::from_nonzero(
            std::num::NonZeroU32::new(4).unwrap_or(std::num::NonZeroU32::MIN),
        ),
        next: EmbeddingGenerationId::from_nonzero(
            std::num::NonZeroU32::new(5).unwrap_or(std::num::NonZeroU32::MIN),
        ),
    };
    assert_eq!(u32::from(generation.active()), 4);
    assert_eq!(generation.next().map(u32::from), Some(5));
    assert!(generation.is_migrating());
    assert_eq!(
        serde_json::to_string(&LlamaCppProbeKind::Embedding)?,
        "\"embedding\""
    );
    assert_eq!(
        serde_json::to_string(&LlamaCppBackendHint::OpenVino)?,
        "\"open-vino\""
    );
    assert_eq!(
        serde_json::to_string(&LlamaCppLifecycleState::PausedChat)?,
        "\"paused-chat\""
    );
    assert_eq!(
        serde_json::to_string(&LlamaCppLifecycleAction::TimeoutKill)?,
        "\"timeout-kill\""
    );
    assert_eq!(
        serde_json::to_string(&ProceduralOutcome::FixSuccess)?,
        "\"fix-success\""
    );
    assert!(ProceduralOutcome::RetrievalSuccess.is_success());
    assert_ne!(Aspect::Overview, Aspect::Dependencies);
    assert_eq!(EntryPointKind::BinaryMain, EntryPointKind::BinaryMain);
    assert_eq!(LayerCategory::Api, LayerCategory::Api);
    assert!(Level::Error.should_emit(Level::Warn));
    assert!(!Level::Debug.should_emit(Level::Info));
    assert_eq!(Format::Json, Format::Json);
    assert_eq!(SkipPhase::Extract.to_string(), "extract");
    assert_eq!(f64::from(NodeLabel::Function.bm25_boost()), 10.0);
    assert!(NodeLabel::Module.is_bm25_noise());
    assert!(matches!(
        TaskOutcome::Succeeded {
            task_key: "file-changed:src/lib.rs".into()
        },
        TaskOutcome::Succeeded { .. }
    ));
    assert!(matches!(
        VectorStaleReason::Dimension {
            expected: 384,
            actual: 768
        },
        VectorStaleReason::Dimension { .. }
    ));
    assert_ne!(ComplexityLanguage::Rust, ComplexityLanguage::Python);
    assert_ne!(ReceiverHint::SelfOrThis, ReceiverHint::Identifier);
    assert_ne!(LanguageTag::Rust, LanguageTag::TypeScript);
    assert!(MemoryProofPrefixStatus::Green.is_green());
    assert_eq!(
        serde_json::to_string(&MemoryProofPrefixStatus::Pending)?,
        "\"pending\""
    );
    assert_ne!(
        GraphEventKind::NodeAdded {
            node_id: "node-a".into(),
            node_kind: "function".into()
        },
        GraphEventKind::EdgeAdded {
            from: "node-a".into(),
            to: "node-b".into(),
            label: "calls".into()
        }
    );
    assert_ne!(
        RecurrenceNegativeKind::RecurrenceCount {
            recurrence_count: 2.into(),
            previous_count: Some(1.into())
        },
        RecurrenceNegativeKind::NegativeEvidence {
            reason: "clean run".into()
        }
    );
    Ok(())
}
