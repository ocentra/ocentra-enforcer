use std::sync::Arc;

use crate::bus::dispatch::{dispatch_concurrent, dispatch_sequential};
use crate::bus::publisher::EventPublisher;
use crate::bus::reports::dead_letter::{DeadLetter, DeadLetterReason};
use crate::bus::reports::{dead_letters_for, empty_publish_report};
use crate::bus::{DispatchMode, EventBus, SubscriberRecord};
use crate::queue::state::NoSubscriberQueueDecision;
use crate::{EventingError, JournalDispatchPhase, PublishReport, QueueDisposition, StoredEventEnvelope};

pub(super) async fn publish_without_subscribers(
    bus: &EventBus,
    stored: StoredEventEnvelope,
    dispatch_mode: DispatchMode,
) -> Result<PublishReport, EventingError> {
    match bus
        .queue
        .enqueue_no_subscriber(stored.clone(), bus.clock.now())?
    {
        NoSubscriberQueueDecision::Dispatch(queue_report)
        | NoSubscriberQueueDecision::Queued(queue_report) => {
            bus.record_stored_snapshot(&stored).await;
            Ok(empty_publish_report(
                &stored,
                dispatch_mode,
                queue_report,
                0,
            ))
        }
        NoSubscriberQueueDecision::QueuedWithDeadLetter(queue_report, dropped, reason, error) => {
            let dropped = *dropped;
            bus.record_stored_snapshot(&stored).await;
            let dead_letter = DeadLetter::for_queue(&dropped, reason, error);
            bus.queue
                .mark_completed(&dropped.event_id, dropped.idempotency_key.clone());
            bus.record_dead_letter(dead_letter).await;
            Ok(empty_publish_report(
                &stored,
                dispatch_mode,
                queue_report,
                1,
            ))
        }
        NoSubscriberQueueDecision::DeadLetter(queue_report, reason, error) => {
            bus.record_stored_snapshot(&stored).await;
            let dead_letter = DeadLetter::for_queue(&stored, reason, error);
            bus.queue
                .mark_completed(&stored.event_id, stored.idempotency_key.clone());
            bus.record_dead_letter(dead_letter).await;
            Ok(empty_publish_report(
                &stored,
                dispatch_mode,
                queue_report,
                1,
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
            event_type: stored.contract.event_type.clone(),
        },
    );
    bus.queue
        .mark_completed(&stored.event_id, stored.idempotency_key.clone());
    bus.record_dead_letter(dead_letter).await;
    Ok(empty_publish_report(
        &stored,
        dispatch_mode,
        bus.queue
            .report(QueueDisposition::DeadLetteredDeadlineExpired),
        1,
    ))
}

pub(super) async fn dispatch(
    bus: &EventBus,
    stored: StoredEventEnvelope,
    subscribers: Vec<SubscriberRecord>,
    dispatch_mode: DispatchMode,
) -> Vec<crate::bus::reports::handler::HandlerReport> {
    match dispatch_mode {
        DispatchMode::Sequential => {
            dispatch_sequential(
                stored,
                subscribers,
                EventPublisher::new(bus.clone()),
                bus.handler_policy.clone(),
                Arc::clone(&bus.clock),
            )
            .await
        }
        DispatchMode::Concurrent => {
            dispatch_concurrent(
                stored,
                subscribers,
                EventPublisher::new(bus.clone()),
                bus.handler_policy.clone(),
                Arc::clone(&bus.clock),
            )
            .await
        }
        DispatchMode::OrderedByAggregateKey => {
            let aggregate_key = stored.aggregate_key.clone();
            let aggregate_gate = bus.aggregate_gate(&aggregate_key);
            // The aggregate gate semaphore is never closed anywhere in this
            // crate, so `acquire_owned` returning `Err` is not reachable in
            // practice. Degrade gracefully (dispatch without the ordering
            // permit) rather than panicking if that invariant is ever
            // violated by a future change.
            let aggregate_permit = Arc::clone(&aggregate_gate).acquire_owned().await.ok();
            let reports = dispatch_sequential(
                stored,
                subscribers,
                EventPublisher::new(bus.clone()),
                bus.handler_policy.clone(),
                Arc::clone(&bus.clock),
            )
            .await;
            drop(aggregate_permit);
            bus.release_idle_aggregate_gate(&aggregate_key, &aggregate_gate);
            reports
        }
    }
}
