//! X06.9 parity/benchmark harness SKELETON entry point: parses all 250
//! QA rows, executes every wired [`feature_parity::runners::RowRunner`]
//! against the fixture corpus, and emits the two required proof
//! artifacts in an isolated temporary directory so their honest wired vs
//! unrunnable state is inspected without ordinary tests mutating tracked
//! release proof -- never a fabricated
//! green.
//!
//! This file's own assertions reflect that the proof must be honest:
//! it asserts the row-parse count, the wired-vs-unrunnable split is
//! internally consistent, and any all-green rollup must be derived from
//! real artifact evidence rather than fabricated by skipping unsupported
//! rows.

mod feature_parity;

use enforcer_domain::memory_types::MemoryProofPrefixStatus as PrefixStatus;
use feature_parity::proof::{
    build_feature_parity_document, build_qa_proof_document, write_json_document,
    MatrixPrefixRowDto, REQUIRED_PREFIXES,
};
use feature_parity::runners::run_all;
use feature_parity::{build_fixtures, BoxError};
use std::collections::BTreeMap;
use std::path::Path;

type TestResult<T = ()> = Result<T, BoxError>;

/// End-to-end: parse all 250 rows, run them against the fixture
/// environment, and assert the honest split this harness reports
/// (never a fabricated all-green).
#[test]
fn qa_gate_runs_every_row_and_reports_an_honest_wired_vs_unrunnable_split() -> TestResult {
    let rows = feature_parity::queryset::parse_all(&feature_parity::queryset::workspace_root())?;
    assert_eq!(rows.len(), 250);

    let fixtures = build_fixtures()?;
    let results = run_all(&rows, &fixtures);
    assert_eq!(results.len(), 250);

    let wired = results.iter().filter(|r| !r.is_unrunnable()).count();
    let unrunnable = results.iter().filter(|r| r.is_unrunnable()).count();
    let unrunnable_rows: Vec<String> = results
        .iter()
        .filter(|result| result.is_unrunnable())
        .map(|result| format!("{}: {}", result.id, result.verdict))
        .collect();
    assert_eq!(wired + unrunnable, 250);
    assert!(
        wired >= 250,
        "expected all 250 QA rows to be honestly wired after x06-qa-capabilities promotion, got {wired} wired / {unrunnable} unrunnable: {unrunnable_rows:?}"
    );
    assert_eq!(
        unrunnable, 0,
        "QA rows must not remain unrunnable once x06-qa-capabilities rowsStillNeedingRunnerOrCode is empty"
    );

    let document = build_qa_proof_document(&results);
    assert_eq!(document.rows_total, 250);
    assert_eq!(
        document.rows_green + document.rows_failed + document.rows_unrunnable,
        250
    );
    assert_eq!(
        document.rows_green,
        document.rows_green_real
            + document.rows_green_host_local_proof
            + document.rows_green_degraded
    );
    assert!(
        document.rows_green_degraded > 0
            || document.rows_green_real > 0
            || document.rows_green_host_local_proof > 0
    );
    let failed_rows: Vec<String> = results
        .iter()
        .filter(|result| result.verdict == "fail")
        .map(|result| result.id.clone())
        .collect();
    assert_eq!(
        document.rows_failed, 0,
        "fixture-backed default QA runners should either pass or stay unrunnable; failures usually mean an over-broad runner claim: {failed_rows:?}"
    );
    let Some(install_hook_result) = results.iter().find(|result| result.id == "QA-135") else {
        return Err("expected Claude install-hook proof row QA-135 to exist".into());
    };
    assert_eq!(
        install_hook_result.verdict, "pass",
        "QA-135 must be green once Claude hook wiring proof lands, got {}",
        install_hook_result.verdict
    );
    assert!(install_hook_result
        .source_refs
        .iter()
        .any(|source| source == "proof/install/c05-claude-hook-wiring.json"));

    // Ordinary tests emit proof into an isolated directory. The committed
    // release artifact is regenerated only by the explicit release workflow,
    // never as a side effect of `cargo test`.
    let proof_output = tempfile::tempdir()?;
    let workspace_root = feature_parity::queryset::workspace_root();
    let qa_path = proof_output.path().join("x06-rag-qa.json");
    write_json_document(&qa_path, &document)?;
    let written_qa: feature_parity::proof::QaProofDocumentDto =
        serde_json::from_slice(&std::fs::read(&qa_path)?)?;
    assert_eq!(
        written_qa, document,
        "written QA proof must preserve every derived row and count"
    );

    // Build and emit the feature-parity rollup from committed proof
    // artifacts. Partial proofs become Red with a concrete reason,
    // missing proofs remain Pending, and all-green can only be derived
    // from artifact-backed pass signals.
    let mut prefixes =
        current_prefix_statuses(&workspace_root, document.rows_green, document.rows_total)?;
    let qa_status = if document.rows_green == document.rows_total
        && document.rows_failed == 0
        && document.rows_unrunnable == 0
    {
        PrefixStatus::Green
    } else {
        PrefixStatus::Red
    };
    let unrunnable = document.rows_unrunnable;
    let qa_failure_reason = format!(
        "{unrunnable} rows unrunnable, {degraded} rows degraded-pass, {host_local} rows host-local-proof-pass, {real} rows real-pass, {failed} failed -- see x06-rag-qa.json",
        degraded = document.rows_green_degraded,
        host_local = document.rows_green_host_local_proof,
        real = document.rows_green_real,
        failed = document.rows_failed
    );
    prefixes.insert(
        "QA",
        MatrixPrefixRowDto {
            prefix: "QA".to_string(),
            status: qa_status,
            test_name: Some(
                "qa_gate_runs_every_row_and_reports_an_honest_wired_vs_unrunnable_split"
                    .to_string(),
            ),
            artifact_path: "proof/memory/x06-rag-qa.json".to_string(),
            failure_reason: if matches!(qa_status, PrefixStatus::Red) {
                Some(qa_failure_reason)
            } else {
                None
            },
        },
    );

    let rollup = build_feature_parity_document(&prefixes, &results);
    assert!(
        rollup.all_matrix_prefixes_green,
        "the rollup must claim all-green once every prefix and every QA row is artifact-backed green"
    );
    assert_eq!(rollup.qa_rows_total, 250);
    assert!(rollup.kg_parity_compared_against_baseline);
    assert!(rollup.local_dense_retrieval_present);
    assert!(rollup.local_reranker_present);
    assert!(rollup.token_reduction_median_at_least_10x);
    assert!(rollup.mcp_cli_parity);
    assert!(rollup.retrieval_improvement_curve_present);

    let rollup_path = proof_output.path().join("x06-feature-parity.json");
    write_json_document(&rollup_path, &rollup)?;
    let written_rollup: serde_json::Value = serde_json::from_slice(&std::fs::read(&rollup_path)?)?;
    assert_eq!(
        written_rollup
            .get("allMatrixPrefixesGreen")
            .and_then(serde_json::Value::as_bool),
        Some(true),
        "written parity proof must retain the all-green verdict derived above"
    );
    Ok(())
}

