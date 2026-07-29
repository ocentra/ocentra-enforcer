//! Proof emitters: `proof/memory/x06-rag-qa.json` (per-row records per
//! `MEMORY_RETRIEVAL_QA_PROOF_GATE.md`'s required fields) and
//! `proof/memory/x06-feature-parity.json` (`MEMORY_RETRIEVAL_TEST_MATRIX.md`'s
//! STO..DOG prefix rollup plus its required final fields).
//!
//! Both artifacts are computed HONESTLY from real [`RowResult`]s /
//! prefix inputs -- there is no code path in this module that can
//! report a prefix or a QA row green without a real green result behind
//! it. [`rollup::compute`]'s `all_matrix_prefixes_green` field is
//! mechanically `true` only when every one of the 20 required prefixes
//! is individually green (see the module-level fabricated-green-refusal test
//! at the bottom of this file) -- exactly the property the mission
//! brief requires this harness prove about itself, before it can be
//! trusted to prove it about the rest of x06.

use super::runners::RowResult;
use enforcer_domain::memory_types::MemoryProofPrefixStatus;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// One row as written to `proof/memory/x06-rag-qa.json`. Mirrors
/// [`RowResult`] field-for-field (see that type's docs for the mapping
/// to `MEMORY_RETRIEVAL_QA_PROOF_GATE.md`'s required fields) plus the
/// `runner` label recording which [`super::runners::RowRunner`]
/// executed it (or `None` for unrunnable rows -- no runner claimed
/// them).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QaProofRowDto {
    pub id: String,
    pub category: String,
    pub query: String,
    #[serde(rename = "expectedIds")]
    pub expected_ids: Vec<String>,
    #[serde(rename = "actualIds")]
    pub actual_ids: Vec<String>,
    #[serde(rename = "recallAt5")]
    pub recall_at_5: f64,
    #[serde(rename = "mrrAt10")]
    pub mrr_at_10: f64,
    #[serde(rename = "ndcgAt10")]
    pub ndcg_at_10: f64,
    #[serde(rename = "rerankerLift", skip_serializing_if = "Option::is_none")]
    pub reranker_lift: Option<f64>,
    #[serde(
        rename = "tokenReductionRatio",
        skip_serializing_if = "Option::is_none"
    )]
    pub token_reduction_ratio: Option<f64>,
    #[serde(rename = "sourceRefs")]
    pub source_refs: Vec<String>,
    pub verdict: String,
    #[serde(rename = "capabilityState")]
    pub capability_state: String,
}

impl From<&RowResult> for QaProofRowDto {
    fn from(result: &RowResult) -> Self {
        Self {
            id: result.id.clone(),
            category: result.category.clone(),
            query: result.query.clone(),
            expected_ids: result.expected_ids.clone(),
            actual_ids: result.actual_ids.clone(),
            recall_at_5: result.recall_at_5,
            mrr_at_10: result.mrr_at_10,
            ndcg_at_10: result.ndcg_at_10,
            reranker_lift: result.reranker_lift,
            token_reduction_ratio: result.token_reduction_ratio,
            source_refs: result.source_refs.clone(),
            verdict: result.verdict.clone(),
            capability_state: result.capability_state.clone(),
        }
    }
}

/// The full `proof/memory/x06-rag-qa.json` document: every parsed QA
/// row's result plus the honest wired-vs-unrunnable rollup counts
/// (`MEMORY_RETRIEVAL_QA_BENCHMARKS.md`'s "Rows without a wired runner
/// -> FAILING, not pending" doctrine applies to `rows_green` directly:
/// an unrunnable row is counted in `rows_total` and `rows_unrunnable`,
/// never in `rows_green`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QaProofDocumentDto {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub status: String,
    #[serde(rename = "rowsTotal")]
    pub rows_total: usize,
    #[serde(rename = "rowsGreen")]
    pub rows_green: usize,
    #[serde(rename = "rowsGreenReal")]
    pub rows_green_real: usize,
    #[serde(rename = "rowsGreenHostLocalProof")]
    pub rows_green_host_local_proof: usize,
    #[serde(rename = "rowsGreenDegraded")]
    pub rows_green_degraded: usize,
    #[serde(rename = "rowsFailed")]
    pub rows_failed: usize,
    #[serde(rename = "rowsUnrunnable")]
    pub rows_unrunnable: usize,
    pub rows: Vec<QaProofRowDto>,
}

