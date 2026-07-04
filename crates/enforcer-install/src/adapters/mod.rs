//! Per-harness [`crate::core::HarnessAdapter`] implementations. Each
//! module here is owned by its own Track C workpack (c03 `claude`, c06
//! `codex`, c07 `generic`, c08/c09 the remaining harnesses) — this
//! `mod.rs` only wires the module tree; it does not itself implement an
//! adapter.

pub mod aider;
pub mod antigravity;
pub mod claude;
pub mod codex;
pub mod cursor;
pub mod gemini;
pub mod generic;
pub mod kilocode;
pub mod kiro;
pub mod opencode;
pub mod windsurf;
pub mod zed;
