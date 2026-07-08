use enforcer_events::error::EventingError;
use enforcer_events::journal::ndjson::{
    NdjsonEventJournal, NdjsonJournalEntry, NdjsonJournalOptions,
};
use enforcer_events::journal::EventJournal;

use super::fixtures::{test_event, test_event_for_type, OTHER_EVENT_TYPE, TEST_LABEL};
use super::support::{
    cleanup, journal_path, read_lines, stored_event, tamper_first_journal_payload_label, TestText,
};

#[tokio::test]
async fn ndjson_journal_appends_one_object_per_line_with_hash_chain(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = journal_path(TestText("hash-chain".to_owned()));
    let journal = NdjsonEventJournal::with_options(&path, NdjsonJournalOptions::hash_chain());
    let first = stored_event(test_event(super::fixtures::TestText(
        TEST_LABEL.to_owned(),
    ))?)?;
    let second = stored_event(test_event_for_type(
        super::fixtures::TestText("second event".to_owned()),
        super::fixtures::TestText(OTHER_EVENT_TYPE.to_owned()),
    )?)?;

    let first_append = journal.append(&first).await?;
    let second_append = journal.append(&second).await?;

    let lines = read_lines(path.clone()).await?;
    let first_entry: NdjsonJournalEntry = serde_json::from_str(&lines[0])?;
    let second_entry: NdjsonJournalEntry = serde_json::from_str(&lines[1])?;

    assert_eq!(lines.len(), 2);
    assert_eq!(first_append.sequence, 1);
    assert_eq!(second_append.sequence, 2);
    assert!(first_append.previous_hash.is_none());
    assert_eq!(first_entry.append.current_hash, first_append.current_hash);
    assert_eq!(second_append.previous_hash, first_append.current_hash);
    assert_eq!(second_entry.append.current_hash, second_append.current_hash);
    assert_eq!(first_entry.envelope.event_id, first.event_id);
    assert_eq!(
        first_entry.envelope.contract.schema_version,
        first.contract.schema_version
    );
    cleanup(path).await;
    Ok(())
}

#[tokio::test]
async fn ndjson_journal_reopen_continues_sequence_and_hash_chain(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = journal_path(TestText("reopen-hash-chain".to_owned()));
    let first_journal = NdjsonEventJournal::with_options(&path, NdjsonJournalOptions::hash_chain());
    let first = stored_event(test_event(super::fixtures::TestText(
        TEST_LABEL.to_owned(),
    ))?)?;
    let first_append = first_journal.append(&first).await?;
    drop(first_journal);

    let reopened = NdjsonEventJournal::with_options(&path, NdjsonJournalOptions::hash_chain());
    let second = stored_event(test_event_for_type(
        super::fixtures::TestText("second event".to_owned()),
        super::fixtures::TestText(OTHER_EVENT_TYPE.to_owned()),
    )?)?;
    let second_append = reopened.append(&second).await?;
    let lines = read_lines(path.clone()).await?;
    let second_entry: NdjsonJournalEntry = serde_json::from_str(&lines[1])?;

    assert_eq!(lines.len(), 2);
    assert_eq!(first_append.sequence, 1);
    assert_eq!(second_append.sequence, 2);
    assert_eq!(second_append.previous_hash, first_append.current_hash);
    assert_eq!(second_entry.append.previous_hash, first_append.current_hash);
    assert_eq!(second_entry.append.current_hash, second_append.current_hash);
    cleanup(path).await;
    Ok(())
}

#[tokio::test]
async fn ndjson_journal_reopen_rejects_tampered_hash_chain_payload(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = journal_path(TestText("reopen-tampered-hash-chain".to_owned()));
    let first_journal = NdjsonEventJournal::with_options(&path, NdjsonJournalOptions::hash_chain());
    first_journal
        .append(&stored_event(test_event(super::fixtures::TestText(
            TEST_LABEL.to_owned(),
        ))?)?)
        .await?;
    first_journal
        .append(&stored_event(test_event_for_type(
            super::fixtures::TestText("second event".to_owned()),
            super::fixtures::TestText(OTHER_EVENT_TYPE.to_owned()),
        )?)?)
        .await?;
    drop(first_journal);
    tamper_first_journal_payload_label(path.clone(), TestText("tampered event".to_owned())).await?;
    let reopened = NdjsonEventJournal::with_options(&path, NdjsonJournalOptions::hash_chain());

    let result = reopened
        .append(&stored_event(test_event_for_type(
            super::fixtures::TestText("third event".to_owned()),
            super::fixtures::TestText(OTHER_EVENT_TYPE.to_owned()),
        )?)?)
        .await;

    let Err(EventingError::JournalCorruptLine { line: 1, reason }) = result else {
        return Err("expected a corrupt-line error at line 1 for the tampered payload".into());
    };
    assert_eq!(
        reason.split(": expected ").next(),
        Some("journal hash-chain current hash mismatch at sequence 1")
    );
    cleanup(path).await;
    Ok(())
}

#[tokio::test]
async fn concurrent_ndjson_appends_do_not_hold_state_lock_across_file_write(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = journal_path(TestText("concurrent-hash-chain".to_owned()));
    let journal = NdjsonEventJournal::with_options(&path, NdjsonJournalOptions::hash_chain());
    let handles = (0..4)
        .map(|index| {
            let journal = journal.clone();
            tokio::spawn(async move {
                let event = test_event_for_type(
                    super::fixtures::TestText(format!("parallel event {index}")),
                    super::fixtures::TestText(OTHER_EVENT_TYPE.to_owned()),
                )
                .map_err(|error| error.to_string())
                .and_then(|event| stored_event(event).map_err(|error| error.to_string()))?;
                journal
                    .append(&event)
                    .await
                    .map_err(|error| error.to_string())
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        let joined: Result<_, String> = handle.await.map_err(|error| error.to_string())?;
        joined.map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { error.into() })?;
    }

    let lines = read_lines(path.clone()).await?;
    let mut entries = Vec::with_capacity(lines.len());
    for line in lines.iter() {
        entries.push(serde_json::from_str::<NdjsonJournalEntry>(line)?);
    }

    assert_eq!(entries.len(), 4);
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(entry.append.sequence, index as u64 + 1);
        if index == 0 {
            assert!(entry.append.previous_hash.is_none());
        } else {
            assert_eq!(
                entry.append.previous_hash,
                entries[index - 1].append.current_hash
            );
        }
    }
    cleanup(path).await;
    Ok(())
}
