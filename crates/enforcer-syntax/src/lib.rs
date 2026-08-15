//! Shared language classification, grammar providers, and safe syntax extraction.
//!
//! This crate owns the behavior-preserving parser/language substrate extracted
//! from `enforcer-memory`. It deliberately has no persistence, retrieval,
//! embedding, model-runtime, coordination, or enforcement-rule dependency.

pub mod boundary;

pub mod facts;
pub mod languages;
pub mod parsers;
pub mod registry;