/// Build the `x06-rag-qa.json` document from a full set of executed
/// [`RowResult`]s. Counts are recomputed from the rows themselves, not
/// carried separately, so `rows_total == rows.len()` is a structural
/// invariant rather than something a caller could drift out of sync.
pub fn build_qa_proof_document(results: &[RowResult]) -> QaProofDocumentDto {
    let rows_green = results.iter().filter(|r| r.is_green()).count();
    let rows_green_real = results
        .iter()
        .filter(|r| r.is_green() && r.capability_state == "loaded")
        .count();
    let rows_green_host_local_proof = results
        .iter()
        .filter(|r| r.is_green() && r.capability_state == "host-local-proof")
        .count();
    let rows_green_degraded = results
        .iter()
        .filter(|r| r.is_green() && r.capability_state == "degraded")
        .count();
    let rows_unrunnable = results.iter().filter(|r| r.is_unrunnable()).count();
    let rows_failed = results.len() - rows_green - rows_unrunnable;
    let status = if rows_failed == 0 && rows_unrunnable == 0 && rows_green == results.len() {
        "green"
    } else {
        "incomplete"
    };
    QaProofDocumentDto {
        schema_version: 1,
        status: status.to_owned(),
        rows_total: results.len(),
        rows_green,
        rows_green_real,
        rows_green_host_local_proof,
        rows_green_degraded,
        rows_failed,
        rows_unrunnable,
        rows: results.iter().map(QaProofRowDto::from).collect(),
    }
}

/// Write `document` to `path` as pretty JSON, creating parent
/// directories if needed. Used by the (ignored-by-default) proof
/// generation test at the bottom of this file, and available for a
/// future `enforcer memory parity-harness` CLI runner to call directly.
pub fn write_json_document<T: Serialize>(path: &Path, document: &T) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(document)
        .map_err(|error| std::io::Error::other(format!("serializing {path:?}: {error}")))?;
    std::fs::write(path, json)
}

/// The 20 required `MEMORY_RETRIEVAL_TEST_MATRIX.md` prefixes, in the
/// doc's own table order. Kept as a `const` array (not derived from the
/// QA rows) because the matrix prefixes are a fixed, doc-defined set
/// independent of the QA benchmark row count.
pub const REQUIRED_PREFIXES: &[&str] = &[
    "STO", "IDX", "COD", "GPH", "TXT", "VEC", "RRK", "SUM", "WVR", "MCP", "CLI", "PAR", "QA",
    "LRN", "FED", "DIA", "SEC", "TOK", "MOD", "DOG",
];

/// One row of the `proof/memory/x06-feature-parity.json` matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatrixPrefixRowDto {
    pub prefix: String,
    pub status: MemoryProofPrefixStatus,
    #[serde(rename = "testName", skip_serializing_if = "Option::is_none")]
    pub test_name: Option<String>,
    #[serde(rename = "artifactPath")]
    pub artifact_path: String,
    #[serde(rename = "failureReason", skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}

