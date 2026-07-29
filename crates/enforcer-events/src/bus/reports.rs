use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::queue::policy::QueueReport;
use enforcer_domain::events_types::EventCount;

use super::DispatchMode;

pub mod dead_letter;
pub mod handler;

use self::dead_letter::DeadLetter;
use self::handler::HandlerReport;

/// Event-runtime data for event queue metrics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventQueueMetrics {
    pub queued_event_count: EventCount,
    pub queued_event_id_count: EventCount,
    pub queued_idempotency_key_count: EventCount,
    pub in_flight_event_id_count: EventCount,
    pub in_flight_idempotency_key_count: EventCount,
    pub completed_idempotency_key_count: EventCount,
    pub capacity: Option<EventCount>,
}

/// Event-runtime data for event request metrics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRequestMetrics {
    pub pending_request_count: EventCount,
    pub completed_request_count: EventCount,
    pub timed_out_request_count: EventCount,
}

/// Event-runtime data for event metrics snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventMetricsSnapshot {
    pub subscription_count: EventCount,
    pub stored_event_count: EventCount,
    pub dead_letter_count: EventCount,
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
    dead_letter_count: EventCount,
) -> handler::PublishReport {
    handler::PublishReport {
        // CLONE-JUSTIFICATION: the report owns event identity while the stored envelope remains available to the caller.
        event_id: stored.event_id.clone(),
        // CLONE-JUSTIFICATION: the report owns contract identity independently of the retained envelope.
        event_type: stored.contract.event_type.clone(),
        dispatch_mode,
        queue_report,
        subscriber_count: EventCount::default(),
        handled_count: EventCount::default(),
        dead_letter_count,
        handler_reports: Vec::new(),
    }
}
