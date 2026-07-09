use std::collections::{BTreeMap, BTreeSet};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn checked_in_rag_qa_proof_matches_feature_rollup_without_fake_green() -> TestResult {
    let qa: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-rag-qa.json"))?;
    let feature: serde_json::Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-feature-parity.json"
    ))?;

    assert_eq!(qa["rowsTotal"], 250);
    assert_eq!(qa["rowsGreen"], 250);
    assert_eq!(qa["rowsFailed"], 0);
    assert_eq!(qa["rowsUnrunnable"], 0);
    assert_eq!(feature["allMatrixPrefixesGreen"], true);
    assert_eq!(feature["qaRowsTotal"], qa["rowsTotal"]);
    assert_eq!(feature["qaRowsGreen"], qa["rowsGreen"]);
    assert_eq!(feature["qaRowsGreenReal"], qa["rowsGreenReal"]);
    assert_eq!(
        feature["qaRowsGreenHostLocalProof"],
        qa["rowsGreenHostLocalProof"]
    );
    assert_eq!(feature["qaRowsGreenDegraded"], qa["rowsGreenDegraded"]);

    let rows = qa["rows"]
        .as_array()
        .ok_or("x06-rag-qa rows must be an array")?;
    assert_eq!(rows.len(), 250);

    let mut ids = BTreeSet::<String>::new();
    let mut capability_counts = BTreeMap::<String, u64>::new();
    for row in rows {
        let id = row["id"]
            .as_str()
            .ok_or("each QA row must have an id")?
            .to_owned();
        assert!(ids.insert(id.clone()), "duplicate QA row id {id}");
        assert_eq!(row["verdict"], "pass", "{id} must remain a passing row");
        assert_eq!(row["recallAt5"], 1.0, "{id} must preserve recall@5 proof");
        assert_eq!(row["mrrAt10"], 1.0, "{id} must preserve mrr@10 proof");
        assert_eq!(row["ndcgAt10"], 1.0, "{id} must preserve ndcg@10 proof");
        assert!(
            row["expectedIds"]
                .as_array()
                .is_some_and(|ids| !ids.is_empty()),
            "{id} must retain expected evidence ids"
        );
        assert!(
            row["actualIds"]
                .as_array()
                .is_some_and(|ids| !ids.is_empty()),
            "{id} must retain actual evidence ids"
        );
        assert!(
            row["sourceRefs"]
                .as_array()
                .is_some_and(|refs| !refs.is_empty()),
            "{id} must retain source refs"
        );

        let state = row["capabilityState"]
            .as_str()
            .ok_or("each QA row must have a capabilityState")?;
        *capability_counts.entry(state.to_owned()).or_default() += 1;
    }

    assert_eq!(ids.len(), 250);
    assert_eq!(
        capability_counts.get("loaded").copied().unwrap_or_default(),
        qa["rowsGreenReal"].as_u64().unwrap_or_default()
    );
    assert_eq!(
        capability_counts
            .get("host-local-proof")
            .copied()
            .unwrap_or_default(),
        qa["rowsGreenHostLocalProof"].as_u64().unwrap_or_default()
    );
    assert_eq!(
        capability_counts
            .get("degraded")
            .copied()
            .unwrap_or_default(),
        qa["rowsGreenDegraded"].as_u64().unwrap_or_default()
    );
    assert_eq!(
        capability_counts.values().sum::<u64>(),
        qa["rowsGreen"].as_u64().unwrap_or_default()
    );
    Ok(())
}
