use enforcer_domain::events_types::JournalSelector;
use enforcer_domain::events_types::QueueDisposition;
use enforcer_domain::events_types::{
    AggregateKey, CorrelationId, EventCustody, EventId, EventType, IdempotencyKey, JournalHash,
    RecordedAt, RequestId, RuntimeInstanceId, RuntimeRole, SchemaVersion, SourceComponent,
    SourceService, SubscriberId, TargetHandler,
};
use enforcer_domain::events_types::{DispatchMode, RegistrarStatus, ShutdownMode};
use enforcer_events::bus;
use enforcer_events::bus::publisher::EventPublisher;
use enforcer_events::bus::reports::dead_letter::dead_letter_recorded_event_type;
use enforcer_events::bus::subscriber::EventSubscriber;
use enforcer_events::bus::EventBus;
use enforcer_events::clock::{EventClock, ManualEventClock};
use enforcer_events::envelope::{DomainEvent, EventContract, EventMetadata, EventSource};
use enforcer_events::error;
use enforcer_events::error::EventingError;
use enforcer_events::execution::HandlerExecutionPolicy;
use enforcer_events::journal::policy::JournalPolicy;
use enforcer_events::journal::{EventJournal, JournalAppend};
use enforcer_events::queue;
use enforcer_events::queue::policy::EventQueuePolicy;
use enforcer_events::registrar::EventRegistrar;
use enforcer_events::request;
use enforcer_events::request::{EventResponseContract, RequestEvent, RequestOptions};
use enforcer_events::testkit::EventRecorder;

fn event_count(value: usize) -> enforcer_domain::events_types::EventCount {
    std::num::NonZeroUsize::new(value)
        .map(enforcer_domain::events_types::EventCount::try_new)
        .unwrap_or(enforcer_domain::events_types::EventCount::ZERO)
}

fn event_count_value(value: enforcer_domain::events_types::EventCount) -> usize {
    value.as_nonzero().map_or(0, std::num::NonZeroUsize::get)
}

#[path = "clock_manual.rs"]
mod clock_manual;
#[path = "envelope.rs"]
mod envelope;
#[path = "fixtures.rs"]
mod fixtures;
#[path = "handler_policy.rs"]
mod handler_policy;
#[path = "ids.rs"]
mod ids;
#[path = "lifecycle.rs"]
mod lifecycle;
#[path = "lifecycle_clear.rs"]
mod lifecycle_clear;
#[path = "metrics.rs"]
mod metrics;
#[path = "production_shutdown.rs"]
mod production_shutdown;
#[path = "queue.rs"]
mod queue_tests;
#[path = "request_response.rs"]
mod request_response;
#[path = "request_response_support.rs"]
mod request_response_support;
