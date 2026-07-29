//! `enforcer_events::event::DomainEvent` Ã¢â‚¬â€ a narrow, enforcer-facing
//! compatibility shim over the vendored bus's own [`crate::envelope::DomainEvent`].
//!
//! # Why this module exists
//!
//! The vendored `ocentra-eventing` crate (see the `lib.rs` attribution note)
//! ships its OWN `DomainEvent` trait in [`crate::envelope`], with a much
//! richer contract: `contract()`, `aggregate_key()`, and `idempotency_key()`,
//! designed for typed payloads that travel inside an
//! [`crate::envelope::EventFrame`] and get dispatched through
//! [`crate::bus::EventBus`].
//!
//! `enforcer-coordination`'s `fix_loop.rs` (the crate's one existing,
//! out-of-scope consumer) predates that richer contract and only needs a
//! stable, human-readable event-kind tag for observability Ã¢â‚¬â€ it never
//! constructs an [`crate::envelope::EventFrame`] or touches the bus. It
//! implements a much smaller marker trait:
//!
//! ```rust
//! use enforcer_events::event::DomainEvent;
//!
//! #[derive(serde::Serialize, serde::Deserialize)]
//! struct FixLoopDecisionEvent;
//!
//! impl DomainEvent for FixLoopDecisionEvent {
//!     fn event_kind(&self) -> Result<enforcer_domain::events_types::EventType, enforcer_domain::boundary::decode_error::DecodeError> {
//!         enforcer_domain::events_types::EventType::parse("coordination.fix_loop.decision")
//!     }
//! }
//! ```
//!
//! This module keeps that call site compiling with ZERO changes by exposing
//! a distinct, deliberately minimal `DomainEvent` trait under
//! `enforcer_events::event`. It is NOT the same trait as
//! [`crate::envelope::DomainEvent`] and there is no blanket impl bridging
//! them Ã¢â‚¬â€ a payload that wants BOTH the enforcer's `event_kind` tag and the
//! vendored bus's full envelope/dispatch machinery implements both traits
//! explicitly.
/// Contract implemented by domain event.
pub trait DomainEvent: serde::Serialize + for<'de> serde::Deserialize<'de> {
    /// Stable, human-readable event kind stamped for observability/logging
    /// (e.g. `"scan.completed"`, `"coordination.fix_loop.decision"`).
    fn event_kind(
        &self,
    ) -> Result<
        enforcer_domain::events_types::EventType,
        enforcer_domain::boundary::decode_error::DecodeError,
    >;
}
