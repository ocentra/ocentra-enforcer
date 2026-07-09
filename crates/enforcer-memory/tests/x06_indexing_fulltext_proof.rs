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

fn assert_artifact_header(proof: &serde_json::Value, artifact: &str) {
    assert_eq!(proof["schemaVersion"], 1);
    assert_eq!(proof["artifact"], artifact);
    assert_eq!(proof["status"], "green");
    assert_eq!(proof["namedTest"], artifact);
    assert_eq!(proof["result"]["testsFailed"], 0);
}

#[test]
fn checked_in_indexing_proof_pins_incremental_and_bootstrap_requirements() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-indexing.json"))?;
    assert_artifact_header(&proof, "x06-indexing");

    let hard_requirements = proof["hardRequirements"]
        .as_object()
        .ok_or("x06-indexing hardRequirements must be an object")?;
    for requirement in [
        "skipUnchangedFiles",
        "reindexChangedFiles",
        "deletedFileTombstone",
        "gitHistoryMode",
        "artifactBootstrap",
    ] {
        assert_eq!(
            hard_requirements
                .get(requirement)
                .and_then(serde_json::Value::as_str),
            Some("covered"),
            "{requirement} must remain covered in x06-indexing"
        );
    }
    assert_eq!(hard_requirements.len(), 5);

    let evidence = string_set(&proof["result"]["evidenceTests"], "indexing evidenceTests")?;
    for test_name in [
        "code_graph_indexer::unchanged_files_are_skipped_across_reindex_runs",
        "code_graph_indexer::changed_file_is_reindexed_and_deleted_file_becomes_tombstone",
        "unit_code_graph::fast_mode_skips_git_history_full_mode_computes_it",
        "unit_code_graph::persistence_true_writes_artifact_and_bootstrap_reimports_same_counts",
    ] {
        assert!(
            evidence.contains(test_name),
            "x06-indexing missing required evidence test {test_name}"
        );
    }
    Ok(())
}

#[test]
fn checked_in_fulltext_proof_pins_tokenization_exact_search_and_rag_aggregate() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-fulltext.json"))?;
    let rag: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-rag.json"))?;
    assert_artifact_header(&proof, "x06-fulltext");
    assert_eq!(proof["aggregateArtifact"], "proof/memory/x06-rag.json");
    assert_eq!(rag["namedTest"], "memory-retrieval-stack");
    assert_eq!(rag["result"]["testsFailed"], 0);
    assert_eq!(
        rag["hardRequirements"]["codeAwareFullTextTokenization"]["status"],
        "DONE"
    );
    assert_eq!(
        rag["hardRequirements"]["degradedModeLabeledNotAcceptedForParity"]["status"],
        "DONE"
    );

    let hard_requirements = proof["hardRequirements"]
        .as_object()
        .ok_or("x06-fulltext hardRequirements must be an object")?;
    for requirement in ["codeAwareTokenization", "exactSearch", "zeroNetworkDefault"] {
        let detail = hard_requirements
            .get(requirement)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{requirement} must be present"))?;
        assert!(
            detail.contains("covered"),
            "{requirement} must remain covered in x06-fulltext"
        );
    }
    assert_eq!(hard_requirements.len(), 3);

    let evidence = string_set(&proof["result"]["evidenceTests"], "fulltext evidenceTests")?;
    for test_name in [
        "retrieval_stack::exact_query_returns_the_exact_match",
        "retrieval_stack::semantic_query_prefers_shared_vocabulary_document_over_unrelated_ones",
        "unit_search_stack::fulltext_tokenize_splits_camel_case",
        "unit_search_stack::fulltext_tokenize_splits_snake_case",
        "unit_search_stack::fulltext_tokenize_splits_kebab_case",
        "unit_search_stack::fulltext_tokenize_splits_path_separators",
    ] {
        assert!(
            evidence.contains(test_name),
            "x06-fulltext missing required evidence test {test_name}"
        );
    }
    Ok(())
}
