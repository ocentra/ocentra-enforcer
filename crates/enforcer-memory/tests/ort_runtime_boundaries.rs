fn occurrence_count(source: &str, needle: &str) -> usize {
    source.match_indices(needle).count()
}

#[test]
fn ort_runtime_models_missing_output_and_shape_dimensions_as_errors() {
    let runtime = include_str!("../src/ort_runtime.rs");

    assert_eq!(occurrence_count(runtime, "outputs.values().next()"), 2);
    assert_eq!(
        occurrence_count(runtime, "ONNX Runtime returned no embedding output"),
        1
    );
    assert_eq!(
        occurrence_count(runtime, "ONNX Runtime returned no reranker output"),
        1
    );
    assert_eq!(
        occurrence_count(runtime, "missing sequence dimension in rank-3 logits"),
        1
    );
    assert_eq!(
        occurrence_count(runtime, "missing vocabulary dimension in rank-3 logits"),
        1
    );
    assert_eq!(
        occurrence_count(runtime, "missing sequence dimension in rank-3 embedding output"),
        1
    );
    assert_eq!(
        occurrence_count(runtime, "missing embedding dimension in rank-3 output"),
        1
    );
    assert_eq!(occurrence_count(runtime, "outputs[0]"), 0);
    assert_eq!(occurrence_count(runtime, "shape[1]"), 0);
    assert_eq!(occurrence_count(runtime, "shape[2]"), 0);
}