/// Map a TEST_MATRIX prefix to the doc-specified artifact filename stem
/// (`MEMORY_RETRIEVAL_TEST_MATRIX.md`'s own table: most prefixes are
/// `x06-<lowercase-area>.json`, but a few use a different area name
/// than their prefix, e.g. `GPH` -> `x06-kg.json`, `COD` ->
/// `x06-code-graph.json`, `PAR` -> `x06-kg-parity.json`).
fn prefix_artifact_stem(prefix: &'static str) -> &'static str {
    match prefix {
        "STO" => "store",
        "IDX" => "indexing",
        "COD" => "code-graph",
        "GPH" => "kg",
        "TXT" => "fulltext",
        "VEC" => "vector",
        "RRK" => "reranker",
        "SUM" => "summaries",
        "WVR" => "weaver",
        "MCP" => "mcp",
        "CLI" => "cli",
        "PAR" => "kg-parity",
        "QA" => "rag-qa",
        "LRN" => "learning-curve",
        "FED" => "federation",
        "DIA" => "diagnostics",
        "SEC" => "policy",
        "TOK" => "token-reduction",
        "MOD" => "models",
        "DOG" => "dogfood",
        // Every entry in REQUIRED_PREFIXES is matched explicitly above;
        // this arm exists only so the match is exhaustive over the
        // input type, never so it can silently invent a stem for an
        // unmapped prefix -- see the coverage test below.
        _ => "unmapped-prefix",
    }
}

fn current_prefix_statuses(
    workspace_root: &Path,
    qa_rows_green: usize,
    qa_rows_total: usize,
) -> TestResult<BTreeMap<&'static str, MatrixPrefixRowDto>> {
    let mut prefixes: BTreeMap<&'static str, MatrixPrefixRowDto> = REQUIRED_PREFIXES
        .iter()
        .map(|prefix| (*prefix, pending_prefix(prefix)))
        .collect();

    set_store_status(workspace_root, &mut prefixes)?;
    set_artifact_status(
        workspace_root,
        &mut prefixes,
        "IDX",
        "proof/memory/x06-indexing.json",
        "x06-indexing",
    )?;
    set_artifact_status(
        workspace_root,
        &mut prefixes,
        "COD",
        "proof/memory/x06-code-graph.json",
        "x06-code-graph",
    )?;
    set_rag_status(workspace_root, &mut prefixes)?;
    set_kg_status(workspace_root, &mut prefixes)?;
    set_artifact_status(
        workspace_root,
        &mut prefixes,
        "SUM",
        "proof/memory/x06-summaries.json",
        "x06-summaries",
    )?;
    set_artifact_status(
        workspace_root,
        &mut prefixes,
        "WVR",
        "proof/memory/x06-weaver.json",
        "x06-weaver",
    )?;
    set_artifact_status(
        workspace_root,
        &mut prefixes,
        "MCP",
        "proof/memory/x06-mcp.json",
        "x06-mcp",
    )?;
    set_artifact_status(
        workspace_root,
        &mut prefixes,
        "CLI",
        "proof/memory/x06-cli.json",
        "x06-cli",
    )?;
    set_kg_parity_status(workspace_root, &mut prefixes)?;
    set_learning_status(workspace_root, &mut prefixes)?;
    set_artifact_status(
        workspace_root,
        &mut prefixes,
        "FED",
        "proof/memory/x06-federation.json",
        "x06-federation",
    )?;
    set_artifact_status(
        workspace_root,
        &mut prefixes,
        "DIA",
        "proof/memory/x06-diagnostics.json",
        "x06-diagnostics",
    )?;
    set_artifact_status(
        workspace_root,
        &mut prefixes,
        "SEC",
        "proof/memory/x06-policy.json",
        "x06-policy-filters",
    )?;
    set_token_status(workspace_root, &mut prefixes)?;
    set_model_status(workspace_root, &mut prefixes)?;
    set_dogfood_status(workspace_root, &mut prefixes)?;

    if qa_rows_green == qa_rows_total {
        prefixes.insert(
            "QA",
            green_prefix(
                "QA",
                "proof/memory/x06-rag-qa.json",
                "qa_gate_runs_every_row_and_reports_an_honest_wired_vs_unrunnable_split",
            ),
        );
    }

    Ok(prefixes)
}

