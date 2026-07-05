use enforcer_events::bus::EventBus;
use enforcer_events::envelope::{EventEnvelope, StoredEventEnvelope};
use enforcer_events::error::EventingError;
use enforcer_events::ids::{EventId, EventType};
use enforcer_events::journal::policy::JournalPolicy;
use enforcer_events::journal::{EventJournal, JournalAppend};
use std::{
    future::Future,
    path::Path,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
};

use super::super::fixtures::{metadata, subscriber, TestEvent, TEST_SUBSCRIBER, TEST_TARGET};

#[derive(Clone, Debug)]
pub(super) struct TestText(pub(super) String);


#[derive(Clone, Debug)]
pub(super) struct JournalPath(pub(super) PathBuf);

#[derive(Clone, Debug, Default)]
pub(super) struct JournalLine(pub(super) String);

impl std::ops::Deref for JournalLine {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct JournalLines(Vec<JournalLine>);

impl std::ops::Deref for JournalLines {
    type Target = [JournalLine];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
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

impl AsRef<Path> for JournalPath {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl From<&JournalPath> for PathBuf {
    fn from(value: &JournalPath) -> Self {
        value.0.clone()
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
) -> Result<(), Box<dyn std::error::Error>> {
    bus.subscribe::<TestEvent, _, _>(
        subscriber(
            TestText(TEST_SUBSCRIBER.to_owned()),
            TestText(TEST_TARGET.to_owned()),
        )?,
        move |_| {
            let log = Arc::clone(&log);
            async move {
                let mut log = log
                    .lock()
                    .map_err(|e| EventingError::InvalidHandlerPolicy {
                        reason: e.to_string(),
                    })?;
                log.push(JournalLine(String::from("handler")));
                Ok(())
            }
        },
    )
    .await?;
    Ok(())
}

pub(super) fn stored_event(
    event: TestEvent,
) -> Result<StoredEventEnvelope, Box<dyn std::error::Error>> {
    Ok(EventEnvelope::from_event(event, metadata(TestText(TEST_TARGET.to_owned()))?)?.store()?)
}

pub(super) fn event_type(value: TestText) -> Result<EventType, Box<dyn std::error::Error>> {
    Ok(EventType::parse(value.0)?)
}

pub(super) fn shared_log() -> Arc<Mutex<JournalLines>> {
    Arc::new(Mutex::new(JournalLines::default()))
}

pub(super) fn snapshot(
    log: &Arc<Mutex<JournalLines>>,
) -> Result<JournalLines, Box<dyn std::error::Error>> {
    Ok(log.lock().map_err(|e| e.to_string())?.clone())
}

pub(super) fn journal_path(label: TestText) -> JournalPath {
    let label = label.0;
    JournalPath(std::env::temp_dir().join(format!(
        "ocentra-eventing-{label}-{}-{}.ndjson",
        std::process::id(),
        EventId::generated().as_str()
    )))
}

pub(super) async fn read_lines(
    path: JournalPath,
) -> Result<JournalLines, Box<dyn std::error::Error>> {
    let lines = tokio::fs::read_to_string(path)
        .await?
        .lines()
        .map(|line| JournalLine(String::from(line)))
        .collect::<Vec<_>>();
    Ok(JournalLines(lines))
}

pub(super) async fn write_lines(
    path: JournalPath,
    lines: &JournalLines,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut content = lines
        .iter()
        .map(|line| line.as_ref())
        .collect::<Vec<_>>()
        .join("\n");
    content.push('\n');
    tokio::fs::write(path, content).await?;
    Ok(())
}

pub(super) async fn tamper_first_journal_payload_label(
    path: JournalPath,
    label: TestText,
) -> Result<(), Box<dyn std::error::Error>> {
    let label = label.0;
    let mut lines = read_lines(path.clone()).await?;
    let mut entry: serde_json::Value = serde_json::from_str(&lines[0])?;
    entry["envelope"]["payload"]["label"] = serde_json::Value::String(label);
    lines[0] = JournalLine(serde_json::to_string(&entry)?);
    write_lines(path, &lines).await?;
    Ok(())
}

pub(super) async fn cleanup(path: JournalPath) {
    let _ = tokio::fs::remove_file(path).await;
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
                    reason: e.to_string(),
                })?;
            log.push(JournalLine(format!("journal:{}", envelope.contract.event_type.as_str())));
            Ok(JournalAppend {
                sequence: log.len() as u64,
                previous_hash: None,
                current_hash: None,
            })
        })
    }
}
