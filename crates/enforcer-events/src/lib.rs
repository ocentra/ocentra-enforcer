//! `enforcer-events` — VENDORED from OcentraParent's `ocentra-eventing`
//! crate (arc-25).
//!
//! # VENDORING ATTRIBUTION (arc-25 / EXECUTION_MODEL §2, lesson L12)
//!
//! This crate's workpack (`docs/plans/enforcer-selfhost-plan/workpacks/
//! arc-25-enforcer-events.md`) specifies VENDORING `enforcer-events` AS-IS
//! from OcentraParent's `ocentra-eventing` crate. Lesson L12 recorded that
//! the canonical source was UNREACHABLE from the original build machine, so
//! an earlier pass shipped a lean, hand-written stand-in implementing only
//! the workpack's behavioral contract. That stand-in has been REPLACED by
//! this file: the source became reachable and was vendored wholesale on
//! 2026-07-05 from:
//!
//! - Repository: OcentraParent
//! - Branch: `codex/tracking-plan-full-continuation-a`
//! - Path: `crates/ocentra-eventing`
//!
//! The full upstream module tree, its own test suite (`contract`,
//! `integration`, `journal_replay`, `unit`, `version-skew`), fixtures, and
//! examples were copied verbatim; only the package name changed
//! (`ocentra-eventing` -> `enforcer-events`) and the clippy-wall remediation
//! described below was applied. Upstream machinery the enforcer does not yet
//! exercise (contract-registry, aggregate-ordering, TTL/overflow queue,
//! request/response, external transport/replay) is kept fully wired and
//! DORMANT, per the workpack's explicit instruction not to re-implement to
//! shrink it.
//!
//! # Portability (overriding design constraint)
//!
//! This crate intentionally has ZERO dependency on the rest of the Enforcer
//! workspace so it can be lifted into any other Rust project unchanged. Its
//! `Cargo.toml` depends only on generic, portable crates (`chrono`,
//! `futures`, `serde`, `serde_json`, `sha2`, `tokio`) — no `enforcer-domain`,
//! no `enforcer-core`. All identifiers (`CorrelationId`, `CausationId`,
//! `EventId`, etc. — see [`ids`]) are defined locally in this crate as plain
//! validated string/integer newtypes, exactly as upstream ships them. Any
//! Enforcer consumer that wants to bridge one of ITS OWN branded newtypes
//! (e.g. `enforcer_domain::ids::CorrelationId`) into this crate's wire shape
//! converts at its own call site (branded -> `&str`/`String` going in,
//! `&str`/`String` -> branded coming out) — that conversion does not belong
//! in this crate.
//!
//! # `enforcer_events::event::DomainEvent` compatibility shim
//!
//! See [`event`] for the narrow, enforcer-specific `DomainEvent` marker
//! trait (`event_kind(&self) -> &'static str`) kept for
//! `enforcer-coordination`'s `fix_loop.rs`, which predates and is distinct
//! from this crate's own richer [`envelope::DomainEvent`] contract
//! (`contract()` / `aggregate_key()` / `idempotency_key()`).
#![forbid(unsafe_code)]

pub mod bus;
pub mod clock;
pub mod compatibility;
pub mod compatibility_markdown;
pub mod contract_registry;
pub mod delivery;
pub mod envelope;
pub mod error;
pub mod event;
pub mod execution;
pub mod ids;
pub mod journal;
pub mod queue;
pub mod registrar;
pub mod replay;
pub mod request;
pub mod testkit;
pub mod topology;

use bus::publisher::{EventContext, EventPublisher};
use bus::reports::dead_letter::{
    dead_letter_recorded_event_type, DeadLetter, DeadLetterEvent, DeadLetterReason,
};
use bus::reports::{
    handler::{
        EventMetricsSnapshot, EventTraceFields, HandlerOutcome, HandlerReport, PublishReport,
        QueueDrainReport,
    },
    EventQueueMetrics, EventRequestMetrics,
};
use bus::subscriber::{EventSubscriber, SubscriptionHandle, SubscriptionReport, UnsubscribeReport};
use bus::{DispatchMode, EventBus, EventBusClearReport, EventBusShutdownReport, ShutdownMode};
use clock::{
    EventClock, EventClockInstant, EventClockSleep, ManualEventClock, SharedEventClock,
    SystemEventClock,
};
use compatibility::{EventCompatibilityEntry, EventCompatibilityMatrix, EventCompatibilityStatus};
use contract_registry::{
    EventContractDescriptor, EventContractRegistry, EventContractRegistryDocumentation,
};
use delivery::decide_event_delivery_route;
use delivery::validation::{
    EventDeliveryBackpressurePolicy, EventDeliveryDecisionError, EventDeliveryDecisionInput,
    EventDeliveryDecisionProof, EventDeliveryDecisionState, EventDeliveryRequiredArtifact,
    EventDeliveryRouteKind, EventDeliverySubscriberFilter,
};
use envelope::{
    DomainEvent, EventContract, EventEnvelope, EventMetadata, EventPriority, EventSource,
    StoredEventEnvelope, StoredEventPayload,
};
use error::EventingError;
use execution::HandlerExecutionPolicy;
use ids::{
    AggregateKey, CausationId, CorrelationId, EventCustody, EventId, EventNamespace, EventType,
    IdempotencyKey, JournalHash, RecordedAt, RequestId, RuntimeInstanceId, RuntimeRole,
    SchemaVersion, SourceComponent, SourceService, SubscriberId, TargetHandler,
};
use journal::ndjson::{
    JournalFlushPolicy, JournalHashChain, NdjsonEventJournal, NdjsonJournalEntry,
    NdjsonJournalOptions,
};
use journal::policy::{JournalDispatchPhase, JournalMode, JournalPolicy, JournalSelector};
use journal::{EventJournal, JournalAppend, SharedEventJournal};
use queue::policy::{
    EventQueuePolicy, NoSubscriberQueuePolicy, QueueDisposition, QueueOverflowPolicy, QueueReport,
};
use queue::state::{EventQueue, EventQueueClearReport, NoSubscriberQueueDecision, QueuedEnvelope};
use registrar::{EventRegistrar, RegistrarDisposeReport};
use replay::{ReplayCursor, ReplayFilter, ReplayMode, ReplayReadReport, ReplayRecord};
use request::RequestRegistry;
use request::{
    EventResponseContract, RequestCompletionOutcome, RequestCompletionReport, RequestEvent,
    RequestOptions, RequestReport,
};
use testkit::EventRecorder;
use topology::{
    EventTopologyEntry, EventTopologyFamilyVariant, EventTopologyManifest, EventTopologyPublisher,
    EventTopologyStatus, EventTopologySubscriber, EventTopologySubscriberTarget,
};

