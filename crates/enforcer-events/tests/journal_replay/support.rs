use enforcer_domain::events_types::EventErrorReason;
use enforcer_domain::events_types::{EventId, EventType, JournalLine, JournalPath};
use enforcer_events::boundary::stored_event_persistence::StoredEventEnvelope;
use enforcer_events::bus::EventBus;
use enforcer_events::envelope::EventFrame;
use enforcer_events::error::EventingError;
use enforcer_events::journal::policy::JournalPolicy;
use enforcer_events::journal::{EventJournal, JournalAppend};
use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
};

use super::fixtures::{
    metadata, subscriber, TestEvent, TestText as FixtureText, TEST_SUBSCRIBER, TEST_TARGET,
};

#[derive(Clone, Debug)]
pub(super) struct TestText(pub(super) String);

#[derive(Clone, Debug, Default)]
pub(super) struct JournalLines(Vec<JournalLine>);

impl std::ops::Deref for JournalLines {
    type Target = [JournalLine];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl PartialEq<Vec<String>> for JournalLines {
    fn eq(&self, other: &Vec<String>) -> bool {
        self.0.len() == other.len()
            && self
                .0
                .iter()
                .zip(other.iter())
                .all(|(line, expected)| line.as_str() == expected)
    }
}

impl std::ops::DerefMut for JournalLines {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut_slice()
    }
}

impl JournalLines {
    fn push(&mut self, value: JournalLine) {
        self.0.push(value);
    }
}

pub(super) fn bus_with_recording_journal(
    policy: JournalPolicy,
    log: Arc<Mutex<JournalLines>>,
) -> EventBus {
    EventBus::with_journal(policy, Arc::new(RecordingJournal { log }))
}

pub(super) async fn subscribe_log_handler(
    bus: &EventBus,
    log: Arc<Mutex<JournalLines>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            FixtureText(TEST_SUBSCRIBER.to_owned()),
            FixtureText(TEST_TARGET.to_owned()),
        )?,
        move |_| {
            let log = Arc::clone(&log);
            async move {
                let mut log = log
                    .lock()
                    .map_err(|e| EventingError::InvalidHandlerPolicy {
                        reason: EventErrorReason::from_diagnostic(e.to_string()),
                    })?;
                log.push(JournalLine::from_diagnostic("handler"));
                Ok(())
            }
        },
    )
    .await?;
    Ok(())
}

pub(super) fn stored_event(
    event: TestEvent,
) -> Result<StoredEventEnvelope, Box<dyn std::error::Error + Send + Sync>> {
    Ok(EventFrame::from_event(event, metadata(FixtureText(TEST_TARGET.to_owned()))?)?.store()?)
}

pub(super) fn event_type(
    value: TestText,
) -> Result<EventType, Box<dyn std::error::Error + Send + Sync>> {
    Ok(EventType::parse(&{ value.0 })?)
}

pub(super) fn shared_log() -> Arc<Mutex<JournalLines>> {
    Arc::new(Mutex::new(JournalLines::default()))
}

pub(super) fn snapshot(
    log: &Arc<Mutex<JournalLines>>,
) -> Result<JournalLines, Box<dyn std::error::Error + Send + Sync>> {
    Ok(log.lock().map_err(|e| e.to_string())?.clone())
}

pub(super) fn journal_path(label: TestText) -> JournalPath {
    let label = label.0;
    JournalPath::from_diagnostic(
        std::env::temp_dir()
            .join(format!(
                "ocentra-eventing-{label}-{}-{}.ndjson",
                std::process::id(),
                EventId::generated().as_str()
            ))
            .display()
            .to_string(),
    )
}

pub(super) fn journal_file_path(path: &JournalPath) -> PathBuf {
    PathBuf::from(path.as_str())
}

pub(super) async fn read_lines(
    path: JournalPath,
) -> Result<JournalLines, Box<dyn std::error::Error + Send + Sync>> {
    let lines = tokio::fs::read_to_string(journal_file_path(&path))
        .await?
        .lines()
        .map(JournalLine::from_diagnostic)
        .collect::<Vec<_>>();
    Ok(JournalLines(lines))
}

pub(super) async fn write_lines(
    path: JournalPath,
    lines: &JournalLines,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut content = lines
        .iter()
        .map(JournalLine::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    content.push('\n');
    tokio::fs::write(journal_file_path(&path), content).await?;
    Ok(())
}

pub(super) async fn tamper_first_journal_payload_label(
    path: JournalPath,
    label: TestText,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let label = label.0;
    let mut lines = read_lines(path.clone()).await?;
    let mut entry: serde_json::Value = serde_json::from_str(lines[0].as_str())?;
    entry["envelope"]["payload"]["label"] = serde_json::Value::String(label);
    lines[0] = JournalLine::from_diagnostic(serde_json::to_string(&entry)?);
    write_lines(path, &lines).await?;
    Ok(())
}

pub(super) async fn cleanup(path: JournalPath) {
    let _ = tokio::fs::remove_file(journal_file_path(&path)).await;
}

struct RecordingJournal {
    log: Arc<Mutex<JournalLines>>,
}

impl EventJournal for RecordingJournal {
    fn append<'a>(
        &'a self,
        envelope: &'a StoredEventEnvelope,
    ) -> Pin<Box<dyn Future<Output = Result<JournalAppend, EventingError>> + Send + 'a>> {
        Box::pin(async move {
            let mut log = self
                .log
                .lock()
                .map_err(|e| EventingError::InvalidHandlerPolicy {
                    reason: EventErrorReason::from_diagnostic(e.to_string()),
                })?;
            log.push(JournalLine::from_diagnostic(format!(
                "journal:{}",
                envelope.contract.event_type.as_str()
            )));
            Ok(JournalAppend {
                sequence: enforcer_domain::events_types::JournalSequence::try_new(
                    std::num::NonZeroU64::new(log.len() as u64).ok_or_else(|| {
                        EventingError::InvalidHandlerPolicy {
                            reason: EventErrorReason::from_diagnostic(String::from(
                                "journal sequence must be positive",
                            )),
                        }
                    })?,
                ),
                previous_hash: None,
                current_hash: None,
            })
        })
    }
}
