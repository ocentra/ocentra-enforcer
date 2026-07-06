//! X06.7 unit-shaped tests for [`enforcer_memory::diagnostics`], moved
//! out of `src/diagnostics.rs` per this crate's "no inline
//! `#[cfg(test)]` modules" style (workspace clippy denies
//! `unwrap`/`expect`/`panic` even in test code, so every assertion here
//! goes through `Result` + `?` rather than the original inline module's
//! `.unwrap()` calls).
//!
//! `MAX_FIELD_LEN` (200) / `MAX_FREE_TEXT_LEN` (40) are private module
//! constants in `src/diagnostics.rs` -- this file uses their literal
//! values directly (documented here, matching the source's own doc
//! comments) rather than widening the module's public surface just for
//! test access.

use enforcer_memory::diagnostics::{
    redact, redact_free_text, Diagnostics, FileSkipRecord, Format, Level, RequestRecord, SkipPhase,
};
use std::error::Error;
use std::time::Duration;

type TestResult = Result<(), Box<dyn Error>>;

/// Mirrors `src/diagnostics.rs`'s private `MAX_FIELD_LEN`.
const MAX_FIELD_LEN: usize = 200;
/// Mirrors `src/diagnostics.rs`'s private `MAX_FREE_TEXT_LEN`.
const MAX_FREE_TEXT_LEN: usize = 40;

#[test]
fn level_should_emit_is_at_or_above_configured_minimum() {
    let info_min = Level::Info;
    assert!(Level::Error.should_emit(info_min));
    assert!(Level::Warn.should_emit(info_min));
    assert!(Level::Info.should_emit(info_min));
    assert!(!Level::Debug.should_emit(info_min));
}

#[test]
fn level_none_as_configured_minimum_suppresses_every_record() {
    let none_min = Level::None;
    assert!(!Level::Error.should_emit(none_min));
    assert!(!Level::Warn.should_emit(none_min));
    assert!(!Level::Info.should_emit(none_min));
    assert!(!Level::Debug.should_emit(none_min));
}

#[test]
fn level_from_env_str_accepts_both_names_and_baseline_numeric_forms() {
    assert_eq!(Level::from_env_str("debug"), Some(Level::Debug));
    assert_eq!(Level::from_env_str("0"), Some(Level::Debug));
    assert_eq!(Level::from_env_str("INFO"), Some(Level::Info));
    assert_eq!(Level::from_env_str("1"), Some(Level::Info));
    assert_eq!(Level::from_env_str("warn"), Some(Level::Warn));
    assert_eq!(Level::from_env_str("2"), Some(Level::Warn));
    assert_eq!(Level::from_env_str("error"), Some(Level::Error));
    assert_eq!(Level::from_env_str("3"), Some(Level::Error));
    assert_eq!(Level::from_env_str("none"), Some(Level::None));
    assert_eq!(Level::from_env_str("4"), Some(Level::None));
    assert_eq!(Level::from_env_str("garbage"), None);
}

#[test]
fn text_format_renders_request_record_as_kv_line_with_msg_key() -> TestResult {
    let diagnostics = Diagnostics::new(Level::Debug, Format::Text);
    let record = RequestRecord {
        protocol: "mcp",
        method: "tools/call".to_owned(),
        tool: Some("index_repository".to_owned()),
        duration: Duration::from_millis(42),
        is_error: false,
    };
    let mut buf = Vec::new();
    diagnostics.emit(&mut buf, record.level(), &record)?;
    let line = String::from_utf8(buf)?;
    assert!(line.contains("msg=mcp.request"));
    assert!(line.contains("protocol=mcp"));
    assert!(line.contains("method=tools/call") || line.contains("method=\"tools/call\""));
    assert!(line.contains("tool=index_repository"));
    assert!(line.contains("durationMs=42"));
    assert!(line.contains("status=ok"));
    assert!(
        line.contains("level=info"),
        "success must log at info: {line}"
    );
    Ok(())
}

#[test]
fn request_record_level_is_warn_on_error_info_otherwise() {
    let ok_record = RequestRecord {
        protocol: "mcp",
        method: "tools/call".to_owned(),
        tool: Some("search_graph".to_owned()),
        duration: Duration::from_millis(1),
        is_error: false,
    };
    assert_eq!(ok_record.level(), Level::Info);

    let err_record = RequestRecord {
        is_error: true,
        ..ok_record
    };
    assert_eq!(err_record.level(), Level::Warn);
}