// Keep the root aliases live for internal modules and tests without turning
// this crate root into a re-export barrel.
const _: () = {
    let _ = core::mem::size_of::<EventBusClearReport>();
    let _ = core::mem::size_of::<EventBusShutdownReport>();
    let _ = core::mem::size_of::<ShutdownMode>();
    let _ = core::mem::size_of::<EventPublisher>();
    let _ = core::mem::size_of::<DeadLetter>();
    let _ = core::mem::size_of::<DeadLetterEvent>();
    let _ = core::mem::size_of::<EventMetricsSnapshot>();
    let _ = core::mem::size_of::<EventTraceFields>();
    let _ = core::mem::size_of::<HandlerOutcome>();
    let _ = core::mem::size_of::<HandlerReport>();
    let _ = core::mem::size_of::<QueueDrainReport>();
    let _ = dead_letter_recorded_event_type;
    let _ = core::mem::size_of::<Option<&dyn EventClock>>();
    let _ = core::mem::size_of::<EventClockSleep>();
    let _ = core::mem::size_of::<ManualEventClock>();
    let _ = core::mem::size_of::<EventCompatibilityEntry>();
    let _ = core::mem::size_of::<EventCompatibilityMatrix>();
    let _ = core::mem::size_of::<EventCompatibilityStatus>();
    let _ = core::mem::size_of::<EventContractDescriptor>();
    let _ = core::mem::size_of::<EventContractRegistryDocumentation>();
    let _ = core::mem::size_of::<EventDeliveryBackpressurePolicy>();
    let _ = core::mem::size_of::<EventDeliveryDecisionError>();
    let _ = core::mem::size_of::<EventDeliveryDecisionInput>();
    let _ = core::mem::size_of::<EventDeliveryDecisionProof>();
    let _ = core::mem::size_of::<EventDeliveryDecisionState>();
    let _ = core::mem::size_of::<EventDeliveryRequiredArtifact>();
    let _ = core::mem::size_of::<EventDeliveryRouteKind>();
    let _ = core::mem::size_of::<EventDeliverySubscriberFilter>();
    let _ = decide_event_delivery_route;
    let _ = core::mem::size_of::<EventPriority>();
    let _ = core::mem::size_of::<EventSource>();
    let _ = core::mem::size_of::<StoredEventPayload>();
    let _ = core::mem::size_of::<Option<&dyn EventJournal>>();
    let _ = core::mem::size_of::<JournalAppend>();
    let _ = core::mem::size_of::<JournalFlushPolicy>();
    let _ = core::mem::size_of::<JournalHashChain>();
    let _ = core::mem::size_of::<NdjsonEventJournal>();
    let _ = core::mem::size_of::<NdjsonJournalEntry>();
    let _ = core::mem::size_of::<NdjsonJournalOptions>();
    let _ = core::mem::size_of::<JournalMode>();
    let _ = core::mem::size_of::<JournalSelector>();
    let _ = core::mem::size_of::<NoSubscriberQueuePolicy>();
    let _ = core::mem::size_of::<QueueOverflowPolicy>();
    let _ = core::mem::size_of::<EventQueueClearReport>();
    let _ = core::mem::size_of::<EventRegistrar>();
    let _ = core::mem::size_of::<RegistrarDisposeReport>();
    let _ = core::mem::size_of::<ReplayCursor>();
    let _ = core::mem::size_of::<ReplayFilter>();
    let _ = core::mem::size_of::<ReplayReadReport>();
    let _ = core::mem::size_of::<RequestCompletionOutcome>();
    let _ = core::mem::size_of::<EventTopologyEntry>();
    let _ = core::mem::size_of::<EventTopologyFamilyVariant>();
    let _ = core::mem::size_of::<EventTopologyManifest>();
    let _ = core::mem::size_of::<EventTopologyPublisher>();
    let _ = core::mem::size_of::<EventTopologyStatus>();
    let _ = core::mem::size_of::<EventTopologySubscriber>();
    let _ = core::mem::size_of::<EventTopologySubscriberTarget>();
    fn _touch_event_response_contract<T: EventResponseContract>() {}
    let _ = core::mem::size_of::<EventRecorder<DeadLetterEvent>>();
};
