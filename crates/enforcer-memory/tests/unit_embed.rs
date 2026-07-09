use enforcer_memory::embed::{
    cosine_similarity, DegradedState, Embedder, HashingEmbedder, LoadState, LocalEmbedder,
    HASHING_EMBEDDER_DIMENSION,
};

#[test]
fn hashing_embedder_is_deterministic_across_calls() -> enforcer_memory::error::Result<()> {
    let embedder = HashingEmbedder::new();
    let a = embedder.embed("parseConfigFile")?;
    let b = embedder.embed("parseConfigFile")?;
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn hashing_embedder_reports_degraded_state() {
    let embedder = HashingEmbedder::new();
    assert_eq!(
        embedder.state(),
        LoadState::Degraded(DegradedState::ProviderUnavailable)
    );
}

#[test]
fn local_embedder_default_is_the_degraded_zero_network_provider(
) -> enforcer_memory::error::Result<()> {
    let embedder = LocalEmbedder::default();
    let vector = embedder.embed("parse config file")?;

    assert_eq!(vector.len(), HASHING_EMBEDDER_DIMENSION);
    assert_eq!(
        embedder.state(),
        LoadState::Degraded(DegradedState::ProviderUnavailable)
    );
    Ok(())
}

#[cfg(not(feature = "ort-models"))]
#[test]
fn local_embedder_rejects_ort_when_feature_is_not_compiled() {
    let spec = enforcer_memory::model_runtime::ModelSpec::qwen3_embedding(
        "missing.onnx",
        "0".repeat(64),
        "tokenizer.json",
        "1".repeat(64),
    );
    let result = LocalEmbedder::try_ort(&spec, enforcer_memory::model_runtime::ProviderKind::Cpu);

    assert!(matches!(
        result,
        Err(enforcer_memory::error::MemoryError::ModelRuntime {
            operation: "load-local-ort-embedder",
            ..
        })
    ));
}

#[test]
fn shared_vocabulary_queries_are_more_similar_than_disjoint_ones(
) -> enforcer_memory::error::Result<()> {
    let embedder = HashingEmbedder::new();
    let a = embedder.embed("parse config file for the widget loader")?;
    let b = embedder.embed("parse config file for the widget reader")?;
    let c = embedder.embed("unrelated network socket timeout retry logic")?;
    let sim_ab = cosine_similarity(&a, &b);
    let sim_ac = cosine_similarity(&a, &c);
    assert!(
        sim_ab > sim_ac,
        "shared-vocabulary texts should be closer than disjoint-vocabulary ones: {sim_ab} vs {sim_ac}"
    );
    Ok(())
}

#[test]
fn cosine_similarity_is_zero_for_mismatched_lengths() {
    assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0, 0.0]), 0.0);
}

#[test]
fn cosine_similarity_is_one_for_identical_normalized_vectors() {
    let mut v = vec![3.0f32, 4.0];
    let norm = (v.iter().map(|value| value * value).sum::<f32>()).sqrt();
    for value in &mut v {
        *value /= norm;
    }
    let sim = cosine_similarity(&v, &v);
    assert!((sim - 1.0).abs() < 1e-6);
}

#[test]
fn model_info_reports_stable_version_vector_fields() {
    let embedder = HashingEmbedder::new();
    let info = embedder.model_info();
    assert_eq!(info.dimension, HASHING_EMBEDDER_DIMENSION);
    assert_eq!(info.similarity_metric, "cosine");
}