#[test]
fn json_format_renders_file_skip_record_as_one_line_object_with_event_key() -> TestResult {
    let diagnostics = Diagnostics::new(Level::Debug, Format::Json);
    let record = FileSkipRecord {
        path: "src/weird.xyz".to_owned(),
        reason: "no extractor for extension".to_owned(),
        phase: SkipPhase::Parse,
    };
    let mut buf = Vec::new();
    diagnostics.emit(&mut buf, Level::Warn, &record)?;
    let line = String::from_utf8(buf)?;
    let parsed: serde_json::Value = serde_json::from_str(line.trim())?;
    assert_eq!(parsed["event"], serde_json::json!("file_skip"));
    assert_eq!(parsed["path"], serde_json::json!("src/weird.xyz"));
    assert_eq!(parsed["phase"], serde_json::json!("parse"));
    assert_eq!(parsed["level"], serde_json::json!("warn"));
    Ok(())
}

#[test]
fn a_record_below_the_configured_level_is_not_emitted_at_all() -> TestResult {
    let diagnostics = Diagnostics::new(Level::Warn, Format::Text);
    let record = RequestRecord {
        protocol: "mcp",
        method: "tools/call".to_owned(),
        tool: Some("search_graph".to_owned()),
        duration: Duration::from_millis(1),
        is_error: false,
    };
    let mut buf = Vec::new();
    diagnostics.emit(&mut buf, Level::Debug, &record)?;
    assert!(
        buf.is_empty(),
        "a Debug record must not emit under a Warn minimum"
    );
    Ok(())
}

#[test]
fn from_env_defaults_when_vars_are_unset_or_garbage() {
    // Use a lock-free approach: set to a known-garbage value, verify
    // fallback, then restore. This crate's tests run single-threaded
    // enough for env mutation in a unit test to be acceptable here (no
    // other test in this file reads these two vars).
    std::env::set_var("ENFORCER_MEMORY_LOG_LEVEL", "not-a-real-level");
    std::env::set_var("ENFORCER_MEMORY_LOG_FORMAT", "not-a-real-format");
    let diagnostics = Diagnostics::from_env();
    assert_eq!(diagnostics.level, Level::Info);
    assert_eq!(diagnostics.format, Format::Text);
    std::env::remove_var("ENFORCER_MEMORY_LOG_LEVEL");
    std::env::remove_var("ENFORCER_MEMORY_LOG_FORMAT");
}

#[test]
fn redaction_truncates_oversized_field_values_and_never_leaks_full_source_text() -> TestResult {
    let huge_source_text = "fn very_long_function_body() {\n".repeat(50);
    assert!(huge_source_text.len() > MAX_FIELD_LEN);
    let redacted = redact(&huge_source_text);
    assert!(redacted.len() < huge_source_text.len());
    assert!(redacted.ends_with("bytes total]"));
    // The record types below are the ONLY things this module ever
    // formats into a line; proving their fields are always redacted is
    // the crate-wide "no raw prompt/private source text in diagnostics"
    // guarantee for every call site that uses them.
    let record = FileSkipRecord {
        path: "irrelevant.rs".to_owned(),
        reason: huge_source_text.clone(),
        phase: SkipPhase::Extract,
    };
    let diagnostics = Diagnostics::new(Level::Debug, Format::Json);
    let mut buf = Vec::new();
    diagnostics.emit(&mut buf, Level::Info, &record)?;
    let line = String::from_utf8(buf)?;
    assert!(
        !line.contains(&huge_source_text),
        "full oversized reason text must never appear verbatim in a diagnostic line"
    );
    Ok(())
}

#[test]
fn redact_free_text_truncates_a_short_but_adversarial_value_that_plain_redact_would_pass_through() {
    // A small source file's content is well under MAX_FIELD_LEN (200)
    // but still real source text -- redact_free_text's much tighter
    // MAX_FREE_TEXT_LEN (40) must catch it even though plain redact()
    // would let it through untouched.
    let small_source_text =
        "fn list_widgets() {\n    load_from_disk();\n}\n\nfn load_from_disk() {\n    validate_secret_marker();\n}\n";
    assert!(small_source_text.len() < MAX_FIELD_LEN);
    assert!(small_source_text.len() > MAX_FREE_TEXT_LEN);
    assert_eq!(
        redact(small_source_text),
        small_source_text.replace('\n', " "),
        "plain redact must NOT truncate a value this short (documents the gap redact_free_text closes)"
    );
    let redacted = redact_free_text(small_source_text);
    assert!(redacted.len() < small_source_text.len());
    assert!(redacted.ends_with("bytes total]"));
    assert!(
        !redacted.contains("validate_secret_marker"),
        "redact_free_text must not leak identifiers past its truncation point"
    );
}

#[test]
fn redaction_strips_control_characters_so_a_value_cannot_forge_extra_log_lines() {
    let malicious = "short\nlevel=error msg=tool_call tool=fabricated";
    let redacted = redact(malicious);
    assert!(!redacted.contains('\n'));
}
