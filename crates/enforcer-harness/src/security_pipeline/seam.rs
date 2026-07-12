//! Shared branded vocabulary for recorded security-tooling output — the
//! typed values every h07 stage carries after its recorded engine report
//! has cleared the `super::adapters` parsing boundary.
//!
//! No functions live here on purpose: minting happens only in
//! `super::adapters` (which validates raw text and rejects malformed
//! shapes with a typed decode failure), and consuming happens in the
//! stage gates. This module is the shared nouns, nothing else.

/// An engine's own identifier for what fired: a static-analysis rule
/// name, a failed property's name, a load-test check name, ...
// BRAND-INVARIANT: non-empty, carried verbatim from the recorded engine
// report; minted only by `super::adapters`, which rejects an empty label
// as malformed before this brand can exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineRuleLabel(pub(crate) String);

/// Human-readable detail text an engine reported (a message, a
/// counterexample rendering, ...).
// BRAND-INVARIANT: carried verbatim from the recorded engine report;
// display-only — never re-parsed, never used as an identifier. Minted
// only by `super::adapters`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineDetailText(pub(crate) String);

/// The persisted seed that reproduces a property/fuzz failure.
// BRAND-INVARIANT: non-empty; a blank seed is rejected as malformed at
// the `super::adapters` boundary because it cannot reproduce anything,
// so `Some(SeedText)` always means "genuinely reproducible".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedText(pub(crate) String);

/// A 1-based line number as the engine reported it.
// BRAND-INVARIANT: carried verbatim from the recorded engine report;
// only ever forwarded into `enforcer_domain::findings::Finding::line`.
// Minted only by `super::adapters`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineLine(pub(crate) u32);

/// The label naming one observed event on a money path (a span name, a
/// transaction id, ...).
// BRAND-INVARIANT: non-empty; `super::adapters::observability_report` rejects
// an empty event label as malformed, so every flagged event is nameable
// in its finding detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLabel(pub(crate) String);
