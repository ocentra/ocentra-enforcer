//! Dogfood onboarding fixture — a minimal Rust workspace shaped like this
//! repo (workspace `Cargo.toml` + one member crate), used to prove the
//! `enforcer-onboarding` skill's procedure generalizes to a Rust project.

/// A clean, unremarkable function. The onboarding test's "seeded violation"
/// case is layered on top of this file's content by the test itself (it
/// writes a known-bad line, runs the gate, then removes it and re-runs) —
/// this fixture file's checked-in state is always the CLEAN baseline.
#[must_use]
pub fn greet(name: &str) -> String {
    format!("hello, {name}")
}
