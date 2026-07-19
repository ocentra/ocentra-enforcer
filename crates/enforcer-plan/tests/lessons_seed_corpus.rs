use std::collections::HashMap;

use enforcer_domain::ids::RuleId;
use enforcer_domain::plan_types::{PlanArtifactPath, PlanFileContent};
use enforcer_domain::severity::Severity;
use enforcer_plan::lessons::{import_seed_corpus, run_doctor, LessonLedger};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn duplicate_displayed_seed_labels_import_as_distinct_stable_ledger_ids() -> TestResult {
    let corpus_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md");
    let corpus = PlanFileContent::try_new(std::fs::read_to_string(corpus_path)?)?;
    let displayed_labels: Vec<String> = corpus
        .as_str()
        .lines()
        .filter_map(|line| line.strip_prefix("| L"))
        .filter_map(|line| line.split(" | ").next())
        .map(|suffix| format!("L{suffix}"))
        .collect();

    let duplicate_counts: HashMap<String, usize> =
        displayed_labels
            .iter()
            .fold(HashMap::new(), |mut counts, label| {
                *counts.entry(label.clone()).or_default() += 1;
                counts
            });
    let duplicates: Vec<_> = duplicate_counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .collect();
    assert!(
        !duplicates.is_empty(),
        "the regression corpus must retain repeated displayed labels"
    );

    let temp = tempfile::tempdir()?;
    let ledger_path = temp.path().join("lessons.ndjson");
    let mut ledger = LessonLedger::open(PlanArtifactPath::try_new(ledger_path)?)?;
    let first = import_seed_corpus(&mut ledger, std::slice::from_ref(&corpus), &[])?;
    assert_eq!(
        first.discovered, first.newly_appended,
        "a fresh import must append every historical row, including repeated labels"
    );

    let persisted = ledger.list()?;
    assert_eq!(persisted.len(), displayed_labels.len());
    for (label, count) in duplicates {
        let prefix = format!("{label}-SRC-");
        let matching: Vec<_> = persisted
            .iter()
            .filter(|record| record.id.as_str().starts_with(&prefix))
            .collect();
        assert_eq!(matching.len(), *count, "{label} rows must all persist");
        assert!(
            matching.iter().all(|record| {
                record
                    .id
                    .as_str()
                    .strip_prefix(&prefix)
                    .is_some_and(|suffix| !suffix.is_empty())
            }),
            "{label} rows must retain their displayed label with a deterministic digest suffix"
        );
    }

    let second = import_seed_corpus(&mut ledger, std::slice::from_ref(&corpus), &[])?;
    assert_eq!(
        usize::from(second.newly_appended),
        0,
        "unchanged re-import must be idempotent"
    );
    assert_eq!(ledger.verify_on_replay()?, first.discovered);
    Ok(())
}

#[test]
fn duplicate_identity_ignores_unrelated_source_row_order() -> TestResult {
    let header =
        "| id | date | observed | lesson | landed-at | ships-via |\n|---|---|---|---|---|---|\n";
    let first = PlanFileContent::try_new(format!(
        "{header}| L7 | 2026-07-04 | first observation | first lesson | this row | plan doc |\n\
         | L7 | 2026-07-04 | second observation | second lesson | this row | plan doc |\n\
         | L8 | 2026-07-04 | unrelated observation | unrelated lesson | this row | plan doc |\n"
    ))?;
    let reordered = PlanFileContent::try_new(format!(
        "{header}| L8 | 2026-07-04 | unrelated observation | unrelated lesson | this row | plan doc |\n\
         | L7 | 2026-07-04 | first observation | first lesson | this row | plan doc |\n\
         | L7 | 2026-07-04 | second observation | second lesson | this row | plan doc |\n"
    ))?;

    let first_temp = tempfile::tempdir()?;
    let first_path = first_temp.path().join("first.ndjson");
    let mut first_ledger = LessonLedger::open(PlanArtifactPath::try_new(first_path)?)?;
    import_seed_corpus(&mut first_ledger, std::slice::from_ref(&first), &[])?;
    let mut first_ids: Vec<_> = first_ledger
        .list()?
        .into_iter()
        .filter(|record| record.id.as_str().starts_with("L7-SRC-"))
        .map(|record| record.id.to_string())
        .collect();
    first_ids.sort();

    let reordered_temp = tempfile::tempdir()?;
    let reordered_path = reordered_temp.path().join("reordered.ndjson");
    let mut reordered_ledger = LessonLedger::open(PlanArtifactPath::try_new(reordered_path)?)?;
    import_seed_corpus(&mut reordered_ledger, std::slice::from_ref(&reordered), &[])?;
    let mut reordered_ids: Vec<_> = reordered_ledger
        .list()?
        .into_iter()
        .filter(|record| record.id.as_str().starts_with("L7-SRC-"))
        .map(|record| record.id.to_string())
        .collect();
    reordered_ids.sort();

    assert_eq!(first_ids.len(), 2);
    assert_eq!(first_ids, reordered_ids);
    Ok(())
}

#[test]
fn real_seed_corpus_preserves_the_doctors_honest_pending_verdict() -> TestResult {
    let corpus_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md");
    let corpus = PlanFileContent::try_new(std::fs::read_to_string(corpus_path)?)?;
    let temporary = tempfile::tempdir()?;
    let ledger_path = temporary.path().join("lessons.ndjson");
    let mut ledger = LessonLedger::open(PlanArtifactPath::try_new(ledger_path)?)?;
    let first = import_seed_corpus(&mut ledger, std::slice::from_ref(&corpus), &[])?;
    assert!(
        usize::from(first.discovered) >= 26,
        "real corpus lost historical lessons"
    );
    assert_eq!(first.discovered, first.newly_appended);
    assert_eq!(
        usize::from(
            import_seed_corpus(&mut ledger, std::slice::from_ref(&corpus), &[])?.newly_appended,
        ),
        0,
        "real corpus re-import must remain idempotent"
    );

    let records = ledger.latest()?;
    let contents = records
        .iter()
        .flat_map(|record| record.landed_at.iter().cloned())
        .map(|artifact| (artifact, corpus.clone()))
        .collect();
    let rule_id: RuleId = "LESSON-DOCTOR.1".parse()?;
    let findings = run_doctor(&rule_id, &records, &contents, &HashMap::new())?;
    assert!(
        findings
            .iter()
            .any(|finding| finding.severity == Severity::Error),
        "unregistered real RuleCandidate fixture parity must fail closed"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.severity == Severity::Warning),
        "real PlanDoc-only captures must remain visible as warnings"
    );
    Ok(())
}
