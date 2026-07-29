use crate::boundary::stored_event_persistence::StoredEventEnvelope;
use enforcer_domain::events_types::{
    JournalAppendDecision, JournalDispatchPhase, JournalMode, JournalSelector,
};

/// Event-runtime data for journal policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalPolicy {
    pub mode: JournalMode,
    pub selector: JournalSelector,
}

impl JournalPolicy {
    /// Executes the disabled event-runtime operation.
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Executes the before dispatch event-runtime operation.
    pub fn before_dispatch(selector: JournalSelector) -> Self {
        Self {
            mode: JournalMode::BeforeDispatch,
            selector,
        }
    }

    /// Executes the after dispatch event-runtime operation.
    pub fn after_dispatch(selector: JournalSelector) -> Self {
        Self {
            mode: JournalMode::AfterDispatch,
            selector,
        }
    }

    /// Executes the before and after dispatch event-runtime operation.
    pub fn before_and_after_dispatch(selector: JournalSelector) -> Self {
        Self {
            mode: JournalMode::BeforeAndAfterDispatch,
            selector,
        }
    }

    pub(crate) fn should_append(
        &self,
        envelope: &StoredEventEnvelope,
        phase: JournalDispatchPhase,
    ) -> JournalAppendDecision {
        let mode_includes = matches!(
            (self.mode, phase),
            (
                JournalMode::BeforeDispatch,
                JournalDispatchPhase::BeforeDispatch
            ) | (
                JournalMode::AfterDispatch,
                JournalDispatchPhase::AfterDispatch
            ) | (JournalMode::BeforeAndAfterDispatch, _)
        );
        let event_type = &envelope.contract.event_type;
        let selector_matches = match &self.selector {
            JournalSelector::All => true,
            JournalSelector::EventTypes(event_types)
            | JournalSelector::ContractAllowlist(event_types) => event_types.contains(event_type),
            JournalSelector::Namespaces(namespaces) => {
                enforcer_domain::events_types::EventNamespace::from_event_type(event_type)
                    .is_ok_and(|namespace| namespaces.contains(&namespace))
            }
        };
        if mode_includes && selector_matches {
            JournalAppendDecision::Append
        } else {
            JournalAppendDecision::Skip
        }
    }
}

impl Default for JournalPolicy {
    fn default() -> Self {
        Self {
            mode: JournalMode::Disabled,
            selector: JournalSelector::All,
        }
    }
}
