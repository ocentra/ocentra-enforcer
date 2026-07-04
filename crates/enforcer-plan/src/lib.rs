//! `enforcer-plan` — Track B: the plan scaffolder and the PLAN-* structure
//! validators (arc-20 skeleton; feature modules land via b01-b06 and x05).
//!
//! # Charter
//!
//! Track B's plan/workpack document contract used to live as plan-doc prose
//! plus ad hoc `.mjs` shape checks. This crate is the Rust replacement: a
//! deterministic scaffolder that emits well-formed workpack documents (the
//! `agent-capsule` block, `owns`/`deps`/`tier`, and the standard
//! Where-We-Are / Where-We-Want-To-Be / Requirement-Checklist /
//! Acceptance-And-Proof / Parallel-Ownership-Notes sections), and a family
//! of PLAN-* `enforcer_validator::validator::Validator` implementations that
//! enforce that same contract on existing plan docs (missing capsule,
//! missing sections, non-disjoint `owns` between no-dep-edge workpacks
//! (`PLAN-PARALLEL-SAFETY`), stale proof rows, missing resume-state
//! sections, ...).
//!
//! This crate owns only the SKELETON (arc-20): this module doc, the
//! crate-local [`error::PlanError`], and the `Cargo.toml` dependency edges
//! onto `enforcer-rules` (rule-record linkage) and `enforcer-validator` (the
//! `Validator` trait + fixture/parity harness). It does not itself implement
//! the scaffolder or any `Validator`. Feature packs mount their own module
//! and add exactly one `pub mod` line here when they land:
//!
//! - `scaffolder` (b01) — the deterministic workpack emitter
//!   (`scaffolder-emit` / `scaffolder-determinism`); refuses to overwrite an
//!   existing workpack without an explicit `--force`.
//! - `validator` (b02) — the PLAN-* `Validator` impls (capsule / sections /
//!   `PLAN-PARALLEL-SAFETY` owns-overlap / `PLAN-RESUME-STATE`), keyed to
//!   their `RuleId`s in `enforcer-rules`, self-validated against THIS plan
//!   dir (`docs/plans/enforcer-selfhost-plan/`).
//! - `templates` (b03) — the frozen capsule/index template fixtures the
//!   scaffolder renders from; no inline capsule literals live outside
//!   `crates/enforcer-plan/templates/`.
//! - `orchestrator` (b04) — the dependency-free-frontier / disjoint-lane
//!   binding (`orchestrator-frontier` / `orchestrator-lanes` /
//!   `orchestrator-claim-guard`) that maps validated plan structure onto
//!   coordination claims.
//! - `skill` (b05) — the `/plan` skill dispatch that self-validates against
//!   the real `validator` module's live findings, never a dispatch that
//!   short-circuits to an empty/fixed result.
//! - `agents_forest` (b06) — the `AGENTS.md` decision-forest scaffold and
//!   chain-resolve/resume-sim checks.
//! - `lessons` (x05) — the lesson-capture loop (doctrine-block + skill
//!   routing, golden artifact emission, seed-corpus import), owned by x05,
//!   not by the B-track packs above.
//!
//! This crate does NOT own: the rule registry (`enforcer-rules`), the
//! `Validator` trait or fixture/parity harness (`enforcer-validator`), or
//! any non-plan-doc validators (the `enforcer-lang-*` / `enforcer-security`
//! families).
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_plan::validator::PlanParallelSafety`.

pub mod error;
pub mod validator;