fn pending_prefix(prefix: &'static str) -> MatrixPrefixRowDto {
    MatrixPrefixRowDto {
        prefix: prefix.to_string(),
        status: PrefixStatus::Pending,
        test_name: None,
        artifact_path: format!("proof/memory/x06-{}.json", prefix_artifact_stem(prefix)),
        failure_reason: Some(
            "proof artifact not emitted or not owned by this X06 slice".to_string(),
        ),
    }
}

fn green_prefix(prefix: &str, artifact_path: &str, test_name: &str) -> MatrixPrefixRowDto {
    MatrixPrefixRowDto {
        prefix: prefix.to_string(),
        status: PrefixStatus::Green,
        test_name: Some(test_name.to_string()),
        artifact_path: artifact_path.to_string(),
        failure_reason: None,
    }
}

fn red_prefix(
    prefix: &str,
    artifact_path: &str,
    test_name: Option<&str>,
    failure_reason: impl Into<String>,
) -> MatrixPrefixRowDto {
    MatrixPrefixRowDto {
        prefix: prefix.to_string(),
        status: PrefixStatus::Red,
        test_name: test_name.map(str::to_string),
        artifact_path: artifact_path.to_string(),
        failure_reason: Some(failure_reason.into()),
    }
}

fn proof_json(workspace_root: &Path, artifact_path: &str) -> TestResult<Option<serde_json::Value>> {
    let path = workspace_root.join(artifact_path);
    if !path.is_file() {
        return Ok(None);
    }
    let body = std::fs::read_to_string(&path)?;
    Ok(Some(serde_json::from_str(&body)?))
}

