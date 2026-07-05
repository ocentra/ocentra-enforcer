use serde::{Deserialize, Serialize};

use crate::{
    AggregateKey, CorrelationId, DomainEvent, EventContract, EventId, EventType, EventingError,
    IdempotencyKey, QueueReport, SchemaVersion, StoredEventEnvelope, SubscriberId, TargetHandler,
};

use super::DispatchMode;

pub mod dead_letter;
pub mod handler;

use self::dead_letter::DeadLetter;
use self::handler::HandlerReport;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventQueueMetrics {
    pub queued_event_count: usize,
    pub queued_event_id_count: usize,
    pub queued_idempotency_key_count: usize,
    pub in_flight_event_id_count: usize,
    pub in_flight_idempotency_key_count: usize,
    pub completed_idempotency_key_count: usize,
    pub capacity: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventRequestMetrics {
    pub pending_request_count: usize,
    pub completed_request_count: usize,
    pub timed_out_request_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventMetricsSnapshot {
    pub subscription_count: usize,
    pub stored_event_count: usize,
    pub dead_letter_count: usize,
    pub queue: EventQueueMetrics,
    pub requests: EventRequestMetrics,
}

pub(super) fn dead_letters_for(
    stored: &StoredEventEnvelope,
    reports: &[HandlerReport],
) -> Vec<DeadLetter> {
    reports
        .iter()
        .filter_map(|report| DeadLetter::for_handler(stored, report))
        .collect()
}

pub(super) fn empty_publish_report(
    stored: &StoredEventEnvelope,
    dispatch_mode: DispatchMode,
    queue_report: QueueReport,
    dead_letter_count: usize,
) -> handler::PublishReport {
    handler::PublishReport {
        event_id: stored.event_id.clone(),
        event_type: stored.contract.event_type.clone(),
        dispatch_mode,
        queue_report,
        subscriber_count: 0,
        handled_count: 0,
        dead_letter_count,
        handler_reports: Vec::new(),
    }
}
