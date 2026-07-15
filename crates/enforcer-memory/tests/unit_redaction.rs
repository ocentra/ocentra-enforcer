use enforcer_memory::boundary::record::MemoryRecordDto as MemoryRecord;
use enforcer_memory::record::{Evidence, Provenance, RecordDomain, RecordKind};
use enforcer_memory::redaction::{
    redact_identity, redact_path, redact_record, redact_secrets, redact_text, truncate_snippet,
    RedactionConfig,
};

#[test]
fn redacts_windows_and_posix_absolute_paths() {
    let text = r"see C:\Projects\enforcer\src\lib.rs and /home/alice/notes.txt";
    let out = redact_path(text, None);
    assert!(!out.contains("Projects"));
    assert!(!out.contains("alice"));
    assert!(out.contains("<repo-path>"));
}

#[test]
fn redacts_repo_root_relative_paths_by_stripping_the_root() {
    let text = r"crash in C:\Projects\enforcer\src\lib.rs line 4";
    let out = redact_path(text, Some(r"C:\Projects\enforcer"));
    assert_eq!(out, "crash in src/lib.rs line 4");
}

#[test]
fn redacts_emails_and_handles() {
    let text = "reported by alice@example.com, cc @bob-dev";
    let out = redact_identity(text, &[]);
    assert!(!out.contains("alice@example.com"));
    assert!(!out.contains("@bob-dev"));
    assert_eq!(out.matches("<redacted-identity>").count(), 2);
}

#[test]
fn redacts_explicit_identity_strings_even_without_pattern_match() {
    let text = "session owned by sujan.mishra on this box";
    let out = redact_identity(text, &["sujan.mishra"]);
    assert!(!out.contains("sujan.mishra"));
}

#[test]
fn redacts_secret_shaped_strings() {
    let cases = [
        "token: sk-abcdefghijklmnopqrstuvwx",
        "ghp_abcdefghijklmnopqrstuvwxyz012345",
        r#"api_key = "abcdef1234567890""#,
        "-----BEGIN RSA PRIVATE KEY-----",
    ];
    for case in cases {
        let out = redact_secrets(case);
        assert!(
            out.contains("<redacted-secret>"),
            "expected secret redaction for {case:?}, got {out:?}"
        );
    }
}

#[test]
fn leaves_ordinary_text_untouched() {
    let text = "this rule fires on missing error handling in async functions";
    assert_eq!(redact_secrets(text), text);
    assert_eq!(redact_path(text, None), text);
    assert_eq!(redact_identity(text, &[]), text);
}

#[test]
fn truncates_beyond_configured_length_with_marker() {
    let long = "x".repeat(1000);
    let out = truncate_snippet(&long, 100);
    assert!(out.len() < long.len());
    assert!(out.ends_with("\n... [truncated for community export]"));
    assert_eq!(&out[..100], &long[..100]);
}

#[test]
fn short_text_is_unchanged_by_truncation() {
    let short = "short snippet";
    assert_eq!(truncate_snippet(short, 400), short);
}

#[test]
fn redact_text_applies_the_full_pipeline_in_order() {
    let out = redact_text(
        "token: sk-abcdefghijklmnopqrstuvwx in C:\\Projects\\enforcer by alice@example.com",
        Some(r"C:\Projects\enforcer"),
        &["alice@example.com"],
        RedactionConfig {
            max_snippet_len: 400,
        },
    );
    assert!(out.contains("<redacted-secret>"));
    assert!(out.contains("<repo-path>"));
    assert!(out.contains("<redacted-identity>"));
}

#[test]
fn redact_record_clears_identity_fields_and_redacts_paths() {
    let record = MemoryRecord {
        schema_version: 1,
        id: "mem-primary-0001".to_string(),
        ts: "2026-07-05T00:00:00Z".to_string(),
        kind: RecordKind::Lesson,
        domain: RecordDomain::Harness,
        statement: r"fix landed in C:\Projects\enforcer\src\lib.rs, reported by alice@example.com"
            .to_string(),
        why: None,
        how_to_apply: None,
        applies_to: vec![],
        evidence: Some(Evidence {
            source: Some("gitHistory".to_string()),
            r#ref: Some(r"C:\Projects\enforcer\src\lib.rs".to_string()),
        }),
        routes: vec![],
        landed_at: vec![r"C:\Projects\enforcer\src\lib.rs".to_string()],
        supersedes: None,
        provenance: Provenance {
            writer: "arc-05".to_string(),
            session_id: Some("agent-abc123".to_string()),
            model: Some("claude-sonnet-5".to_string()),
            user: Some("sujan.mishra".to_string()),
        },
    };

    let record = enforcer_memory::record::MemoryRecord::from_dto(record);
    let redacted = redact_record(
        &record,
        Some(r"C:\Projects\enforcer"),
        RedactionConfig::default(),
    );
    assert!(redacted.provenance().user.is_none());
    assert!(redacted.provenance().session_id.is_none());
    assert!(redacted.provenance().model.is_none());
    assert_eq!(redacted.provenance().writer, "arc-05");
    assert!(!redacted.statement().contains("Projects"));
    assert!(!redacted.statement().contains("alice@example.com"));
    assert_eq!(redacted.landed_at()[0], "src/lib.rs");
    assert_eq!(
        redacted.evidence().and_then(|e| e.r#ref.as_deref()),
        Some("src/lib.rs")
    );
}

/// GOLDEN: fixture bundle in -> byte-exact expected redacted output.
#[test]
fn golden_community_export_redaction_is_byte_exact() -> Result<(), Box<dyn std::error::Error>> {
    let fixture_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/memory/redaction");
    let input = std::fs::read_to_string(fixture_dir.join("community-input.ndjson"))?;
    let expected = std::fs::read_to_string(fixture_dir.join("community-expected.ndjson"))?;

    let record: MemoryRecord = serde_json::from_str(input.trim_end())?;
    let record = enforcer_memory::record::MemoryRecord::from_dto(record);
    let redacted = redact_record(
        &record,
        Some(r"C:\Projects\enforcer"),
        RedactionConfig::default(),
    );
    let actual = serde_json::to_string(&redacted.to_dto())? + "\n";
    assert_eq!(
        actual, expected,
        "community redaction output must be byte-exact against the committed fixture"
    );
    Ok(())
}