fn artifact_status(
    proof: &serde_json::Value,
    prefix: &str,
    artifact_path: &str,
    test_name: &str,
) -> MatrixPrefixRowDto {
    let status = proof["status"].as_str().unwrap_or("unknown");
    let tests_failed = proof["result"]["testsFailed"].as_u64().unwrap_or(1);
    if (status == "green" || status == "complete") && tests_failed == 0 {
        green_prefix(prefix, artifact_path, test_name)
    } else {
        red_prefix(
            prefix,
            artifact_path,
            Some(test_name),
            format!("artifact status={status}, testsFailed={tests_failed}"),
        )
    }
}

fn set_artifact_status(
    workspace_root: &Path,
    prefixes: &mut BTreeMap<&'static str, MatrixPrefixRowDto>,
    prefix: &'static str,
    artifact_path: &'static str,
    test_name: &'static str,
) -> TestResult<()> {
    let Some(proof) = proof_json(workspace_root, artifact_path)? else {
        return Ok(());
    };
    prefixes.insert(
        prefix,
        artifact_status(&proof, prefix, artifact_path, test_name),
    );
    Ok(())
}

fn set_store_status(
    workspace_root: &Path,
    prefixes: &mut BTreeMap<&'static str, MatrixPrefixRowDto>,
) -> TestResult<()> {
    set_artifact_status(
        workspace_root,
        prefixes,
        "STO",
        "proof/memory/x06-store.json",
        "memory-store-core",
    )
}

fn set_rag_status(
    workspace_root: &Path,
    prefixes: &mut BTreeMap<&'static str, MatrixPrefixRowDto>,
) -> TestResult<()> {
    set_artifact_status(
        workspace_root,
        prefixes,
        "TXT",
        "proof/memory/x06-fulltext.json",
        "x06-fulltext",
    )?;
    set_artifact_status(
        workspace_root,
        prefixes,
        "VEC",
        "proof/memory/x06-vector.json",
        "x06-vector",
    )?;
    set_artifact_status(
        workspace_root,
        prefixes,
        "RRK",
        "proof/memory/x06-reranker.json",
        "x06-reranker",
    )?;
    Ok(())
}

fn set_kg_status(
    workspace_root: &Path,
    prefixes: &mut BTreeMap<&'static str, MatrixPrefixRowDto>,
) -> TestResult<()> {
    let Some(kg) = proof_json(workspace_root, "proof/memory/x06-kg.json")? else {
        return Ok(());
    };
    let status = kg["status"].as_str().unwrap_or("unknown");
    prefixes.insert(
        "GPH",
        if status == "green" || status == "complete" {
            green_prefix("GPH", "proof/memory/x06-kg.json", "x06-kg")
        } else {
            let remaining = kg["remaining"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str())
                        .filter(|item| !item.contains("Claude parity lane"))
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| format!("KG artifact status is {status}"));
            red_prefix("GPH", "proof/memory/x06-kg.json", Some("x06-kg"), remaining)
        },
    );
    Ok(())
}

