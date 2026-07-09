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

fn assert_requirements_covered(
    proof: &serde_json::Value,
    artifact: &str,
    named_test: &str,
    requirements: &[&str],
) -> TestResult {
    assert_eq!(proof["artifact"], artifact);
    assert_eq!(proof["status"], "green");
    assert_eq!(proof["namedTest"], named_test);
    assert_eq!(proof["result"]["testsFailed"], 0);

    let hard_requirements = proof["hardRequirements"]
        .as_object()
        .ok_or_else(|| format!("{artifact} hardRequirements must be an object"))?;
    for requirement in requirements {
        assert_eq!(
            hard_requirements
                .get(*requirement)
                .and_then(serde_json::Value::as_str),
            Some("covered"),
            "{artifact} requirement {requirement} must remain covered"
        );
    }
    assert_eq!(
        hard_requirements.len(),
        requirements.len(),
        "{artifact} should not gain unreviewed hard requirements without this proof test changing"
    );
    Ok(())
}

#[test]
fn checked_in_policy_proof_pins_runtime_cache_and_import_policy_requirements() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-policy.json"))?;

    assert_requirements_covered(
        &proof,
        "x06-policy",
        "x06-policy-filters",
        &[
            "retrievalPolicyFilters",
            "explicitExportConsent",
            "communityCreatorRedaction",
            "cacheSourcePolicyAndIntegrity",
            "noAbsoluteModelArtifactPaths",
            "zeroTrustImportPolicy",
            "diagnosticRedactionPolicy",
        ],
    )?;

    let evidence = string_set(&proof["result"]["evidenceTests"], "policy evidenceTests")?;
    for test_name in [
        "retrieval_stack::hard_filters_exclude_a_document_from_the_full_pipeline_result",
        "share::tests::personal_export_still_requires_consent",
        "share::tests::team_export_without_consent_is_rejected",
        "share::tests::community_export_drops_creator_even_if_supplied",
        "model_cache::absolute_artifact_paths_are_rejected",
        "model_runtime::parent_policy_cache_and_integrity_states_are_represented",
        "federation_roundtrip::tampering_the_signature_bytes_is_rejected_with_a_recorded_reason",
        "unit_diagnostics::redaction_truncates_oversized_field_values_and_never_leaks_full_source_text",
    ] {
        assert!(
            evidence.contains(test_name),
            "x06-policy missing required evidence test {test_name}"
        );
    }
    Ok(())
}

#[test]
fn checked_in_federation_proof_pins_zero_trust_artifact_exchange_requirements() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-federation.json"))?;

    assert_requirements_covered(
        &proof,
        "x06-federation",
        "x06-federation",
        &[
            "exactArtifactRetrieval",
            "zeroTrustImport",
            "signatureAndChecksumRejection",
            "inactiveImportUntilLocalLanding",
            "communityRedactionGolden",
            "graphArtifactBootstrap",
        ],
    )?;

    let evidence = string_set(
        &proof["result"]["evidenceTests"],
        "federation evidenceTests",
    )?;
    for test_name in [
        "federation_roundtrip::exact_artifact_retrieval_wrong_id_and_traversal_are_all_fail_closed",
        "federation_roundtrip::personal_bundle_export_import_roundtrips_exactly",
        "federation_roundtrip::tampering_the_signature_bytes_is_rejected_with_a_recorded_reason",
        "federation_roundtrip::tampering_with_the_manifests_content_hash_is_rejected_as_a_checksum_failure",
        "federation_roundtrip::imported_content_stays_inactive_until_a_local_landing_activates_it",
        "federation_roundtrip::community_redaction_matches_the_committed_golden_fixture_byte_exact",
        "federation_roundtrip::code_graph_artifact_export_then_bootstrap_import_reconstructs_identical_counts",
    ] {
        assert!(
            evidence.contains(test_name),
            "x06-federation missing required evidence test {test_name}"
        );
    }
    Ok(())
}
