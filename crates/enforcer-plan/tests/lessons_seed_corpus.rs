use std::collections::HashMap;

use enforcer_plan::lessons::{import_seed_corpus, LessonLedger};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn duplicate_displayed_seed_labels_import_as_distinct_stable_ledger_ids() -> TestResult {
    let corpus_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md");
    let corpus = std::fs::read_to_string(corpus_path)?;
    let displayed_labels: Vec<String> = corpus
        .lines()
        .filter_map(|line| line.strip_prefix("| L"))
        .filter_map(|line| line.split(" | ").next())
        .map(|suffix| format!("L{suffix}"))
        .collect();

    let duplicate_counts: HashMap<String, usize> = displayed_labels
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
    let mut ledger = LessonLedger::open(&ledger_path)?;
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
    assert_eq!(second.newly_appended, 0, "unchanged re-import must be idempotent");
    assert_eq!(ledger.verify_on_replay()?, first.discovered);
    Ok(())
}

#[test]
fn duplicate_identity_ignores_unrelated_source_row_order() -> TestResult {
    let header = "| id | date | observed | lesson | landed-at | ships-via |\n|---|---|---|---|---|---|\n";
    let first = format!(
        "{header}| L7 | 2026-07-04 | first observation | first lesson | this row | plan doc |\n\
         | L7 | 2026-07-04 | second observation | second lesson | this row | plan doc |\n\
         | L8 | 2026-07-04 | unrelated observation | unrelated lesson | this row | plan doc |\n"
    );
    let reordered = format!(
        "{header}| L8 | 2026-07-04 | unrelated observation | unrelated lesson | this row | plan doc |\n\
         | L7 | 2026-07-04 | first observation | first lesson | this row | plan doc |\n\
         | L7 | 2026-07-04 | second observation | second lesson | this row | plan doc |\n"
    );

    let first_temp = tempfile::tempdir()?;
    let first_path = first_temp.path().join("first.ndjson");
    let mut first_ledger = LessonLedger::open(&first_path)?;
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
    let mut reordered_ledger = LessonLedger::open(&reordered_path)?;
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
