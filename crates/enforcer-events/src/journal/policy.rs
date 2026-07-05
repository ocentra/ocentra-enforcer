use crate::{EventNamespace, EventType, StoredEventEnvelope};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalMode {
    Disabled,
    BeforeDispatch,
    AfterDispatch,
    BeforeAndAfterDispatch,
}

impl JournalMode {
    pub(crate) fn includes(self, phase: JournalDispatchPhase) -> bool {
        matches!(
            (self, phase),
            (Self::BeforeDispatch, JournalDispatchPhase::BeforeDispatch)
                | (Self::AfterDispatch, JournalDispatchPhase::AfterDispatch)
                | (Self::BeforeAndAfterDispatch, _)
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JournalDispatchPhase {
    BeforeDispatch,
    AfterDispatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JournalSelector {
    All,
    EventTypes(Vec<EventType>),
    Namespaces(Vec<EventNamespace>),
    ContractAllowlist(Vec<EventType>),
}

impl JournalSelector {
    pub fn matches(&self, envelope: &StoredEventEnvelope) -> bool {
        match self {
            Self::All => true,
            Self::EventTypes(event_types) | Self::ContractAllowlist(event_types) => event_types
                .iter()
                .any(|event_type| event_type == &envelope.contract.event_type),
            Self::Namespaces(namespaces) => namespaces
                .iter()
                .any(|namespace| namespace.matches_event_type(&envelope.contract.event_type)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalPolicy {
    pub mode: JournalMode,
    pub selector: JournalSelector,
}

impl JournalPolicy {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn before_dispatch(selector: JournalSelector) -> Self {
        Self {
            mode: JournalMode::BeforeDispatch,
            selector,
        }
    }

    pub fn after_dispatch(selector: JournalSelector) -> Self {
        Self {
            mode: JournalMode::AfterDispatch,
            selector,
        }
    }

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
    ) -> bool {
        self.mode.includes(phase) && self.selector.matches(envelope)
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