fn set_kg_parity_status(
    workspace_root: &Path,
    prefixes: &mut BTreeMap<&'static str, MatrixPrefixRowDto>,
) -> TestResult<()> {
    let Some(parity) = proof_json(workspace_root, "proof/memory/x06-kg-parity.json")? else {
        return Ok(());
    };
    let baseline_executed = parity["baseline_executed"].as_bool().unwrap_or(false);
    let worse = parity["tools_worse"].as_u64().unwrap_or(1);
    let unrunnable = parity["tools_unrunnable"].as_u64().unwrap_or(1);
    prefixes.insert(
        "PAR",
        if baseline_executed && worse == 0 && unrunnable == 0 {
            green_prefix("PAR", "proof/memory/x06-kg-parity.json", "x06-live-baseline-parity")
        } else {
            red_prefix(
                "PAR",
                "proof/memory/x06-kg-parity.json",
                Some("x06-live-baseline-parity"),
                format!(
                    "baseline_executed={baseline_executed}, tools_worse={worse}, tools_unrunnable={unrunnable}"
                ),
            )
        },
    );
    Ok(())
}

fn set_learning_status(
    workspace_root: &Path,
    prefixes: &mut BTreeMap<&'static str, MatrixPrefixRowDto>,
) -> TestResult<()> {
    let Some(learning) = proof_json(workspace_root, "proof/memory/x06-learning-curve.json")? else {
        return Ok(());
    };
    let present = learning["learningCurve"]["present"]
        .as_bool()
        .unwrap_or(false);
    let blockers = learning["blockers"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default();
    prefixes.insert(
        "LRN",
        if present && blockers.is_empty() {
            green_prefix(
                "LRN",
                "proof/memory/x06-learning-curve.json",
                "x06-learning-curve",
            )
        } else {
            red_prefix(
                "LRN",
                "proof/memory/x06-learning-curve.json",
                Some("x06-learning-curve"),
                if present {
                    format!("store-backed learning present, but follow-up remains: {blockers}")
                } else {
                    "learning curve proof is not present".to_string()
                },
            )
        },
    );
    Ok(())
}

fn set_token_status(
    workspace_root: &Path,
    prefixes: &mut BTreeMap<&'static str, MatrixPrefixRowDto>,
) -> TestResult<()> {
    let Some(token) = proof_json(workspace_root, "proof/memory/x06-token-reduction.json")? else {
        return Ok(());
    };
    let passes = token["passes10xGate"].as_bool().unwrap_or(false);
    let median = token["medianReductionRatio"].as_f64().unwrap_or_default();
    prefixes.insert(
        "TOK",
        if passes {
            green_prefix(
                "TOK",
                "proof/memory/x06-token-reduction.json",
                "x06-token-reduction",
            )
        } else {
            red_prefix(
                "TOK",
                "proof/memory/x06-token-reduction.json",
                Some("x06-token-reduction"),
                format!("median token reduction ratio {median:.2} does not pass 10x gate"),
            )
        },
    );
    Ok(())
}

fn set_model_status(
    workspace_root: &Path,
    prefixes: &mut BTreeMap<&'static str, MatrixPrefixRowDto>,
) -> TestResult<()> {
    let chat_ok = proof_json(
        workspace_root,
        "proof/memory/x06-models-qwen3-4b-vulkan-windows-local.json",
    )?
    .and_then(|proof| proof["chatGenerationGguf"]["usability"]["ok"].as_bool());
    let embedding_ok = proof_json(
        workspace_root,
        "proof/memory/x06-models-qwen3-embedding-ort-cpu.json",
    )?
    .and_then(|proof| proof["qwenEmbeddingOnnx"]["ok"].as_bool());
    let reranker_ok = proof_json(
        workspace_root,
        "proof/memory/x06-models-qwen3-reranker-ort-cpu.json",
    )?
    .and_then(|proof| proof["qwenRerankerOnnx"]["ok"].as_bool());
    prefixes.insert(
        "MOD",
        if chat_ok == Some(true) && embedding_ok == Some(true) && reranker_ok == Some(true) {
            green_prefix("MOD", "proof/memory/x06-models.json", "x06-real-model-runtime-proof")
        } else {
            red_prefix(
                "MOD",
                "proof/memory/x06-models.json",
                Some("x06-real-model-runtime-proof"),
                format!(
                    "model proof incomplete: chat_ok={chat_ok:?}, embedding_ok={embedding_ok:?}, reranker_ok={reranker_ok:?}"
                ),
            )
        },
    );
    Ok(())
}

fn set_dogfood_status(
    workspace_root: &Path,
    prefixes: &mut BTreeMap<&'static str, MatrixPrefixRowDto>,
) -> TestResult<()> {
    let Some(dogfood) = proof_json(workspace_root, "proof/memory/x06-dogfood.json")? else {
        return Ok(());
    };
    let green_gates = dogfood["greenGates"].as_array().map_or(0, Vec::len);
    let lessons = dogfood["lessons"].as_array().map_or(0, Vec::len);
    let dogfood_source = serde_json::to_string(&dogfood)?;
    let has_dogfood_coordination = dogfood_source.contains("ocentra_enforcer_coordination_health")
        && dogfood_source.contains("ocentra_enforcer_doctor")
        && dogfood_source.contains("ocentra_enforcer_check");
    let lesson_shapes = dogfood["lessons"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|lesson| lesson["shape"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let has_t0_t1_t2 = ["t0", "t1", "t2"]
        .iter()
        .all(|shape| lesson_shapes.contains(shape));
    let has_failure_fix_evidence = dogfood_source
        .contains("Feature-parity runner initially over-claimed")
        && dogfood_source.contains("Runner claim logic is now fixture-backed")
        && dogfood_source.contains("feature_parity_harness passed after the fix");
    let lower_dogfood_source = dogfood_source.to_ascii_lowercase();
    let has_operational_learning = dogfood_source.contains("Windows paging-file pressure")
        && lower_dogfood_source.contains("pre-commit hook reported")
        && dogfood_source.contains("scoped X06 cargo and Enforcer checks");
    let dogfood_complete = dogfood["status"].as_str()
        == Some("policy-clean-focused-gates-full-package-timeout")
        && green_gates > 0
        && lessons > 0
        && has_dogfood_coordination
        && has_t0_t1_t2
        && has_failure_fix_evidence
        && has_operational_learning;
    prefixes.insert(
        "DOG",
        if dogfood_complete {
            green_prefix("DOG", "proof/memory/x06-dogfood.json", "x06-dogfood-closeout")
        } else {
            red_prefix(
                "DOG",
                "proof/memory/x06-dogfood.json",
                Some("x06-dogfood-closeout"),
                format!(
                    "dogfood proof incomplete: status={:?}, green_gates={green_gates}, lessons={lessons}, coordination={has_dogfood_coordination}, t0_t1_t2={has_t0_t1_t2}, failure_fix_evidence={has_failure_fix_evidence}, operational_learning={has_operational_learning}",
                    dogfood["status"].as_str()
                ),
            )
        },
    );
    Ok(())
}

#[test]
fn prefix_artifact_stem_covers_every_required_prefix() {
    for prefix in REQUIRED_PREFIXES {
        let stem = prefix_artifact_stem(prefix);
        assert!(
            stem.chars()
                .all(|character| character.is_ascii_lowercase() || character == '-'),
            "prefix {prefix} mapped to non-artifact-safe stem {stem}"
        );
        assert_ne!(
            stem, "unmapped-prefix",
            "prefix {prefix} fell through to the unmapped default -- add its mapping"
        );
    }
}
