use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn checked_in_feature_parity_rollup_pins_green_matrix_and_qa_counts() -> TestResult {
    let feature: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-feature-parity.json"
    ))?;
    let qa: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-rag-qa.json"))?;

    assert_eq!(feature["allMatrixPrefixesGreen"], true);
    assert_eq!(feature["qaRowsTotal"], 250);
    assert_eq!(feature["qaRowsGreen"], 250);
    assert_eq!(feature["qaRowsGreenReal"], 0);
    assert_eq!(feature["qaRowsGreenHostLocalProof"], 22);
    assert_eq!(feature["qaRowsGreenDegraded"], 228);
    assert_eq!(feature["kgParityComparedAgainstBaseline"], true);
    assert_eq!(feature["mcpCliParity"], true);
    assert_eq!(feature["localDenseRetrievalPresent"], true);
    assert_eq!(feature["localRerankerPresent"], true);
    assert_eq!(feature["retrievalImprovementCurvePresent"], true);
    assert_eq!(feature["tokenReductionMedianAtLeast10x"], true);
    assert_eq!(feature["exactArtifactMismatchCount"], 0);
    assert_eq!(feature["externalModelProviderUsed"], false);

    assert_eq!(qa["rowsTotal"], feature["qaRowsTotal"]);
    assert_eq!(qa["rowsGreen"], feature["qaRowsGreen"]);
    assert_eq!(qa["rowsGreenReal"], feature["qaRowsGreenReal"]);
    assert_eq!(
        qa["rowsGreenHostLocalProof"],
        feature["qaRowsGreenHostLocalProof"]
    );
    assert_eq!(qa["rowsGreenDegraded"], feature["qaRowsGreenDegraded"]);
    assert_eq!(qa["rowsFailed"], 0);
    assert_eq!(qa["rowsUnrunnable"], 0);

    let prefixes = feature["prefixes"]
        .as_array()
        .ok_or("x06-feature-parity prefixes must be an array")?;
    assert_eq!(
        prefixes.len(),
        20,
        "feature rollup should pin every X06 matrix prefix"
    );

    let mut actual = BTreeMap::new();
    for prefix in prefixes {
        let prefix_id = prefix["prefix"]
            .as_str()
            .ok_or("prefix id must be a string")?;
        assert_eq!(
            prefix["status"], "green",
            "prefix {prefix_id} must remain green"
        );
        let artifact_path = prefix["artifactPath"]
            .as_str()
            .ok_or("prefix artifactPath must be a string")?;
        assert!(
            artifact_path.starts_with("proof/memory/x06-")
                && artifact_path.ends_with(".json")
                && Path::new(artifact_path).is_relative(),
            "prefix {prefix_id} artifact path must be portable X06 proof JSON, got {artifact_path}"
        );
        actual.insert(
            prefix_id.to_owned(),
            (
                prefix["testName"]
                    .as_str()
                    .ok_or("prefix testName must be a string")?
                    .to_owned(),
                artifact_path.to_owned(),
            ),
        );
    }

    let expected = BTreeMap::from([
        (
            "CLI".to_owned(),
            ("x06-cli".to_owned(), "proof/memory/x06-cli.json".to_owned()),
        ),
        (
            "COD".to_owned(),
            (
                "x06-code-graph".to_owned(),
                "proof/memory/x06-code-graph.json".to_owned(),
            ),
        ),
        (
            "DIA".to_owned(),
            (
                "x06-diagnostics".to_owned(),
                "proof/memory/x06-diagnostics.json".to_owned(),
            ),
        ),
        (
            "DOG".to_owned(),
            (
                "x06-dogfood-closeout".to_owned(),
                "proof/memory/x06-dogfood.json".to_owned(),
            ),
        ),
        (
            "FED".to_owned(),
            (
                "x06-federation".to_owned(),
                "proof/memory/x06-federation.json".to_owned(),
            ),
        ),
        (
            "GPH".to_owned(),
            ("x06-kg".to_owned(), "proof/memory/x06-kg.json".to_owned()),
        ),
        (
            "IDX".to_owned(),
            (
                "x06-indexing".to_owned(),
                "proof/memory/x06-indexing.json".to_owned(),
            ),
        ),
        (
            "LRN".to_owned(),
            (
                "x06-learning-curve".to_owned(),
                "proof/memory/x06-learning-curve.json".to_owned(),
            ),
        ),
        (
            "MCP".to_owned(),
            ("x06-mcp".to_owned(), "proof/memory/x06-mcp.json".to_owned()),
        ),
        (
            "MOD".to_owned(),
            (
                "x06-real-model-runtime-proof".to_owned(),
                "proof/memory/x06-models.json".to_owned(),
            ),
        ),
        (
            "PAR".to_owned(),
            (
                "x06-live-baseline-parity".to_owned(),
                "proof/memory/x06-kg-parity.json".to_owned(),
            ),
        ),
        (
            "QA".to_owned(),
            (
                "qa_gate_runs_every_row_and_reports_an_honest_wired_vs_unrunnable_split".to_owned(),
                "proof/memory/x06-rag-qa.json".to_owned(),
            ),
        ),
        (
            "RRK".to_owned(),
            (
                "x06-reranker".to_owned(),
                "proof/memory/x06-reranker.json".to_owned(),
            ),
        ),
        (
            "SEC".to_owned(),
            (
                "x06-policy-filters".to_owned(),
                "proof/memory/x06-policy.json".to_owned(),
            ),
        ),
        (
            "STO".to_owned(),
            (
                "memory-store-core".to_owned(),
                "proof/memory/x06-store.json".to_owned(),
            ),
        ),
        (
            "SUM".to_owned(),
            (
                "x06-summaries".to_owned(),
                "proof/memory/x06-summaries.json".to_owned(),
            ),
        ),
        (
            "TOK".to_owned(),
            (
                "x06-token-reduction".to_owned(),
                "proof/memory/x06-token-reduction.json".to_owned(),
            ),
        ),
        (
            "TXT".to_owned(),
            (
                "x06-fulltext".to_owned(),
                "proof/memory/x06-fulltext.json".to_owned(),
            ),
        ),
        (
            "VEC".to_owned(),
            (
                "x06-vector".to_owned(),
                "proof/memory/x06-vector.json".to_owned(),
            ),
        ),
        (
            "WVR".to_owned(),
            (
                "x06-weaver".to_owned(),
                "proof/memory/x06-weaver.json".to_owned(),
            ),
        ),
    ]);
    assert_eq!(actual, expected);

    let rows = qa["rows"]
        .as_array()
        .ok_or("x06-rag-qa rows must be an array")?;
    assert_eq!(rows.len(), 250);
    let mut ids = BTreeSet::new();
    let mut capability_counts = BTreeMap::new();
    for row in rows {
        let id = row["id"].as_str().ok_or("QA row id must be a string")?;
        assert!(ids.insert(id.to_owned()), "duplicate QA row id {id}");
        assert_eq!(row["verdict"], "pass", "QA row {id} must remain pass");
        let capability = row["capabilityState"]
            .as_str()
            .ok_or("QA row capabilityState must be a string")?;
        *capability_counts
            .entry(capability.to_owned())
            .or_insert(0usize) += 1;
    }
    assert_eq!(capability_counts.get("loaded"), None);
    assert_eq!(capability_counts.get("host-local-proof"), Some(&22));
    assert_eq!(capability_counts.get("degraded"), Some(&228));
    Ok(())
}
