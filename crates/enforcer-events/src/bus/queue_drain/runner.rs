use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::{clock::EventClockInstant, error::EventingError};
use enforcer_domain::events_types::EventType;
use enforcer_domain::events_types::{
    DeadLetterReason, JournalAppendDecision, QueueDisposition, QueueExpirationState,
};

use crate::bus::{
    publish::flow::DispatchRequest,
    publish::DispatchStoredError,
    reports::{dead_letter::DeadLetter, handler::QueueDrainReport},
    DispatchMode, EventBus,
};

pub(super) async fn drain_queued_matching_unchecked(
    bus: &EventBus,
    dispatch_mode: DispatchMode,
    event_type: Option<&EventType>,
) -> Result<QueueDrainReport, EventingError> {
    let queued_before = bus.queue.queued_count(event_type);
    let mut expired_count = 0_usize;
    let mut dispatch_reports = Vec::new();
    let mut attempted_count = 0_usize;

    while attempted_count < crate::boundary::event_values::event_count_value(queued_before) {
        let Some(queued_envelope) = bus.queue.take_next_queued(event_type) else {
            break;
        };
        attempted_count += 1;
        let now = bus.clock.now();
        if let Some((reason, error)) = queued_expiration(
            &queued_envelope.stored,
            queued_envelope.expiration(now, bus.queue.policy().ttl()),
            now,
        ) {
            expired_count += 1;
            let dead_letter = DeadLetter::for_queue(&queued_envelope.stored, reason, error);
            // CLONE-JUSTIFICATION: the completion registry owns the idempotency key after the queued envelope is consumed.
            bus.queue.mark_completed(
                &queued_envelope.stored.event_id,
                queued_envelope.stored.idempotency_key.clone(),
            );
            bus.record_dead_letter(dead_letter).await;
            continue;
        }

        let subscribers = bus.subscribers_for(&queued_envelope.stored);
        if subscribers.is_empty() {
            bus.queue.requeue(queued_envelope);
            continue;
        }

        // CLONE-JUSTIFICATION: dispatch consumes an owned snapshot while queue recovery retains the original envelope.
        let report = match bus
            .dispatch_stored_checked(
                DispatchRequest {
                    stored: queued_envelope.stored.clone(),
                    subscribers,
                    dispatch_mode,
                },
                bus.queue.report(QueueDisposition::Dispatched),
                JournalAppendDecision::Skip,
            )
            .await
        {
            Ok(report) => report,
            Err(DispatchStoredError::BeforeDispatch(error)) => {
                bus.queue.requeue(queued_envelope);
                return Err(error);
            }
            Err(DispatchStoredError::AfterDispatch(error)) => return Err(error),
        };
        dispatch_reports.push(report);
    }

    Ok(QueueDrainReport {
        queued_before,
        dispatched_count: crate::boundary::event_values::event_count(dispatch_reports.len()),
        expired_count: crate::boundary::event_values::event_count(expired_count),
        remaining_count: bus.queue.queued_count(event_type),
        dispatch_reports,
    })
}

fn queued_expiration(
    stored: &StoredEventEnvelope,
    expiration: QueueExpirationState,
    now: EventClockInstant,
) -> Option<(DeadLetterReason, EventingError)> {
    if stored.is_deadline_expired(now) {
        // CLONE-JUSTIFICATION: the deadline error owns the event type while the stored envelope remains available for dead-letter construction.
        return Some((
            DeadLetterReason::DeadlineExpired,
            EventingError::EventDeadlineExpired {
                event_type: stored.contract.event_type.clone(),
            },
        ));
    }
    if expiration == QueueExpirationState::Expired {
        // CLONE-JUSTIFICATION: the expiry error owns the event type while the stored envelope remains available for dead-letter construction.
        return Some((
            DeadLetterReason::QueueExpired,
            EventingError::NoSubscriber {
                event_type: stored.contract.event_type.clone(),
            },
        ));
    }
    None
}
