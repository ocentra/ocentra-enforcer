use std::collections::BTreeSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn string_set(
    value: &serde_json::Value,
    name: &str,
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let array = value
        .as_array()
        .ok_or_else(|| format!("{name} must be an array"))?;
    array
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{name} must contain only strings").into())
        })
        .collect()
}

#[test]
fn checked_in_vector_proof_pins_dense_index_staleness_and_degraded_default() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-vector.json"))?;
    let rag: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-rag.json"))?;

    assert_eq!(proof["schemaVersion"], 1);
    assert_eq!(proof["artifact"], "x06-vector");
    assert_eq!(proof["status"], "green");
    assert_eq!(proof["aggregateArtifact"], "proof/memory/x06-rag.json");
    assert_eq!(proof["namedTest"], "x06-vector");
    assert_eq!(proof["result"]["testsFailed"], 0);

    let hard_requirements = proof["hardRequirements"]
        .as_object()
        .ok_or("x06-vector hardRequirements must be an object")?;
    for requirement in [
        "denseVectorIndex",
        "staleVectorDetection",
        "localDefaultState",
    ] {
        let detail = hard_requirements
            .get(requirement)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{requirement} must be present"))?;
        assert!(
            detail.contains("covered")
                || detail.contains("default CI uses deterministic degraded provider"),
            "{requirement} must remain explicitly covered in x06-vector"
        );
    }
    assert_eq!(hard_requirements.len(), 3);

    assert_eq!(rag["namedTest"], "memory-retrieval-stack");
    assert_eq!(rag["result"]["testsFailed"], 0);
    assert_eq!(
        rag["hardRequirements"]["hnswVectorIndexCodeChunks"]["status"],
        "DONE"
    );
    assert_eq!(
        rag["hardRequirements"]["hnswVectorIndexLessonsArtifactsSummaries"]["status"],
        "DONE"
    );
    assert_eq!(
        rag["hardRequirements"]["degradedModeLabeledNotAcceptedForParity"]["status"],
        "DONE"
    );

    let evidence = string_set(&proof["result"]["evidenceTests"], "vector evidenceTests")?;
    for test_name in [
        "retrieval_stack::semantic_query_prefers_shared_vocabulary_document_over_unrelated_ones",
        "retrieval_stack::vector_index_manifest_detects_staleness_on_dimension_change",
        "retrieval_stack::model_manifest_carries_the_full_version_vector",
    ] {
        assert!(
            evidence.contains(test_name),
            "x06-vector missing required evidence test {test_name}"
        );
    }
    Ok(())
}
