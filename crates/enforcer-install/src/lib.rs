//! `enforcer-install` (arc-23) — the multi-harness installer SKELETON.
//!
//! # Charter
//!
//! Harness registration (writing MCP server config for claude/codex/
//! gemini/cursor/...) belonged to the retired Node engine. This crate is
//! the Rust replacement's Track C home: a harness-neutral
//! install/uninstall/update/doctor CORE, over which per-harness adapters
//! (Track C c01-c09 + the x03 legacy-name migration) register the
//! `enforcer` binary as each harness's MCP server.
//!
//! This crate lays the SKELETON only — the mount points c01-c09/x03 build
//! against:
//! - [`core`] — the harness-neutral `install`/`uninstall`/`update`/`doctor`
//!   verbs and the [`core::HarnessAdapter`] trait (`plan`/`apply`/`verify`).
//! - [`report`] — the report/result/check types every adapter produces.
//! - [`cli_contract`] — the `--scope user|project` (default `user`),
//!   `--dry-run`, non-TTY-JSON seam consumed by `enforcer-cli` (arc-22).
//! - [`managed_block`] — idempotent managed-block markers for text configs
//!   an adapter edits in place (e.g. a `CLAUDE.md` block) without clobbering
//!   surrounding user content.
//! - [`backup`] — pre-write backup-and-restore helpers so a failed/aborted
//!   install never leaves a harness config corrupted.
//! - [`distribution`] — platform binary resolution (win/mac/linux incl.
//!   musl + apple-silicon) and the download path for `enforcer install`.
//! - [`ci`] (c10) — this repo's OWN release pipeline, the reusable
//!   `enforcer-scan` composite GitHub Action, the portable
//!   `install.sh`/`install.ps1` scripts, and the optional npm wrapper
//!   package, so consumer CI never needs a Rust toolchain.
//! - [`commands`] (b05) — harness-neutral `/`-command emitters (the
//!   `/plan` command, dispatching to the real `enforcer plan new`/`plan
//!   check` binary invocation) — a distinct mount point from
//!   [`adapters`]'s per-harness config writers.
//!
//! # Global-install scope contract (binding — RUST_ARCHITECTURE.md)
//!
//! The canonical/default install is **USER/GLOBAL scope**: install-once,
//! zero-per-repo-config, so any repo the agent opens already has the
//! enforcer. Every adapter writes into its harness's **user-level**
//! registry, never a per-repo project file, and points at the **absolute**
//! path of the installed `enforcer` binary. `--scope project` is an
//! explicit, non-default opt-in (useful only for developing the enforcer
//! itself). See [`cli_contract::Scope`].
//!
//! # Update UX — binary swap, not a repo pull (binding — RUST_ARCHITECTURE.md)
//!
//! Staying current is a **binary swap**: `enforcer update` checks the
//! release channel and, if newer, removes the old binary and downloads the
//! new one — no source checkout, no toolchain. Because the MCP registration
//! points at the binary path (not a repo/folder/branch), the harness keeps
//! firing the identical command; only the bytes behind it change. See
//! [`core::update`] and [`distribution`].
//!
//! # Ownership (Parallel Ownership Notes, arc-23 workpack)
//!
//! This crate SKELETON owns `Cargo.toml`, `src/lib.rs`, `src/core.rs`,
//! `src/cli_contract.rs`, `src/report.rs`, `src/managed_block.rs`,
//! `src/backup.rs`, `src/distribution.rs`. The Track C packs each own
//! SPECIFIC files under this crate, disjoint by file, sequenced after this
//! skeleton: c02 `src/detect.rs`; c03 `src/adapters/claude.rs`; c04
//! `src/hooks/pretooluse.rs`; c05 `src/hooks/sessionstart.rs`; c06
//! `src/adapters/codex.rs`; c07 `src/adapters/generic.rs` +
//! `src/doctor.rs`; c08 `src/adapters/{gemini,cursor,zed}.rs`; c09
//! `src/adapters/{antigravity,windsurf,opencode,aider,kilocode,kiro}.rs`;
//! x03 `src/migrate_legacy_name.rs`. This crate does NOT create those
//! files or directories — it only documents the mount points here.

pub mod adapters;
pub mod backup;
pub mod ci;
pub mod cli_contract;
pub mod commands;
pub mod core;
pub mod detect;
pub mod distribution;
pub mod doctor;
pub mod emitters;
pub mod error;
pub mod hooks;
pub mod managed_block;
pub mod migrate_legacy_name;
pub mod report;