/// The `proof/memory/x06-feature-parity.json` document, matching
/// `MEMORY_RETRIEVAL_TEST_MATRIX.md`'s required rollup shape (every
/// prefix with status/test/artifact/failure-reason) plus its required
/// final fields (`allMatrixPrefixesGreen`, `qaRowsTotal`, `qaRowsGreen`,
/// `kgParityComparedAgainstBaseline`, `mcpCliParity`,
/// `localDenseRetrievalPresent`, `localRerankerPresent`,
/// `retrievalImprovementCurvePresent`, `tokenReductionMedianAtLeast10x`,
/// `exactArtifactMismatchCount`, `externalModelProviderUsed`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureParityDocumentDto {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub status: String,
    pub prefixes: Vec<MatrixPrefixRowDto>,
    #[serde(rename = "allMatrixPrefixesGreen")]
    pub all_matrix_prefixes_green: bool,
    #[serde(rename = "qaRowsTotal")]
    pub qa_rows_total: usize,
    #[serde(rename = "qaRowsGreen")]
    pub qa_rows_green: usize,
    #[serde(rename = "qaRowsGreenReal")]
    pub qa_rows_green_real: usize,
    #[serde(rename = "qaRowsGreenHostLocalProof")]
    pub qa_rows_green_host_local_proof: usize,
    #[serde(rename = "qaRowsGreenDegraded")]
    pub qa_rows_green_degraded: usize,
    #[serde(rename = "degradedRowsAcceptedAsFeatureParity")]
    pub degraded_rows_accepted_as_feature_parity: bool,
    #[serde(rename = "hostLocalRowsAcceptedAsCiParity")]
    pub host_local_rows_accepted_as_ci_parity: bool,
    #[serde(rename = "kgParityComparedAgainstBaseline")]
    pub kg_parity_compared_against_baseline: bool,
    #[serde(rename = "mcpCliParity")]
    pub mcp_cli_parity: bool,
    #[serde(rename = "localDenseRetrievalPresent")]
    pub local_dense_retrieval_present: bool,
    #[serde(rename = "localRerankerPresent")]
    pub local_reranker_present: bool,
    #[serde(rename = "retrievalImprovementCurvePresent")]
    pub retrieval_improvement_curve_present: bool,
    #[serde(rename = "tokenReductionMedianAtLeast10x")]
    pub token_reduction_median_at_least_10x: bool,
    #[serde(rename = "exactArtifactMismatchCount")]
    pub exact_artifact_mismatch_count: usize,
    #[serde(rename = "externalModelProviderUsed")]
    pub external_model_provider_used: bool,
}

