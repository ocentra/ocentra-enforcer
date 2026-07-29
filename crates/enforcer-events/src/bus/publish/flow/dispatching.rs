use std::sync::Arc;

use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use crate::bus::dispatch::{dispatch_concurrent, dispatch_sequential};
use crate::bus::publisher::EventPublisher;
use crate::bus::reports::dead_letter::DeadLetter;
use crate::bus::reports::empty_publish_report;
use crate::bus::{DispatchMode, EventBus, SubscriberRecord};
use crate::queue::state::NoSubscriberQueueDecision;
use crate::{bus::reports::handler::PublishReport, error::EventingError};
use enforcer_domain::events_types::{DeadLetterReason, EventCount, QueueDisposition};

pub(super) async fn publish_without_subscribers(
    bus: &EventBus,
    stored: StoredEventEnvelope,
    dispatch_mode: DispatchMode,
) -> Result<PublishReport, EventingError> {
    match bus
        .queue
        // CLONE-JUSTIFICATION: queue owns the event while this flow retains it for reporting.
        .enqueue_no_subscriber(stored.clone(), bus.clock.now())?
    {
        NoSubscriberQueueDecision::Dispatch(queue_report)
        | NoSubscriberQueueDecision::Queued(queue_report) => {
            bus.record_stored_snapshot(&stored).await;
            Ok(empty_publish_report(
                &stored,
                dispatch_mode,
                queue_report,
                EventCount::ZERO,
            ))
        }
        NoSubscriberQueueDecision::QueuedWithDeadLetter(queue_report, dropped, reason, error) => {
            let dropped = *dropped;
            bus.record_stored_snapshot(&stored).await;
            let dead_letter = DeadLetter::for_queue(&dropped, reason, error);
            bus.queue
                // CLONE-JUSTIFICATION: completion bookkeeping owns the key beyond this borrowed dropped event.
                .mark_completed(&dropped.event_id, dropped.idempotency_key.clone());
            bus.record_dead_letter(dead_letter).await;
            Ok(empty_publish_report(
                &stored,
                dispatch_mode,
                queue_report,
                crate::boundary::event_values::event_count(1),
            ))
        }
        NoSubscriberQueueDecision::DeadLetter(queue_report, reason, error) => {
            bus.record_stored_snapshot(&stored).await;
            let dead_letter = DeadLetter::for_queue(&stored, reason, error);
            bus.queue
                // CLONE-JUSTIFICATION: completion bookkeeping owns the key beyond this event flow.
                .mark_completed(&stored.event_id, stored.idempotency_key.clone());
            bus.record_dead_letter(dead_letter).await;
            Ok(empty_publish_report(
                &stored,
                dispatch_mode,
                queue_report,
                crate::boundary::event_values::event_count(1),
            ))
        }
    }
}

pub(super) async fn dead_letter_expired_deadline(
    bus: &EventBus,
    stored: StoredEventEnvelope,
    dispatch_mode: DispatchMode,
) -> Result<PublishReport, EventingError> {
    bus.record_stored_snapshot(&stored).await;
    let dead_letter = DeadLetter::for_queue(
        &stored,
        DeadLetterReason::DeadlineExpired,
        EventingError::EventDeadlineExpired {
            // CLONE-JUSTIFICATION: dead-letter error owns event type after stored envelope is consumed.
            event_type: stored.contract.event_type.clone(),
        },
    );
    bus.queue
        // CLONE-JUSTIFICATION: completion bookkeeping owns the key beyond this event flow.
        .mark_completed(&stored.event_id, stored.idempotency_key.clone());
    bus.record_dead_letter(dead_letter).await;
    Ok(empty_publish_report(
        &stored,
        dispatch_mode,
        bus.queue
            .report(QueueDisposition::DeadLetteredDeadlineExpired),
        crate::boundary::event_values::event_count(1),
    ))
}

pub(super) async fn dispatch(
    bus: &EventBus,
    stored: StoredEventEnvelope,
    subscribers: Vec<SubscriberRecord>,
    dispatch_mode: DispatchMode,
) -> Result<Vec<crate::bus::reports::handler::HandlerReport>, EventingError> {
    match dispatch_mode {
        DispatchMode::Sequential => {
            Ok(dispatch_sequential(
                stored,
                subscribers,
                // CLONE-JUSTIFICATION: dispatch task owns publisher context beyond this borrowed bus.
                EventPublisher::new(bus.clone()),
                // CLONE-JUSTIFICATION: dispatch task owns handler policy beyond this borrowed bus.
                bus.handler_policy.clone(),
                Arc::clone(&bus.clock),
            )
            .await)
        }
        DispatchMode::Concurrent => {
            Ok(dispatch_concurrent(
                stored,
                subscribers,
                // CLONE-JUSTIFICATION: dispatch task owns publisher context beyond this borrowed bus.
                EventPublisher::new(bus.clone()),
                // CLONE-JUSTIFICATION: dispatch task owns handler policy beyond this borrowed bus.
                bus.handler_policy.clone(),
                Arc::clone(&bus.clock),
            )
            .await)
        }
        DispatchMode::OrderedByAggregateKey => {
            // CLONE-JUSTIFICATION: aggregate gate lookup retains key after stored event moves into dispatch.
            let aggregate_key = stored.aggregate_key.clone();
            let aggregate_gate = bus.aggregate_gate(&aggregate_key);
            let aggregate_permit = Arc::clone(&aggregate_gate)
                .acquire_owned()
                .await
                .map_err(|_closed| EventingError::BusShutdown)?;
            let reports = dispatch_sequential(
                stored,
                subscribers,
                // CLONE-JUSTIFICATION: dispatch task owns publisher context beyond this borrowed bus.
                EventPublisher::new(bus.clone()),
                // CLONE-JUSTIFICATION: dispatch task owns handler policy beyond this borrowed bus.
                bus.handler_policy.clone(),
                Arc::clone(&bus.clock),
            )
            .await;
            drop(aggregate_permit);
            bus.release_idle_aggregate_gate(&aggregate_key, &aggregate_gate);
            Ok(reports)
        }
    }
}
