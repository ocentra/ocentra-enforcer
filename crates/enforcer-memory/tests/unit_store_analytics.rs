use enforcer_memory::boundary::log_schema::{ObservationLogEntryDto, SCHEMA_VERSION};
use enforcer_memory::error::Result;
use enforcer_memory::store::analytics::{
    AnalyticsReadModel, InProcessAnalytics, RepoContextCounts,
};

fn entry(seq: u64, repo_context: &str, clean: bool) -> ObservationLogEntryDto {
    ObservationLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: seq.into(),
        id: format!("obs-{seq:04}").into(),
        lesson_id: "L1".into(),
        rule_id: None,
        fault_class: None,
        repo_context: repo_context.into(),
        clean: clean.into(),
        source_surface: "scan".into(),
        ts: "2026-07-04T00:00:00Z".into(),
        supersedes_seq: None,
        payload_kind: None,
        payload: None,
    }
}

#[test]
fn aggregates_deterministically_grouped_and_sorted() -> Result<()> {
    let entries = vec![
        entry(0, "crates/b", true),
        entry(1, "crates/a", false),
        entry(2, "crates/a", true),
        entry(3, "crates/a", true),
    ];
    let mut backend = InProcessAnalytics::default();
    backend.load(&entries)?;
    let counts = backend.counts_by_repo_context()?;
    assert_eq!(
        counts,
        vec![
            RepoContextCounts {
                repo_context: "crates/a".to_owned().into(),
                clean: 2.into(),
                findings: 1.into(),
            },
            RepoContextCounts {
                repo_context: "crates/b".to_owned().into(),
                clean: 1.into(),
                findings: 0.into(),
            },
        ]
    );
    Ok(())
}

#[test]
fn reload_replaces_rather_than_accumulates() -> Result<()> {
    let mut backend = InProcessAnalytics::default();
    backend.load(&[entry(0, "crates/a", true)])?;
    backend.load(&[entry(0, "crates/a", false)])?;
    let counts = backend.counts_by_repo_context()?;
    assert_eq!(counts.len(), 1);
    assert_eq!(counts[0].clean, 0);
    assert_eq!(counts[0].findings, 1);
    Ok(())
}
