//! Per-harness [`crate::core::HarnessAdapter`] implementations. Each
//! module here is owned by its own Track C workpack (c03 `claude`, c06
//! `codex`, c07 `generic`, c08/c09 the remaining harnesses) — this
//! `mod.rs` only wires the module tree; it does not itself implement an
//! adapter.

pub mod claude;
