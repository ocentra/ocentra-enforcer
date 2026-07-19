use enforcer_events::boundary::journal_persistence::{JournalAppendDto, NdjsonJournalEntryDto};
use enforcer_events::boundary::journal_phase_persistence::JournalDispatchPhaseDto;
use enforcer_events::boundary::stored_event_persistence::StoredEventEnvelopeDto;
use enforcer_events::error::EventingError;
use enforcer_events::journal::ndjson::{
    NdjsonEventJournal, NdjsonJournalEntry, NdjsonJournalOptions,
};
use enforcer_events::journal::EventJournal;

use super::fixtures::{test_event, test_event_for_type, OTHER_EVENT_TYPE, TEST_LABEL};
use super::support::{
    cleanup, journal_path, read_lines, stored_event, tamper_first_journal_payload_label, TestText,
};

fn assert_json_round_trip<T>(original: T) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let wire = serde_json::to_string(&original)?;
    let decoded: T = serde_json::from_str(&wire)?;
    assert_eq!(decoded, original);
    Ok(())
}

#[test]
fn journal_entry_dto_round_trip_preserves_append_and_envelope(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let stored = stored_event(test_event(super::fixtures::TestText(
        TEST_LABEL.to_owned(),
    ))?)?;
    let dto = NdjsonJournalEntryDto {
        append: JournalAppendDto {
            sequence: 1,
            previous_hash: None,
            current_hash: None,
        },
        phase: JournalDispatchPhaseDto::AfterDispatch,
        envelope: StoredEventEnvelopeDto::from(&stored),
    };
    assert_json_round_trip::<JournalAppendDto>(dto.append.clone())?;
    assert_json_round_trip::<NdjsonJournalEntryDto>(dto.clone())?;

    let wire = serde_json::to_string(&dto)?;
    let round_trip_entry: NdjsonJournalEntryDto = serde_json::from_str(&wire)?;
    let round_trip_append: &JournalAppendDto = &round_trip_entry.append;
    assert_eq!(round_trip_append.sequence, 1);
    let domain_entry: NdjsonJournalEntry = round_trip_entry.try_into()?;
    assert_eq!(domain_entry.envelope.event_id, stored.event_id);
    Ok(())
}

#[test]
fn journal_dto_conversions_reject_zero_sequence_numbers(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    assert!(
        enforcer_events::journal::JournalAppend::try_from(JournalAppendDto {
            sequence: 0,
            previous_hash: None,
            current_hash: None,
        })
        .is_err()
    );

    let stored = stored_event(test_event(super::fixtures::TestText(
        TEST_LABEL.to_owned(),
    ))?)?;
    assert!(NdjsonJournalEntry::try_from(NdjsonJournalEntryDto {
        append: JournalAppendDto {
            sequence: 0,
            previous_hash: None,
            current_hash: None,
        },
        phase: JournalDispatchPhaseDto::AfterDispatch,
        envelope: StoredEventEnvelopeDto::from(&stored),
    })
    .is_err());
    Ok(())
}

#[tokio::test]
async fn ndjson_journal_appends_one_object_per_line_with_hash_chain(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = journal_path(TestText("hash-chain".to_owned()));
    let journal =
        NdjsonEventJournal::with_options(path.clone(), NdjsonJournalOptions::hash_chain());
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
    let round_trip_first_dto: NdjsonJournalEntryDto = serde_json::from_str(lines[0].as_str())?;
    let round_trip_append: &JournalAppendDto = &round_trip_first_dto.append;
    assert_eq!(round_trip_append.sequence, 1);
    let first_entry: NdjsonJournalEntry = round_trip_first_dto.try_into()?;
    let round_trip_second_dto: NdjsonJournalEntryDto = serde_json::from_str(lines[1].as_str())?;
    let second_entry: NdjsonJournalEntry = round_trip_second_dto.try_into()?;

    assert_eq!(lines.len(), 2);
    assert_eq!(first_append.sequence.as_nonzero().get(), 1);
    assert_eq!(second_append.sequence.as_nonzero().get(), 2);
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
    let first_journal =
        NdjsonEventJournal::with_options(path.clone(), NdjsonJournalOptions::hash_chain());
    let first = stored_event(test_event(super::fixtures::TestText(
        TEST_LABEL.to_owned(),
    ))?)?;
    let first_append = first_journal.append(&first).await?;
    drop(first_journal);

    let reopened =
        NdjsonEventJournal::with_options(path.clone(), NdjsonJournalOptions::hash_chain());
    let second = stored_event(test_event_for_type(
        super::fixtures::TestText("second event".to_owned()),
        super::fixtures::TestText(OTHER_EVENT_TYPE.to_owned()),
    )?)?;
    let second_append = reopened.append(&second).await?;
    let lines = read_lines(path.clone()).await?;
    let second_entry: NdjsonJournalEntry =
        serde_json::from_str::<NdjsonJournalEntryDto>(lines[1].as_str())?.try_into()?;

    assert_eq!(lines.len(), 2);
    assert_eq!(first_append.sequence.as_nonzero().get(), 1);
    assert_eq!(second_append.sequence.as_nonzero().get(), 2);
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
    let first_journal =
        NdjsonEventJournal::with_options(path.clone(), NdjsonJournalOptions::hash_chain());
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
    let reopened =
        NdjsonEventJournal::with_options(path.clone(), NdjsonJournalOptions::hash_chain());

    let result = reopened
        .append(&stored_event(test_event_for_type(
            super::fixtures::TestText("third event".to_owned()),
            super::fixtures::TestText(OTHER_EVENT_TYPE.to_owned()),
        )?)?)
        .await;

    let Err(EventingError::JournalCorruptLine { line, reason }) = result else {
        return Err("expected a corrupt-line error at line 1 for the tampered payload".into());
    };
    assert_eq!(crate::event_count_value(line), 1);
    assert_eq!(
        reason.as_str().split(": expected ").next(),
        Some("journal hash-chain current hash mismatch at sequence 1")
    );
    cleanup(path).await;
    Ok(())
}

#[tokio::test]
async fn concurrent_ndjson_appends_do_not_hold_state_lock_across_file_write(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = journal_path(TestText("concurrent-hash-chain".to_owned()));
    let journal =
        NdjsonEventJournal::with_options(path.clone(), NdjsonJournalOptions::hash_chain());
    let handles = (0..4)
        .map(|index| {
            let journal = journal.clone();
            // CANCELLATION-TEST: every finite append task is retained below and joined before assertions.
            let handle = tokio::spawn(async move {
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
            });
            handle
        })
        .collect::<Vec<_>>();

    for handle in handles {
        let joined: Result<_, String> = handle.await.map_err(|error| error.to_string())?;
        joined.map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { error.into() })?;
    }

    let lines = read_lines(path.clone()).await?;
    let mut entries: Vec<NdjsonJournalEntry> = Vec::with_capacity(lines.len());
    for line in lines.iter() {
        entries.push(serde_json::from_str::<NdjsonJournalEntryDto>(line.as_str())?.try_into()?);
    }

    assert_eq!(entries.len(), 4);
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(entry.append.sequence.as_nonzero().get(), index as u64 + 1);
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
// CANCELLATION-TEST: concurrent append tasks are finite, retained in the handle set, and joined before journal assertions.
