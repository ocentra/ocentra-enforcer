use std::sync::PoisonError;

use crate::{DispatchMode, EventingError};

use super::{
    reports::dead_letter::{DeadLetter, DeadLetterReason},
    EventBus, EventBusClearReport, EventBusShutdownReport, ShutdownMode,
};

impl EventBus {
    pub async fn shutdown(
        &self,
        mode: ShutdownMode,
    ) -> Result<EventBusShutdownReport, EventingError> {
        if self.begin_shutdown() {
            return Ok(empty_shutdown_report(mode, true));
        }

        let mut report = match self.shutdown_queue(mode).await {
            Ok(report) => report,
            Err(error) => {
                self.rollback_shutdown();
                return Err(error);
            }
        };
        report.in_flight_dispatch_count = self.active_dispatches.active_count();
        self.active_dispatches.wait_for_idle().await;
        report.subscription_count = self.clear_subscriptions_for_shutdown();
        report.aggregate_gate_count = self.clear_aggregate_gates_for_shutdown();
        let request_report = self.requests.cancel_for_shutdown();
        self.queue.clear_for_test();
        self.mark_shutdown();

        report.pending_request_count = request_report.pending_request_count;
        report.completed_request_count = request_report.completed_request_count;
        report.timed_out_request_count = request_report.timed_out_request_count;
        Ok(report)
    }

    async fn shutdown_queue(
        &self,
        mode: ShutdownMode,
    ) -> Result<EventBusShutdownReport, EventingError> {
        let drain = if mode == ShutdownMode::Drain {
            Some(
                self.drain_queued_unchecked(DispatchMode::Sequential)
                    .await?,
            )
        } else {
            None
        };
        let remaining_queued = self.queue.take_all_queued();
        let mut report = empty_shutdown_report(mode, false);
        report.queued_dispatched_count = drain.as_ref().map_or(0, |drain| drain.dispatched_count);
        report.queued_expired_count = drain.as_ref().map_or(0, |drain| drain.expired_count);
        report.queued_event_count =
            report.queued_dispatched_count + report.queued_expired_count + remaining_queued.len();
        match mode {
            ShutdownMode::Drain | ShutdownMode::DeadLetterQueued => {
                report.queued_dead_lettered_count = remaining_queued.len();
                self.dead_letter_shutdown_queue(remaining_queued).await;
            }
            ShutdownMode::DropQueuedForTestOnly => {
                report.queued_dropped_count = remaining_queued.len();
            }
        }
        Ok(report)
    }

    async fn dead_letter_shutdown_queue(&self, queued: Vec<crate::QueuedEnvelope>) {
        let dead_letters = queued
            .into_iter()
            .map(|queued| {
                DeadLetter::for_queue(
                    &queued.stored,
                    DeadLetterReason::Shutdown,
                    EventingError::BusShutdown,
                )
            })
            .collect::<Vec<_>>();
        self.record_dead_letters(dead_letters).await;
    }

    fn clear_subscriptions_for_shutdown(&self) -> usize {
        let mut registry = self
            .registry
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let subscription_count = registry.values().map(Vec::len).sum();
        registry.clear();
        subscription_count
    }

    fn clear_aggregate_gates_for_shutdown(&self) -> usize {
        let mut aggregate_gates = self
            .aggregate_gates
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let aggregate_gate_count = aggregate_gates.len();
        aggregate_gates.clear();
        aggregate_gate_count
    }

    pub async fn clear_for_test(&self) -> EventBusClearReport {
        let subscription_count = {
            let mut registry = self
                .registry
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let subscription_count = registry.values().map(Vec::len).sum();
            registry.clear();
            subscription_count
        };
        let stored_journal_count = {
            let mut stored_journal = self.stored_journal.write().await;
            let stored_journal_count = stored_journal.len();
            stored_journal.clear();
            stored_journal_count
        };
        let dead_letter_count = {
            let mut dead_letters = self.dead_letters.write().await;
            let dead_letter_count = dead_letters.len();
            dead_letters.clear();
            dead_letter_count
        };
        let aggregate_gate_count = {
            let mut aggregate_gates = self
                .aggregate_gates
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let aggregate_gate_count = aggregate_gates.len();
            aggregate_gates.clear();
            aggregate_gate_count
        };
        let queue_report = self.queue.clear_for_test();
        let request_report = self.requests.clear_for_test();
        EventBusClearReport {
            subscription_count,
            stored_journal_count,
            dead_letter_count,
            aggregate_gate_count,
            queued_event_count: queue_report.queued_event_count,
            queued_idempotency_key_count: queue_report.queued_idempotency_key_count,
            in_flight_idempotency_key_count: queue_report.in_flight_idempotency_key_count,
            completed_idempotency_key_count: queue_report.completed_idempotency_key_count,
            pending_request_count: request_report.pending_request_count,
            completed_request_count: request_report.completed_request_count,
            timed_out_request_count: request_report.timed_out_request_count,
        }
    }
}
fn empty_shutdown_report(mode: ShutdownMode, already_shutdown: bool) -> EventBusShutdownReport {
    EventBusShutdownReport {
        mode,
        already_shutdown,
        subscription_count: 0,
        aggregate_gate_count: 0,
        queued_event_count: 0,
        queued_dispatched_count: 0,
        queued_expired_count: 0,
        queued_dead_lettered_count: 0,
        queued_dropped_count: 0,
        in_flight_dispatch_count: 0,
        pending_request_count: 0,
        completed_request_count: 0,
        timed_out_request_count: 0,
    }
}
