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
    requirements: &[&str],
) -> TestResult {
    assert_eq!(proof["artifact"], artifact);
    assert_eq!(proof["status"], "green");
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
fn checked_in_diagnostics_proof_pins_safe_emission_and_redaction_requirements() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-diagnostics.json"))?;

    assert_requirements_covered(
        &proof,
        "x06-diagnostics",
        &[
            "ndjsonShape",
            "levelFiltering",
            "sourceTextRedaction",
            "controlCharacterDefense",
            "stderrOnlyForRealEmission",
        ],
    )?;
    assert_eq!(proof["namedTest"], "x06-diagnostics");

    let evidence = string_set(
        &proof["result"]["evidenceTests"],
        "diagnostics evidenceTests",
    )?;
    for test_name in [
        "unit_diagnostics::json_format_renders_file_skip_record_as_one_line_object_with_event_key",
        "unit_diagnostics::redaction_truncates_oversized_field_values_and_never_leaks_full_source_text",
        "unit_diagnostics::redaction_strips_control_characters_so_a_value_cannot_forge_extra_log_lines",
        "mcp_cli_live::diagnostics_never_leak_full_source_text_and_never_touch_stdout",
    ] {
        assert!(
            evidence.contains(test_name),
            "x06-diagnostics missing required evidence test {test_name}"
        );
    }
    Ok(())
}

#[test]
fn checked_in_summaries_proof_pins_stale_state_and_delete_unlink_requirements() -> TestResult {
    let proof: serde_json::Value =
        serde_json::from_str(include_str!("../../../proof/memory/x06-summaries.json"))?;

    assert_requirements_covered(
        &proof,
        "x06-summaries",
        &[
            "summaryInvalidation",
            "entityUnlinkOnDelete",
            "safeMissingSummaryState",
        ],
    )?;
    assert_eq!(proof["namedTest"], "x06-summaries");

    let evidence = string_set(&proof["result"]["evidenceTests"], "summaries evidenceTests")?;
    for test_name in [
        "unit_summaries::invalidate_marks_existing_summary_stale_without_deleting_it",
        "unit_summaries::missing_summary_is_considered_stale",
        "unit_summaries::remove_deletes_the_entry_entirely",
        "unit_summaries::deleting_a_file_unlinks_its_entities",
    ] {
        assert!(
            evidence.contains(test_name),
            "x06-summaries missing required evidence test {test_name}"
        );
    }
    Ok(())
}
