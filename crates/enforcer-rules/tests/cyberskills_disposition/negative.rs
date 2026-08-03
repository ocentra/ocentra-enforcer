//! CP00 negative fixture integration tests.
//!
//! BOUNDARY-INVARIANT: malformed JSON is tested at the boundary and never
//! changes accepted evidence.
//! NEGATIVE-TEST: every named fixture must be rejected by parsing or validation.

use proptest::prelude::any;
use proptest::proptest;
use serde_json::Value;

use super::cp08_validation::validate_cp08_artifacts;
use super::negative_mutations::mutate;
use super::support::repo_root;
use super::{parse_manifest, validate_manifest, DISPOSITION_JSON, NEGATIVE_FIXTURES};

#[test]
fn negative_fixture_matrix_rejects_contract_drift() -> Result<(), Box<dyn std::error::Error>> {
    let cases: Vec<Value> = serde_json::from_str(NEGATIVE_FIXTURES)?;
    let baseline: Value = serde_json::from_str(DISPOSITION_JSON)?;
    let artifact_root = repo_root()?;
    for case in cases {
        let name = case["name"].as_str().ok_or("fixture name missing")?;
        let mutated = serde_json::to_string(&mutate(baseline.clone(), name)?)?;
        let parsed = parse_manifest(&mutated);
        let semantic_rejected = parsed
            .as_ref()
            .map(|manifest| validate_manifest(manifest).is_err())
            .unwrap_or(true);
        let artifact_rejected = parsed
            .as_ref()
            .map(|manifest| {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    validate_cp08_artifacts(&artifact_root, manifest)
                }))
                .map(|result| result.is_err())
                .unwrap_or(true)
            })
            .unwrap_or(true);
        assert!(
            semantic_rejected || artifact_rejected,
            "negative case unexpectedly accepted: {name}"
        );
    }
    Ok(())
}

proptest! {
    #[test]
    fn parser_rejects_or_accepts_arbitrary_utf8_without_panicking(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        let raw = String::from_utf8_lossy(&bytes);
        let _ = parse_manifest(&raw);
    }
}
