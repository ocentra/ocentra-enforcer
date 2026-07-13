use std::sync::{Arc, PoisonError};

use tokio::sync::Semaphore;

use crate::AggregateKey;

use super::EventBus;

impl EventBus {
    pub(super) fn aggregate_gate(&self, aggregate_key: &AggregateKey) -> Arc<Semaphore> {
        let mut gates = self
            .aggregate_gates
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        Arc::clone(
            gates
                // CLONE-JUSTIFICATION: the map owns its aggregate key while the caller retains its typed lookup key.
                .entry(aggregate_key.clone())
                .or_insert_with(|| Arc::new(Semaphore::new(1))),
        )
    }

    pub(super) fn release_idle_aggregate_gate(
        &self,
        aggregate_key: &AggregateKey,
        aggregate_gate: &Arc<Semaphore>,
    ) {
        if aggregate_gate.available_permits() == 0 || Arc::strong_count(aggregate_gate) > 2 {
            return;
        }
        let mut gates = self
            .aggregate_gates
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if gates
            .get(aggregate_key)
            .is_some_and(|current| Arc::ptr_eq(current, aggregate_gate))
            && aggregate_gate.available_permits() == 1
            && Arc::strong_count(aggregate_gate) <= 2
        {
            gates.remove(aggregate_key);
        }
    }
}
