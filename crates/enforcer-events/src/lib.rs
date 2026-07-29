//! `enforcer-events` â€” VENDORED from OcentraParent's `ocentra-eventing`
//! crate (arc-25).
//!
//! # VENDORING ATTRIBUTION (arc-25 / EXECUTION_MODEL Â§2, lesson L12)
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
//! # Canonical domain values
//!
//! Event identifiers, counts, durations, policies, and report outcomes come
//! directly from `enforcer_domain::events_types`. This crate owns event
//! runtime behavior and converts raw persistence or presentation values only
//! in its `boundary` modules. It does not define or re-export duplicate domain
//! brands.
//!
//! # `enforcer_events::event::DomainEvent` compatibility shim
//!
//! See [`event`] for the narrow, enforcer-specific `DomainEvent` marker
//! trait (`event_kind(&self) -> &'static str`) kept for
//! `enforcer-coordination`'s `fix_loop.rs`, which predates and is distinct
//! from this crate's own richer [`envelope::DomainEvent`] contract
//! (`contract()` / `aggregate_key()` / `idempotency_key()`).
#![forbid(unsafe_code)]

pub mod boundary;
pub mod bus;
pub mod clock;
pub mod compatibility;
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