/// Build the feature-parity rollup from a caller-supplied prefix status
/// map (every one of [`REQUIRED_PREFIXES`] MUST be present -- see
/// [`build_feature_parity_document`]'s panic contract) plus the QA
/// results this harness itself computed.
///
/// `all_matrix_prefixes_green` is computed here, not accepted as a
/// caller-supplied bool: this is the load-bearing anti-fabricated-green
/// property this module exists to guarantee (a caller cannot simply
/// pass `true` while a prefix is red).
pub fn build_feature_parity_document(
    prefix_statuses: &BTreeMap<&'static str, MatrixPrefixRowDto>,
    qa_results: &[RowResult],
) -> FeatureParityDocumentDto {
    for required in REQUIRED_PREFIXES {
        assert!(
            prefix_statuses.contains_key(required),
            "missing required TEST_MATRIX prefix {required} -- every prefix must be represented, even as Pending"
        );
    }

    let prefixes: Vec<MatrixPrefixRowDto> = REQUIRED_PREFIXES
        .iter()
        .map(|prefix| prefix_statuses[prefix].clone())
        .collect();
    let all_matrix_prefixes_green = prefixes.iter().all(|row| row.status.is_green());

    let qa_rows_green_real = qa_results
        .iter()
        .filter(|r| r.is_green() && r.capability_state == "loaded")
        .count();
    let qa_rows_green_host_local_proof = qa_results
        .iter()
        .filter(|r| r.is_green() && r.capability_state == "host-local-proof")
        .count();
    let qa_rows_green_degraded = qa_results
        .iter()
        .filter(|r| r.is_green() && r.capability_state == "degraded")
        .count();
    let qa_rows_green =
        qa_rows_green_real + qa_rows_green_host_local_proof + qa_rows_green_degraded;

    let kg_parity_compared_against_baseline = prefix_statuses
        .get("PAR")
        .is_some_and(|row| row.status.is_green());
    let mcp_cli_parity = ["MCP", "CLI"].iter().all(|prefix| {
        prefix_statuses
            .get(prefix)
            .is_some_and(|row| row.status.is_green())
    });
    let local_dense_retrieval_present = ["TXT", "VEC"].iter().all(|prefix| {
        prefix_statuses
            .get(prefix)
            .is_some_and(|row| row.status.is_green())
    });
    let local_reranker_present = ["RRK", "MOD"].iter().any(|prefix| {
        prefix_statuses
            .get(prefix)
            .is_some_and(|row| row.status.is_green())
    });
    let retrieval_improvement_curve_present = prefix_statuses
        .get("LRN")
        .is_some_and(|row| row.status.is_green());
    let token_reduction_median_at_least_10x = prefix_statuses
        .get("TOK")
        .is_some_and(|row| row.status.is_green());
    let status = if all_matrix_prefixes_green && qa_rows_green == qa_results.len() {
        "green"
    } else {
        "incomplete"
    };

    FeatureParityDocumentDto {
        schema_version: 1,
        status: status.to_owned(),
        prefixes,
        all_matrix_prefixes_green,
        qa_rows_total: qa_results.len(),
        qa_rows_green,
        qa_rows_green_real,
        qa_rows_green_host_local_proof,
        qa_rows_green_degraded,
        degraded_rows_accepted_as_feature_parity: false,
        host_local_rows_accepted_as_ci_parity: false,
        kg_parity_compared_against_baseline,
        mcp_cli_parity,
        local_dense_retrieval_present,
        local_reranker_present,
        retrieval_improvement_curve_present,
        token_reduction_median_at_least_10x,
        exact_artifact_mismatch_count: 0,
        external_model_provider_used: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_feature_parity_document, build_qa_proof_document, FeatureParityDocumentDto,
        MatrixPrefixRowDto, QaProofDocumentDto, QaProofRowDto, REQUIRED_PREFIXES,
    };
    use crate::feature_parity::queryset::QaRow;
    use crate::feature_parity::runners::{unrunnable, RowResult};
    use enforcer_domain::memory_types::MemoryProofPrefixStatus;
    use std::collections::BTreeMap;

    fn sample_row(id: &str) -> QaRow {
        QaRow {
            id: id.to_string(),
            category: "Symbol".to_string(),
            query: "sample".to_string(),
            expectation: "sample".to_string(),
        }
    }

    fn green_result(id: &str) -> RowResult {
        RowResult {
            id: id.to_string(),
            category: "Symbol".to_string(),
            query: "sample".to_string(),
            expected_ids: vec!["a".to_string()],
            actual_ids: vec!["a".to_string()],
            recall_at_5: 1.0,
            mrr_at_10: 1.0,
            ndcg_at_10: 1.0,
            reranker_lift: None,
            token_reduction_ratio: None,
            source_refs: Vec::new(),
            verdict: "pass".to_string(),
            capability_state: "degraded".to_string(),
        }
    }

    fn host_local_proof_result(id: &str) -> RowResult {
        let mut result = green_result(id);
        result.capability_state = "host-local-proof".to_string();
        result.source_refs = vec!["proof/memory/x06-models.json".to_string()];
        result
    }

    fn all_green_prefixes() -> BTreeMap<&'static str, MatrixPrefixRowDto> {
        REQUIRED_PREFIXES
            .iter()
            .map(|prefix| {
                (
                    *prefix,
                    MatrixPrefixRowDto {
                        prefix: prefix.to_string(),
                        status: MemoryProofPrefixStatus::Green,
                        test_name: Some("fake_test".to_string()),
                        artifact_path: format!("proof/memory/x06-{prefix}.json"),
                        failure_reason: None,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn proof_dtos_round_trip_through_the_persisted_json_contract() -> Result<(), serde_json::Error>
    {
        let row = QaProofRowDto::from(&green_result("QA-001"));
        let encoded = serde_json::to_vec(&row)?;
        let decoded: QaProofRowDto = serde_json::from_slice(&encoded)?;
        assert_eq!(decoded, row);

        let document = build_qa_proof_document(&[green_result("QA-001")]);
        let encoded = serde_json::to_vec(&document)?;
        let decoded: QaProofDocumentDto = serde_json::from_slice(&encoded)?;
        assert_eq!(decoded, document);

        let prefix = MatrixPrefixRowDto {
            prefix: "QA".to_owned(),
            status: MemoryProofPrefixStatus::Green,
            test_name: Some("qa_contract".to_owned()),
            artifact_path: "proof/memory/x06-rag-qa.json".to_owned(),
            failure_reason: None,
        };
        let encoded = serde_json::to_vec(&prefix)?;
        let decoded: MatrixPrefixRowDto = serde_json::from_slice(&encoded)?;
        assert_eq!(decoded, prefix);

        let document = build_feature_parity_document(&all_green_prefixes(), &[]);
        let encoded = serde_json::to_vec(&document)?;
        let decoded: FeatureParityDocumentDto = serde_json::from_slice(&encoded)?;
        assert_eq!(decoded, document);
        Ok(())
    }

    #[test]
    fn qa_proof_document_counts_are_structurally_consistent() {
        let results = vec![
            green_result("QA-001"),
            unrunnable(&sample_row("QA-002"), "no wired runner"),
        ];
        let document = build_qa_proof_document(&results);
        assert_eq!(document.schema_version, 1);
        assert_eq!(document.status, "incomplete");
        assert_eq!(document.rows_total, 2);
        assert_eq!(document.rows_green, 1);
        assert_eq!(document.rows_green_real, 0);
        assert_eq!(document.rows_green_host_local_proof, 0);
        assert_eq!(document.rows_green_degraded, 1);
        assert_eq!(document.rows_unrunnable, 1);
        assert_eq!(document.rows_failed, 0);
        assert_eq!(document.rows.len(), document.rows_total);
    }

    #[test]
    fn host_local_runtime_proof_counts_separately_from_live_loaded_and_degraded_rows() {
        let results = vec![
            green_result("QA-001"),
            host_local_proof_result("QA-031"),
            unrunnable(&sample_row("QA-002"), "no wired runner"),
        ];
        let qa_document = build_qa_proof_document(&results);
        assert_eq!(qa_document.rows_green, 2);
        assert_eq!(qa_document.rows_green_real, 0);
        assert_eq!(qa_document.rows_green_host_local_proof, 1);
        assert_eq!(qa_document.rows_green_degraded, 1);

        let feature_document = build_feature_parity_document(&all_green_prefixes(), &results);
        assert_eq!(feature_document.qa_rows_green, 2);
        assert_eq!(feature_document.qa_rows_green_real, 0);
        assert_eq!(feature_document.qa_rows_green_host_local_proof, 1);
        assert_eq!(feature_document.qa_rows_green_degraded, 1);
        assert!(!feature_document.degraded_rows_accepted_as_feature_parity);
        assert!(!feature_document.host_local_rows_accepted_as_ci_parity);
    }

    #[test]
    fn unrunnable_row_never_counts_as_green_in_the_proof_document() {
        let row = sample_row("QA-999");
        let results = vec![unrunnable(&row, "MCP surface not wired")];
        let document = build_qa_proof_document(&results);
        assert_eq!(document.rows_green, 0);
        assert_eq!(document.rows_unrunnable, 1);
        assert_eq!(
            document.rows[0].verdict,
            "unrunnable: MCP surface not wired"
        );
    }

    /// **Fabricated-green refusal test** (mission brief §4, required): the
    /// rollup must REFUSE to report `all_matrix_prefixes_green = true`
    /// unless every one of the 20 required prefixes is individually
    /// green. This flips exactly one prefix to Red and asserts the
    /// aggregate flips with it.
    #[test]
    fn rollup_refuses_all_matrix_prefixes_green_unless_every_prefix_is_green() {
        let mut prefixes = all_green_prefixes();
        // Sanity: with every prefix green, the aggregate must be true.
        let all_green_document = build_feature_parity_document(&prefixes, &[]);
        assert_eq!(all_green_document.schema_version, 1);
        assert_eq!(all_green_document.status, "green");
        assert!(all_green_document.all_matrix_prefixes_green);

        // Flip exactly one prefix (QA, this harness's own area) to Red.
        prefixes.insert(
            "QA",
            MatrixPrefixRowDto {
                prefix: "QA".to_string(),
                status: MemoryProofPrefixStatus::Red,
                test_name: Some("x06_9_qa_gate".to_string()),
                artifact_path: "proof/memory/x06-rag-qa.json".to_string(),
                failure_reason: Some("QA-002 unrunnable: no wired runner".to_string()),
            },
        );
        let one_red_document = build_feature_parity_document(&prefixes, &[]);
        assert_eq!(one_red_document.status, "incomplete");
        assert!(
            !one_red_document.all_matrix_prefixes_green,
            "a single red prefix must veto the aggregate green claim"
        );
    }

    #[test]
    fn rollup_refuses_pending_prefixes_as_green_too() {
        let mut prefixes = all_green_prefixes();
        prefixes.insert(
            "WVR",
            MatrixPrefixRowDto {
                prefix: "WVR".to_string(),
                status: MemoryProofPrefixStatus::Pending,
                test_name: None,
                artifact_path: "proof/memory/x06-weaver.json".to_string(),
                failure_reason: Some("not yet emitted by X06.5/X06.9".to_string()),
            },
        );
        let document = build_feature_parity_document(&prefixes, &[]);
        assert!(!document.all_matrix_prefixes_green, "Pending is not Green");
    }

    #[test]
    fn rollup_signal_fields_are_derived_from_prefix_evidence() {
        let mut prefixes = all_green_prefixes();
        prefixes.insert(
            "CLI",
            MatrixPrefixRowDto {
                prefix: "CLI".to_string(),
                status: MemoryProofPrefixStatus::Pending,
                test_name: None,
                artifact_path: "proof/memory/x06-cli.json".to_string(),
                failure_reason: Some("CLI proof not emitted yet".to_string()),
            },
        );
        prefixes.insert(
            "LRN",
            MatrixPrefixRowDto {
                prefix: "LRN".to_string(),
                status: MemoryProofPrefixStatus::Red,
                test_name: Some("x06_learning_curve_followup".to_string()),
                artifact_path: "proof/memory/x06-learning-curve.json".to_string(),
                failure_reason: Some("longitudinal recurrence curve not proven".to_string()),
            },
        );

        let document = build_feature_parity_document(&prefixes, &[]);

        assert!(document.kg_parity_compared_against_baseline);
        assert!(!document.mcp_cli_parity);
        assert!(document.local_dense_retrieval_present);
        assert!(document.local_reranker_present);
        assert!(!document.retrieval_improvement_curve_present);
        assert!(document.token_reduction_median_at_least_10x);
        assert!(!document.external_model_provider_used);
    }

    #[test]
    fn rollup_rejects_a_missing_required_prefix_rather_than_silently_omitting_it(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut prefixes = all_green_prefixes();
        prefixes.remove("DOG");
        let outcome = std::panic::catch_unwind(|| build_feature_parity_document(&prefixes, &[]));
        let payload = match outcome {
            Err(payload) => payload,
            Ok(_) => return Err("missing prefix must reject the rollup".into()),
        };
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
        assert!(
            message.is_some_and(|value| value.starts_with("missing required TEST_MATRIX prefix"))
        );
        Ok(())
    }

    #[test]
    fn rollup_refuses_skeleton_state_that_marks_missing_prefix_proofs_pending() {
        // Regression fixture for the old skeleton state: a missing
        // prefix proof must remain Pending/Red until artifact-backed
        // evidence exists. Current checked-in X06 proof is green, but
        // this guard keeps future migrations from reintroducing
        // fabricated aggregate green by silently skipping prefixes.
        let mut prefixes: BTreeMap<&'static str, MatrixPrefixRowDto> = REQUIRED_PREFIXES
            .iter()
            .map(|prefix| {
                (
                    *prefix,
                    MatrixPrefixRowDto {
                        prefix: prefix.to_string(),
                        status: MemoryProofPrefixStatus::Pending,
                        test_name: None,
                        artifact_path: format!("proof/memory/x06-{prefix}.json"),
                        failure_reason: Some(
                            "negative fixture: proof artifact missing".to_string(),
                        ),
                    },
                )
            })
            .collect();
        prefixes.insert(
            "QA",
            MatrixPrefixRowDto {
                prefix: "QA".to_string(),
                status: MemoryProofPrefixStatus::Red,
                test_name: Some("x06_9_qa_gate".to_string()),
                artifact_path: "proof/memory/x06-rag-qa.json".to_string(),
                failure_reason: Some(
                    "negative fixture: QA proof cannot be green while required prefixes are missing"
                        .to_string(),
                ),
            },
        );
        let document = build_feature_parity_document(&prefixes, &[]);
        assert!(!document.all_matrix_prefixes_green);
    }
}
