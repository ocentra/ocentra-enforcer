use enforcer_domain::events_types::{EventCount, RequestCompletionOutcome, RequestId};

pub(super) fn complete_pending_request(
    state: &mut super::RequestRegistryState,
    request_id: RequestId,
    payload: crate::boundary::request_persistence::RequestPayload,
) -> Result<super::RequestCompletionReport, crate::error::EventingError> {
    let Some(entry) = state.entries.get_mut(&request_id) else {
        return Ok(super::completion_report(
            request_id,
            RequestCompletionOutcome::Late,
        ));
    };
    entry.state = super::RequestState::Completed;
    let outcome = match entry.sender.take() {
        Some(sender) => {
            if sender.send(payload).is_ok() {
                RequestCompletionOutcome::Completed
            } else {
                RequestCompletionOutcome::Late
            }
        }
        None => RequestCompletionOutcome::Late,
    };
    mark_terminal(state, &request_id);
    trim_terminal_requests(state);
    Ok(super::completion_report(request_id, outcome))
}

pub(super) fn request_registry_report(
    state: &super::RequestRegistryState,
) -> super::RequestRegistryClearReport {
    super::RequestRegistryClearReport {
        pending_request_count: count_request_state(state, super::RequestState::Pending),
        completed_request_count: count_request_state(state, super::RequestState::Completed),
        timed_out_request_count: count_request_state(state, super::RequestState::TimedOut),
    }
}

fn count_request_state(
    state: &super::RequestRegistryState,
    requested: super::RequestState,
) -> EventCount {
    crate::boundary::event_values::event_count(
        state
            .entries
            .values()
            .filter(|entry| entry.state == requested)
            .count(),
    )
}

pub(super) fn mark_terminal(state: &mut super::RequestRegistryState, request_id: &RequestId) {
    if !state
        .terminal_order
        .iter()
        .any(|terminal_id| terminal_id == request_id)
    {
        // CLONE-JUSTIFICATION: terminal ordering owns the request identity while the result map retains the same key.
        state.terminal_order.push_back(request_id.clone());
    }
}

pub(super) fn trim_terminal_requests(state: &mut super::RequestRegistryState) {
    while state.terminal_order.len() > super::TERMINAL_REQUEST_RETENTION_LIMIT {
        if let Some(request_id) = state.terminal_order.pop_front() {
            if state
                .entries
                .get(&request_id)
                .is_some_and(|entry| entry.state != super::RequestState::Pending)
            {
                state.entries.remove(&request_id);
            }
        }
    }
}
